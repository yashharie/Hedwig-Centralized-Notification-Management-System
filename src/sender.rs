use crate::host_policy::check_host;
use crate::types::{FailureKind, WebhookOutcome, WebhookRequest};
use crate::validate::parse_webhook_url;
use reqwest::{Client, header::RETRY_AFTER};
use std::error::Error;
use std::time::Duration;
use url::Url;

// ------------------------------------------------------------------
// Status classification
// ------------------------------------------------------------------

/// Maps an HTTP status code (and optional `Retry-After` value) to a `WebhookOutcome`.
///
/// Retryable statuses: 429 Too Many Requests and all 5xx Server Errors.
/// All other non-2xx codes are treated as permanent failures.
pub fn classify_status(status: u16, retry_after_secs: Option<u64>) -> WebhookOutcome {
    if (200..300).contains(&status) {
        return WebhookOutcome::Delivered { status };
    }
    let retryable = status == 429 || (500..600).contains(&status);
    WebhookOutcome::Failed {
        kind: FailureKind::HttpStatus,
        status: Some(status),
        retryable,
        retry_after_secs,
    }
}

// ------------------------------------------------------------------
// Retry-After header parsing
// ------------------------------------------------------------------

/// Parse a `Retry-After` header value.
///
/// Supports both the seconds form (`"30"`) and the HTTP-date form
/// (`"Wed, 21 Oct 2015 07:28:00 GMT"`).  Returns the delay in whole seconds,
/// or `None` if the value cannot be interpreted.
pub fn parse_retry_after(value: &str) -> Option<u64> {
    let trimmed = value.trim();

    // Seconds form: a plain non-negative integer.
    if let Ok(secs) = trimmed.parse::<u64>() {
        return Some(secs);
    }

    // HTTP-date form: parse via httpdate and compute delta from now.
    parse_http_date_delta(trimmed)
}

/// Parse an HTTP-date string and return the number of seconds from now until
/// that instant (saturating at zero if the date is in the past).
fn parse_http_date_delta(date_str: &str) -> Option<u64> {
    // We parse RFC 7231 / RFC 850 / asctime formats manually because we have
    // no httpdate dependency.  We support the most common form produced by
    // servers: "Day, DD Mon YYYY HH:MM:SS GMT".
    //
    // Format: "Wed, 21 Oct 2015 07:28:00 GMT"
    //          ^3   ^2 ^3  ^4   ^8         ^3
    let s = date_str.trim();
    // Strip optional weekday prefix "Xxx, "
    let s = if let Some(idx) = s.find(", ") {
        &s[idx + 2..]
    } else {
        s
    };

    // Now expect: "DD Mon YYYY HH:MM:SS GMT"
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }

    let day: u64 = parts[0].parse().ok()?;
    let month: u64 = month_abbr(parts[1])?;
    let year: u64 = parts[2].parse().ok()?;
    let time_parts: Vec<&str> = parts[3].split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let hour: u64 = time_parts[0].parse().ok()?;
    let min: u64 = time_parts[1].parse().ok()?;
    let sec: u64 = time_parts[2].parse().ok()?;

    // Convert the parsed date/time to a Unix timestamp (seconds since epoch).
    let target_secs = date_to_unix(year, month, day, hour, min, sec)?;

    // Compute delta against the current system time.
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();

    Some(target_secs.saturating_sub(now_secs))
}

fn month_abbr(m: &str) -> Option<u64> {
    match m {
        "Jan" => Some(1),
        "Feb" => Some(2),
        "Mar" => Some(3),
        "Apr" => Some(4),
        "May" => Some(5),
        "Jun" => Some(6),
        "Jul" => Some(7),
        "Aug" => Some(8),
        "Sep" => Some(9),
        "Oct" => Some(10),
        "Nov" => Some(11),
        "Dec" => Some(12),
        _ => None,
    }
}

/// Convert a calendar date + time (UTC) to a Unix timestamp.
/// Uses the standard Gregorian proleptic calendar formula.
fn date_to_unix(year: u64, month: u64, day: u64, h: u64, m: u64, s: u64) -> Option<u64> {
    if month < 1 || month > 12 || day < 1 || day > 31 || h > 23 || m > 59 || s > 59 {
        return None;
    }
    // Days from epoch (1970-01-01) using Julian Day Number difference.
    // JDN formula: https://en.wikipedia.org/wiki/Julian_day#Converting_Gregorian_calendar_date_to_Julian_Day_Number
    let jdn = |y: u64, mo: u64, d: u64| -> u64 {
        let a = (14u64.saturating_sub(mo)) / 12;
        let yy = y + 4800 - a;
        let mm = mo + 12 * a - 3;
        d + (153 * mm + 2) / 5 + 365 * yy + yy / 4 - yy / 100 + yy / 400 - 32045
    };
    let epoch_jdn = jdn(1970, 1, 1);
    let target_jdn = jdn(year, month, day);
    if target_jdn < epoch_jdn {
        return None; // date before Unix epoch
    }
    let days = target_jdn - epoch_jdn;
    Some(days * 86400 + h * 3600 + m * 60 + s)
}

// ------------------------------------------------------------------
// Internal delivery (single attempt)
// ------------------------------------------------------------------

fn failed(kind: FailureKind, retryable: bool) -> WebhookOutcome {
    WebhookOutcome::Failed {
        kind,
        status: None,
        retryable,
        retry_after_secs: None,
    }
}

fn chain_mentions_blocked(err: &reqwest::Error) -> bool {
    let mut cur: Option<&(dyn Error + 'static)> = Some(err);
    while let Some(e) = cur {
        if e.to_string().contains("blocked destination") {
            return true;
        }
        cur = e.source();
    }
    false
}

/// Sends the request exactly once. Does NOT validate the URL. Internal use only.
pub(crate) async fn deliver(
    client: &Client,
    url: &Url,
    payload: &serde_json::Value,
) -> WebhookOutcome {
    match client.post(url.clone()).json(payload).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let retry_after = resp
                .headers()
                .get(RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(parse_retry_after);
            classify_status(status, retry_after)
        }
        Err(e) => {
            if chain_mentions_blocked(&e) {
                failed(FailureKind::BlockedDestination, false)
            } else if e.is_timeout() {
                failed(FailureKind::Timeout, true)
            } else {
                failed(FailureKind::Connection, true)
            }
        }
    }
}

// ------------------------------------------------------------------
// Public API — single attempt
// ------------------------------------------------------------------

/// Validates the URL and host policy, then sends a single HTTP POST.
///
/// Returns a `WebhookOutcome`. If the outcome is `Failed { retryable: true, .. }`
/// the caller should schedule a retry (see `send_webhook_with_retry` for a
/// built-in exponential-backoff loop).
pub async fn send_webhook(client: &Client, req: &WebhookRequest) -> WebhookOutcome {
    let url = match parse_webhook_url(&req.url) {
        Ok(u) => u,
        Err(_) => return failed(FailureKind::InvalidUrl, false),
    };
    if check_host(&url).is_err() {
        return failed(FailureKind::BlockedDestination, false);
    }
    deliver(client, &url, &req.payload).await
}

// ------------------------------------------------------------------
// Public API — retry loop with exponential back-off
// ------------------------------------------------------------------

/// Configuration for the retry loop used by `send_webhook_with_retry`.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of delivery attempts (must be ≥ 1).
    pub max_attempts: u32,
    /// Base back-off delay before the second attempt.
    pub base_delay: Duration,
    /// Maximum back-off cap so delays don't grow unbounded.
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(60),
        }
    }
}

/// Sends a webhook with automatic retries on transient failures.
///
/// The delay between attempts follows exponential back-off:
/// `min(base_delay * 2^(attempt), max_delay)`.
/// If the server returns a `Retry-After` header that is larger than the
/// computed back-off, that value is used instead.
///
/// Stops early if:
/// - A delivery succeeds.
/// - A failure with `retryable: false` is returned.
/// - All `max_attempts` have been exhausted.
pub async fn send_webhook_with_retry(
    client: &Client,
    req: &WebhookRequest,
    cfg: &RetryConfig,
) -> WebhookOutcome {
    assert!(cfg.max_attempts >= 1, "max_attempts must be at least 1");

    let mut last_outcome = WebhookOutcome::Failed {
        kind: FailureKind::Connection,
        status: None,
        retryable: true,
        retry_after_secs: None,
    };

    for attempt in 0..cfg.max_attempts {
        last_outcome = send_webhook(client, req).await;

        match &last_outcome {
            WebhookOutcome::Delivered { .. } => return last_outcome,
            WebhookOutcome::Failed { retryable, retry_after_secs, .. } => {
                if !retryable {
                    return last_outcome;
                }
                let is_last = attempt + 1 == cfg.max_attempts;
                if is_last {
                    break;
                }

                // Compute exponential back-off: base * 2^attempt, capped.
                // Use checked_shl to avoid overflow on large attempt counts.
                let multiplier = 1u64.checked_shl(attempt).unwrap_or(u64::MAX);
                let backoff_secs = cfg
                    .base_delay
                    .as_secs()
                    .saturating_mul(multiplier)
                    .min(cfg.max_delay.as_secs());

                // Honour server-requested Retry-After if it's larger than our backoff.
                let wait_secs = retry_after_secs
                    .map(|ra| ra.max(backoff_secs))
                    .unwrap_or(backoff_secs);

                tokio::time::sleep(Duration::from_secs(wait_secs)).await;
            }
        }
    }

    last_outcome
}

// ------------------------------------------------------------------
// Unit tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn req(url: &str) -> WebhookRequest {
        WebhookRequest {
            job_id: 1,
            url: url.to_string(),
            payload: serde_json::json!({}),
        }
    }

    #[test]
    fn two_hundred_is_delivered() {
        assert!(matches!(
            classify_status(200, None),
            WebhookOutcome::Delivered { status: 200 }
        ));
    }

    #[test]
    fn too_many_requests_keeps_retry_after() {
        assert!(matches!(
            classify_status(429, Some(30)),
            WebhookOutcome::Failed {
                status: Some(429),
                retryable: true,
                retry_after_secs: Some(30),
                ..
            }
        ));
    }

    #[test]
    fn server_error_is_retryable() {
        assert!(matches!(
            classify_status(503, None),
            WebhookOutcome::Failed {
                status: Some(503),
                retryable: true,
                ..
            }
        ));
    }

    #[test]
    fn client_error_is_not_retryable() {
        assert!(matches!(
            classify_status(404, None),
            WebhookOutcome::Failed {
                status: Some(404),
                retryable: false,
                ..
            }
        ));
    }

    #[test]
    fn retry_after_seconds_form() {
        assert_eq!(parse_retry_after("30"), Some(30));
        assert_eq!(parse_retry_after(" 5 "), Some(5));
        assert_eq!(parse_retry_after("0"), Some(0));
        assert_eq!(parse_retry_after("abc"), None);
        assert_eq!(parse_retry_after("-1"), None);
    }

    #[test]
    fn retry_after_http_date_form_past_returns_zero() {
        // A date far in the past should give 0 (saturating sub).
        let result = parse_retry_after("Wed, 21 Oct 2015 07:28:00 GMT");
        assert_eq!(result, Some(0));
    }

    #[test]
    fn retry_after_http_date_form_future() {
        // A date far in the future (year 2099) should give a large positive delta.
        let result = parse_retry_after("Mon, 01 Jan 2099 00:00:00 GMT");
        assert!(result.is_some());
        assert!(result.unwrap() > 0);
    }

    #[test]
    fn retry_after_invalid_date_returns_none() {
        assert_eq!(parse_retry_after("Not a date at all"), None);
    }

    #[tokio::test]
    async fn http_url_is_invalid() {
        let out = send_webhook(&Client::new(), &req("http://example.com/")).await;
        assert!(matches!(
            out,
            WebhookOutcome::Failed {
                kind: FailureKind::InvalidUrl,
                retryable: false,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn loopback_is_blocked() {
        let out = send_webhook(&Client::new(), &req("https://127.0.0.1/")).await;
        assert!(matches!(
            out,
            WebhookOutcome::Failed {
                kind: FailureKind::BlockedDestination,
                retryable: false,
                ..
            }
        ));
    }
}

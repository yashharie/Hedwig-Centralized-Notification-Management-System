use crate::host_policy::check_host;
use crate::types::{FailureKind, WebhookOutcome, WebhookRequest};
use crate::validate::parse_webhook_url;
use reqwest::{Client, header::RETRY_AFTER};
use std::error::Error;
use url::Url;

/// DRAFT classification. NOT agreed team policy.
pub fn classify_status(status: u16, retry_after_secs: Option<u64>) -> WebhookOutcome {
    if (200..300).contains(&status) {
        return WebhookOutcome::Delivered { status };
    }
    // DRAFT: 429 and 5xx retryable, everything else not. Confirm with team.
    let retryable = status == 429 || (500..600).contains(&status);
    WebhookOutcome::Failed {
        kind: FailureKind::HttpStatus,
        status: Some(status),
        retryable,
        retry_after_secs,
    }
}

/// Seconds form only. HTTP-date form is not supported yet.
pub fn parse_retry_after(value: &str) -> Option<u64> {
    value.trim().parse::<u64>().ok()
}

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

/// Sends the request. Does NOT validate the URL. Internal use only.
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
                // DRAFT: timeout retryable. Confirm with team.
                failed(FailureKind::Timeout, true)
            } else {
                // DRAFT: connection error retryable. Confirm with team.
                failed(FailureKind::Connection, true)
            }
        }
    }
}

/// Production entry point: validates URL and host, then sends.
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
    fn server_error_is_draft_retryable() {
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
    fn client_error_is_draft_not_retryable() {
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
    fn retry_after_parsing() {
        assert_eq!(parse_retry_after("30"), Some(30));
        assert_eq!(parse_retry_after(" 5 "), Some(5));
        assert_eq!(parse_retry_after("abc"), None);
        assert_eq!(parse_retry_after("-1"), None);
        assert_eq!(parse_retry_after("Wed, 21 Oct 2015 07:28:00 GMT"), None);
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

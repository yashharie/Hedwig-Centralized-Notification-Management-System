//! Axum middleware that enforces HMAC-SHA256 request authentication.
//!
//! # Usage
//!
//! Wrap any router that requires authentication:
//!
//! ```rust,ignore
//! use axum::middleware;
//! use crate::auth::middleware::require_hmac_auth;
//!
//! let protected = Router::new()
//!     .route("/api/v1/notifications", post(submit_notification))
//!     .layer(middleware::from_fn_with_state(state.clone(), require_hmac_auth));
//! ```
//!
//! # Required headers
//!
//! | Header               | Description                                        |
//! |----------------------|----------------------------------------------------|
//! | `X-Hedwig-App-Id`    | Identifier of the calling application              |
//! | `X-Hedwig-Timestamp` | Unix timestamp (seconds, UTC) of the request       |
//! | `X-Hedwig-Nonce`     | Random string ≥ 16 chars; must be unique per window|
//! | `X-Hedwig-Signature` | Lowercase hex HMAC-SHA256 of the canonical string  |
//!
//! # Rejection conditions
//!
//! | Condition                          | HTTP status | Error code           |
//! |------------------------------------|-------------|----------------------|
//! | Any required header missing        | 401         | `missing_auth_headers`|
//! | Timestamp not parseable            | 401         | `expired_timestamp`  |
//! | Timestamp outside tolerance window | 401         | `expired_timestamp`  |
//! | Nonce shorter than 16 characters   | 401         | `missing_auth_headers`|
//! | Nonce already seen (replay)        | 401         | `invalid_signature`  |
//! | Signature does not match           | 401         | `invalid_signature`  |

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::header::HeaderMap,
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use http_body_util::BodyExt;

use crate::{
    auth::hmac,
    errors::ApiError,
    state::AppState,
};

// Header name constants — agreed API contract shared with all client developers.
pub const HEADER_APP_ID: &str = "x-hedwig-app-id";
pub const HEADER_TIMESTAMP: &str = "x-hedwig-timestamp";
pub const HEADER_NONCE: &str = "x-hedwig-nonce";
pub const HEADER_SIGNATURE: &str = "x-hedwig-signature";

/// Minimum nonce length enforced server-side.
const MIN_NONCE_LEN: usize = 16;

// ── Middleware function ───────────────────────────────────────────────────────

/// Axum middleware that verifies HMAC-SHA256 authentication on every request.
///
/// Must be registered with `middleware::from_fn_with_state`.
pub async fn require_hmac_auth(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // ── 1. Extract required headers ───────────────────────────────────────────
    let headers = request.headers();

    let app_id = extract_header(headers, HEADER_APP_ID)
        .ok_or(ApiError::MissingAuthHeaders)?;

    let timestamp_str = extract_header(headers, HEADER_TIMESTAMP)
        .ok_or(ApiError::MissingAuthHeaders)?;

    let nonce = extract_header(headers, HEADER_NONCE)
        .ok_or(ApiError::MissingAuthHeaders)?;

    let provided_sig = extract_header(headers, HEADER_SIGNATURE)
        .ok_or(ApiError::MissingAuthHeaders)?;

    // ── 2. Validate nonce length ──────────────────────────────────────────────
    if nonce.len() < MIN_NONCE_LEN {
        tracing::warn!(
            app_id = %app_id,
            "Auth rejected: nonce too short ({} < {})",
            nonce.len(),
            MIN_NONCE_LEN,
        );
        return Err(ApiError::MissingAuthHeaders);
    }

    // ── 3. Validate timestamp (parse + window check) ──────────────────────────
    let request_ts: i64 = timestamp_str
        .parse()
        .map_err(|_| ApiError::ExpiredTimestamp)?;

    let now = Utc::now().timestamp();
    let tolerance = state.config.hmac_timestamp_tolerance_secs;
    let delta = (now - request_ts).abs();

    if delta > tolerance {
        tracing::warn!(
            app_id = %app_id,
            request_ts,
            now,
            delta,
            tolerance,
            "Auth rejected: timestamp outside window",
        );
        return Err(ApiError::ExpiredTimestamp);
    }

    // ── 4. Consume and buffer the request body ────────────────────────────────
    // We need the raw bytes to compute the body hash.  After buffering we
    // reconstruct the request so the next handler still receives the body.
    let (parts, body) = request.into_parts();

    let body_bytes = body
        .collect()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to read request body: {e}")))?
        .to_bytes();

    // ── 5. Verify HMAC signature ──────────────────────────────────────────────
    let method = parts.method.as_str();

    // Build path_and_query: prefer the full path+query, fall back to just path.
    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or(parts.uri.path());

    let signing_input = hmac::SigningInput {
        method,
        path_and_query,
        app_id: &app_id,
        timestamp: &timestamp_str,
        nonce: &nonce,
        body: &body_bytes,
    };

    let valid = hmac::verify_signature(
        state.config.hmac_secret.as_bytes(),
        &signing_input,
        &provided_sig,
    );

    if !valid {
        // Deliberately vague log — do NOT log the provided or expected signature.
        tracing::warn!(
            app_id = %app_id,
            method,
            path = path_and_query,
            "Auth rejected: signature mismatch",
        );
        return Err(ApiError::InvalidSignature);
    }

    // ── 6. Nonce replay check ─────────────────────────────────────────────────
    // Done AFTER signature verification so attackers can't probe the nonce
    // store without first knowing the secret.
    let accepted = state.nonce_store.check_and_insert(&app_id, &nonce);
    if !accepted {
        tracing::warn!(
            app_id = %app_id,
            nonce = %nonce,
            "Auth rejected: nonce replay detected",
        );
        return Err(ApiError::InvalidSignature);
    }

    // ── 7. Reconstruct request and pass to next handler ───────────────────────
    let request = Request::from_parts(parts, Body::from(body_bytes));

    Ok(next.run(request).await)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract a header value as a `String`.  Returns `None` if the header is
/// absent or its value contains non-UTF-8 bytes.
fn extract_header(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

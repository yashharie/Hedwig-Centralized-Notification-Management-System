//! Centralised error types and HTTP error responses for the Hedwig API.
//!
//! [`ApiError`] is the single error type returned by all handlers.  It
//! implements [`axum::response::IntoResponse`] so Axum can convert it directly
//! into a JSON HTTP response with the correct status code.
//!
//! All error responses follow this structure:
//! ```json
//! {
//!   "error": "short_snake_case_code",
//!   "message": "Human-readable explanation."
//! }
//! ```

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// A machine-readable + human-readable error returned by API handlers.
#[derive(Debug, Error)]
pub enum ApiError {
    // --- 400 Bad Request ---
    #[error("Invalid request payload: {0}")]
    InvalidPayload(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    // --- 401 Unauthorized ---
    #[error("Missing or malformed authentication headers")]
    MissingAuthHeaders,

    #[error("Request signature is invalid")]
    InvalidSignature,

    #[error("Request timestamp has expired or is too far in the future")]
    ExpiredTimestamp,

    // --- 403 Forbidden ---
    #[error("Access to this resource is not permitted")]
    Forbidden,

    // --- 404 Not Found ---
    #[error("The requested resource was not found")]
    NotFound,

    // --- 409 Conflict ---
    #[error("A duplicate request was detected: {0}")]
    Conflict(String),

    // --- 422 Unprocessable Entity ---
    #[error("Unprocessable entity: {0}")]
    UnprocessableEntity(String),

    // --- 429 Too Many Requests ---
    #[error("Rate limit exceeded. Slow down and retry.")]
    RateLimitExceeded,

    // --- 500 Internal Server Error ---
    #[error("An unexpected internal error occurred")]
    Internal(String),

    // --- 503 Service Unavailable ---
    #[error("A required dependency is not available")]
    ServiceUnavailable(String),
}

/// The JSON body sent for every error response.
#[derive(Serialize)]
struct ErrorBody {
    /// Short snake_case error code clients can match on programmatically.
    error: &'static str,
    /// Human-readable description of what went wrong.
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            ApiError::InvalidPayload(_) => (StatusCode::BAD_REQUEST, "invalid_payload"),
            ApiError::Validation(_) => (StatusCode::BAD_REQUEST, "validation_error"),
            ApiError::MissingAuthHeaders => (StatusCode::UNAUTHORIZED, "missing_auth_headers"),
            ApiError::InvalidSignature => (StatusCode::UNAUTHORIZED, "invalid_signature"),
            ApiError::ExpiredTimestamp => (StatusCode::UNAUTHORIZED, "expired_timestamp"),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            ApiError::UnprocessableEntity(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "unprocessable_entity")
            }
            ApiError::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, "rate_limit_exceeded"),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
            ApiError::ServiceUnavailable(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable")
            }
        };

        // Log internal errors with full detail; surface a safe message to clients.
        let message = match &self {
            ApiError::Internal(detail) => {
                tracing::error!(error_detail = %detail, "Internal server error");
                "An unexpected error occurred. Please try again later.".to_string()
            }
            other => other.to_string(),
        };

        let body = Json(ErrorBody { error: code, message });
        (status, body).into_response()
    }
}

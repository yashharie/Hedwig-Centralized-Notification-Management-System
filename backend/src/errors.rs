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
//!   "message": "Human-readable explanation.",
//!   "fields": [{ "field": "name", "message": "is required" }]
//! }
//! ```
//! The `fields` key is only present for field-level validation errors.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// A single field-level validation failure.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationFieldError {
    /// The name of the field that failed validation.
    pub field: String,
    /// A human-readable description of why the field is invalid.
    pub message: String,
}

/// A machine-readable + human-readable error returned by API handlers.
#[derive(Debug, Error)]
pub enum ApiError {
    // --- 400 Bad Request ---
    #[error("Invalid request payload: {0}")]
    InvalidPayload(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Validation failed")]
    ValidationFields(Vec<ValidationFieldError>),

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
    /// Per-field validation errors, present only for `ValidationFields` responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<Vec<ValidationFieldError>>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            ApiError::InvalidPayload(_) => (StatusCode::BAD_REQUEST, "invalid_payload"),
            ApiError::Validation(_) => (StatusCode::BAD_REQUEST, "validation_error"),
            ApiError::ValidationFields(_) => (StatusCode::BAD_REQUEST, "validation_error"),
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

        // Build the message and optional fields payload.
        let (message, fields) = match self {
            ApiError::Internal(ref detail) => {
                tracing::error!(error_detail = %detail, "Internal server error");
                (
                    "An unexpected error occurred. Please try again later.".to_string(),
                    None,
                )
            }
            ApiError::ValidationFields(ref field_errors) => (
                "One or more fields failed validation.".to_string(),
                Some(field_errors.clone()),
            ),
            ref other => (other.to_string(), None),
        };

        let body = Json(ErrorBody { error: code, message, fields });
        (status, body).into_response()
    }
}

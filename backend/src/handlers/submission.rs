use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use crate::state::AppState;
use crate::batch::service::{self, NewNotificationInput};

#[derive(Debug, Deserialize)]
pub struct SubmitBatchRequest {
    pub application_id: u64,
    pub notifications: Vec<NewNotificationInput>,
}

pub async fn submit_batch(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SubmitBatchRequest>,
) -> impl IntoResponse {
    if request.application_id == 0
        || request.notifications.is_empty()
        || request.notifications.len() > 100
        || request.notifications.iter().any(|n| {
            n.recipient.trim().is_empty()
                || n.body.trim().is_empty()
                || !["email", "webhook"].contains(&n.channel.as_str())
                || n.idempotency_key.as_ref().is_some_and(|k| k.len() > 100)
        })
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"invalid_batch_request"})),
        );
    }

    let Some(pool) = &state.db else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error":"database_unavailable"})),
        );
    };

    match service::submit_batch(pool, request.application_id, &request.notifications).await {
        Ok(batch) => (StatusCode::CREATED, Json(json!(batch))),
        Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("1062") => (
            StatusCode::CONFLICT,
            Json(json!({"error":"duplicate_idempotency_key"})),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":"batch_submission_failed"})),
        ),
    }
}

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub limit: Option<u32>,
    pub offset: Option<u64>,
}

pub async fn get_batch(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let Some(pool) = &state.db else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "database_unavailable"})),
        );
    };

    match crate::batch::service::get_batch_status(pool, id).await {
        Ok(Some(batch)) => (StatusCode::OK, Json(json!(batch))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "batch_not_found"})),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "database_error"})),
        ),
    }
}

pub async fn list_batch_notifications(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
    Query(pagination): Query<Pagination>,
) -> impl IntoResponse {
    let Some(pool) = &state.db else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "database_unavailable"})),
        );
    };

    let limit = pagination.limit.unwrap_or(20).clamp(1, 100);
    let offset = pagination.offset.unwrap_or(0);

    match crate::batch::service::get_batch_notifications(pool, id, limit, offset).await {
        Ok(notifications) => (StatusCode::OK, Json(json!({
            "batch_id": id,
            "limit": limit,
            "offset": offset,
            "notifications": notifications
        }))),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "database_error"})),
        ),
    }
}

pub async fn cancel_notification(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let Some(pool) = &state.db else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "database_unavailable"})),
        );
    };

    match crate::batch::service::cancel_notification(pool, id).await {
        Ok(true) => (
            StatusCode::OK,
            Json(json!({"notification_id": id, "status": "cancelled"})),
        ),
        Ok(false) => (
            StatusCode::CONFLICT,
            Json(json!({
                "error": "not_cancellable",
                "message": "Notification was not found or is no longer queued."
            })),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "database_error"})),
        ),
    }
}

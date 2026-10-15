use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use std::sync::Arc;
use crate::state::AppState;

pub async fn get_notification(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let Some(pool) = &state.db else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error":"database_unavailable"})));
    };

    match crate::batch::service::get_notification_status(pool, id).await {
        Ok(Some(notification)) => (StatusCode::OK, Json(json!(notification))),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error":"notification_not_found"}))),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"database_error"}))),
    }
}

pub async fn get_delivery_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let Some(pool) = &state.db else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error":"database_unavailable"})));
    };

    match crate::batch::service::get_delivery_history(pool, id).await {
        Ok(history) => (StatusCode::OK, Json(json!({
            "notification_id": id,
            "history": history
        }))),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"database_error"}))),
    }
}

//! Axum router construction.
//!
//! All routes are assembled here.  The function [`build_router`] is the single
//! source of truth for the application's URL structure.
//!
//! ## Route groups
//!
//! | Group           | Prefix          | Auth required | Owner        |
//! |-----------------|-----------------|---------------|--------------|
//! | Health checks   | `/health`       | No            | Developer 1  |
//! | Notifications   | `/api/v1`       | Yes (HMAC)    | Developer 1 + 5 |
//! | Batches         | `/api/v1`       | Yes (HMAC)    | Developer 1 + 5 |
//!
//! Authentication middleware (Task 2) will be layered onto the `/api/v1`
//! sub-router once it is implemented.  Stub handlers are used for now so
//! that routing can be tested independently.

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::{handlers::health, state::AppState};

/// Build and return the complete Axum [`Router`].
///
/// Called once in `main` after [`AppState`] is constructed.
pub fn build_router(state: Arc<AppState>) -> Router {
    // ── Health routes (no authentication) ────────────────────────────────────
    let health_router = Router::new()
        .route("/live", get(health::live))
        .route("/ready", get(health::ready));

    // ── API v1 routes (HMAC authentication will be added in Task 2) ──────────
    //
    // Stub handlers are `todo!()` placeholders that return 501 Not Implemented.
    // Developer 5 will replace batch/notification business logic handlers.
    // Developer 1 owns routing and auth middleware.
    let api_v1_router = Router::new()
        // Single notification submission — Developer 1 wires auth; business
        // logic (persisting the notification + job) belongs to Developer 2 & 5.
        .route("/notifications", post(stubs::submit_notification))
        // Batch submission
        .route("/batches", post(crate::handlers::submission::submit_batch))
        // Status queries
        .route("/notifications/{id}", get(crate::handlers::notification::get_notification))
        .route("/notifications/{id}/history", get(crate::handlers::notification::get_delivery_history))
        .route("/batches/{id}", get(crate::handlers::batch::get_batch))
        .route("/batches/{id}/notifications", get(crate::handlers::batch::list_batch_notifications))
        // Cancellation
        .route("/notifications/{id}/cancel", post(crate::handlers::batch::cancel_notification));

    // ── Assemble full router ──────────────────────────────────────────────────
    Router::new()
        .nest("/health", health_router)
        .nest("/api/v1", api_v1_router)
        .with_state(state)
}

// ── Stub handlers ─────────────────────────────────────────────────────────────
//
// These stubs return 501 Not Implemented.  They exist so the full route tree
// compiles and can be tested for routing correctness before business logic
// modules are integrated.  Replace each stub with the real handler once the
// owning developer's module is ready.

mod stubs {
    use axum::{extract::Path, http::StatusCode, response::IntoResponse, Json};
    use serde_json::json;
    use uuid::Uuid;

    pub async fn submit_notification() -> impl IntoResponse {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({
                "error": "not_implemented",
                "message": "Notification submission handler not yet connected."
            })),
        )
    }

    pub async fn submit_batch() -> impl IntoResponse {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({
                "error": "not_implemented",
                "message": "Batch submission handler not yet connected."
            })),
        )
    }

    pub async fn get_notification(Path(_id): Path<Uuid>) -> impl IntoResponse {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({
                "error": "not_implemented",
                "message": "Notification status handler not yet connected."
            })),
        )
    }

    pub async fn get_batch(Path(_id): Path<Uuid>) -> impl IntoResponse {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({
                "error": "not_implemented",
                "message": "Batch status handler not yet connected."
            })),
        )
    }

    pub async fn list_batch_notifications(Path(_id): Path<Uuid>) -> impl IntoResponse {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({
                "error": "not_implemented",
                "message": "Batch notifications list handler not yet connected."
            })),
        )
    }

    pub async fn cancel_notification(Path(_id): Path<Uuid>) -> impl IntoResponse {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({
                "error": "not_implemented",
                "message": "Notification cancellation handler not yet connected."
            })),
        )
    }
}





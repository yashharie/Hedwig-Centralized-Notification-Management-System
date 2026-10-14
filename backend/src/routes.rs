//! Axum router construction.
//!
//! All routes are assembled here.  The function [`build_router`] is the single
//! source of truth for the application's URL structure.
//!
//! ## Route groups
//!
//! | Group           | Prefix          | Auth required | Owner           |
//! |-----------------|-----------------|---------------|-----------------|
//! | Health checks   | `/health`       | No            | Developer 1     |
//! | Notifications   | `/api/v1`       | Yes (HMAC)    | Developer 1 + 5 |
//! | Batches         | `/api/v1`       | Yes (HMAC)    | Developer 1 + 5 |

use std::sync::Arc;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};

use crate::{
    auth::middleware::require_hmac_auth,
    handlers::health,
    state::AppState,
};

/// Build and return the complete Axum [`Router`].
///
/// Called once in `main` after [`AppState`] is constructed.
pub fn build_router(state: Arc<AppState>) -> Router {
    // ── Health routes (no authentication) ────────────────────────────────────
    let health_router = Router::new()
        .route("/live", get(health::live))
        .route("/ready", get(health::ready));

    // ── API v1 routes (protected by HMAC auth middleware) ────────────────────
    //
    // Stub handlers return 501 Not Implemented until Developer 5's business
    // logic modules are integrated.  Developer 1 owns routing + auth.
    let api_v1_router = Router::new()
        .route("/notifications", post(stubs::submit_notification))
        .route("/batches", post(stubs::submit_batch))
        .route("/notifications/{id}", get(stubs::get_notification))
        .route("/batches/{id}", get(stubs::get_batch))
        .route("/batches/{id}/notifications", get(stubs::list_batch_notifications))
        .route("/notifications/{id}/cancel", post(stubs::cancel_notification))
        // Apply HMAC auth to every route in this sub-router.
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_hmac_auth,
        ));

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

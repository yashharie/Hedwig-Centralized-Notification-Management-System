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

    // ── API v1 routes (protected by HMAC auth middleware) ─────────────────────
    let api_v1_router = Router::new()
        // Single notification submission (Developer 1/2 — stub until wired)
        .route("/notifications", post(stubs::submit_notification))
        // Batch submission (Developer 5)
        .route("/batches", post(crate::handlers::submission::submit_batch))
        // Single-notification status and delivery history (Developer 5)
        .route(
            "/notifications/{id}",
            get(crate::handlers::notification::get_notification),
        )
        .route(
            "/notifications/{id}/history",
            get(crate::handlers::notification::get_delivery_history),
        )
        // Batch status and paginated notification list (Developer 5)
        .route("/batches/{id}", get(crate::handlers::batch::get_batch))
        .route(
            "/batches/{id}/notifications",
            get(crate::handlers::batch::list_batch_notifications),
        )
        // Notification cancellation (Developer 5)
        .route(
            "/notifications/{id}/cancel",
            post(crate::handlers::batch::cancel_notification),
        )
        // Apply HMAC auth middleware to every route in this sub-router (Developer 1)
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
// Replace each stub with the real handler once the owning developer's module
// is ready.  Stubs return 501 Not Implemented so the route tree compiles.

mod stubs {
    use axum::{http::StatusCode, response::IntoResponse, Json};
    use serde_json::json;

    /// `POST /api/v1/notifications`
    ///
    /// Owned by Developer 1/2 (single-notification auth + submission flow).
    pub async fn submit_notification() -> impl IntoResponse {
        (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({
                "error": "not_implemented",
                "message": "Single-notification submission handler not yet connected."
            })),
        )
    }
}

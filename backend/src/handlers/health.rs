//! Health-check handlers.
//!
//! Two endpoints are exposed:
//!
//! * `GET /health/live`  — Liveness probe.  Returns 200 as long as the process
//!   is running and the event loop is responsive.  No external dependencies
//!   are checked.  Orchestrators (Kubernetes, load balancers) use this to
//!   decide whether to restart the process.
//!
//! * `GET /health/ready` — Readiness probe.  Returns 200 only when all
//!   required dependencies (currently the MySQL connection pool) are healthy.
//!   Orchestrators use this to decide whether to route traffic to the instance.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

use crate::state::AppState;

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct LivenessResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct ReadinessResponse {
    status: &'static str,
    database: &'static str,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// `GET /health/live`
///
/// Returns `200 OK` with `{ "status": "ok" }` if the process is alive.
/// This handler intentionally has no side effects and performs no I/O.
pub async fn live() -> impl IntoResponse {
    (StatusCode::OK, Json(LivenessResponse { status: "ok" }))
}

/// `GET /health/ready`
///
/// Pings the database.  Returns:
/// * `200 OK`            — all dependencies healthy.
/// * `503 Service Unavailable` — at least one dependency is not reachable.
pub async fn ready(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db_status = match &state.db {
        Some(pool) => {
            match sqlx::query("SELECT 1").execute(pool).await {
                Ok(_) => "ok",
                Err(err) => {
                    // Log the real error server-side; return a safe message.
                    tracing::warn!(error = %err, "Readiness check: database ping failed");
                    "unavailable"
                }
            }
        }
        None => {
            tracing::warn!("Readiness check: no database pool configured");
            "unavailable"
        }
    };

    let http_status = if db_status == "ok" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        http_status,
        Json(ReadinessResponse {
            status: if db_status == "ok" { "ok" } else { "degraded" },
            database: db_status,
        }),
    )
}

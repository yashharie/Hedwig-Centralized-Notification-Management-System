//! Hedwig — Centralized Notification Management System
//!
//! Entry point.  Responsibilities:
//! 1. Load `.env` (if present) and parse [`Config`].
//! 2. Initialise structured logging.
//! 3. Connect to MySQL and run pending migrations.
//! 4. Build [`AppState`] and the Axum router.
//! 5. Bind the TCP listener and start serving.

mod config;
mod errors;
mod handlers;
mod routes;
mod state;

use std::sync::Arc;

use sqlx::mysql::MySqlPoolOptions;
use tracing::info;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() {
    // ── 1. Load environment ───────────────────────────────────────────────────
    // dotenvy::dotenv() loads `.env` if it exists; it is not an error if the
    // file is absent (production environments supply vars directly).
    let _ = dotenvy::dotenv();

    // ── 2. Initialise structured logging ─────────────────────────────────────
    // Respects the RUST_LOG env var.  Default: info-level for this crate.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hedwig_backend=info,tower_http=info".into()),
        )
        .init();

    // ── 3. Parse configuration ────────────────────────────────────────────────
    let config = Config::from_env().unwrap_or_else(|e| {
        // Fatal — refuse to start with missing or invalid configuration.
        tracing::error!(error = %e, "Failed to load configuration");
        std::process::exit(1);
    });

    let addr = config.socket_addr();

    // ── 4. Connect to MySQL ───────────────────────────────────────────────────
    info!("Connecting to database…");
    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "Failed to connect to the database");
            std::process::exit(1);
        });

    // Run any pending SQLx migrations (Developer 2 will add migration files).
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "Database migration failed");
            std::process::exit(1);
        });

    info!("Database connection established and migrations applied.");

    // ── 5. Build application state and router ────────────────────────────────
    let state: Arc<AppState> = AppState::new(config, pool);
    let app = routes::build_router(state);

    // ── 6. Bind and serve ────────────────────────────────────────────────────
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, addr = %addr, "Failed to bind TCP listener");
            std::process::exit(1);
        });

    info!(addr = %addr, "Hedwig is listening");

    axum::serve(listener, app)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "Server terminated with error");
            std::process::exit(1);
        });
}

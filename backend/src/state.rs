//! Shared application state injected into every Axum handler via [`Extension`]
//! or [`State`].
//!
//! [`AppState`] is constructed once in `main` and cloned cheaply (it wraps
//! an [`Arc`] internally via Axum's state sharing mechanism).  Add only values
//! that handlers legitimately need at runtime.  Secrets are stored here only
//! as long as they are needed for request processing; they must never be
//! logged or serialised.

use std::sync::Arc;

use sqlx::MySqlPool;

use crate::{auth::nonce::NonceStore, config::Config};

/// Inner state shared across the entire application.
///
/// Wrapped in an `Arc` by Axum when passed to `.with_state(...)`.  Handlers
/// receive a `State<Arc<AppState>>` or `State<AppState>` depending on how the
/// router is configured — we use `Arc<AppState>` so cloning the state is
/// zero-cost.
pub struct AppState {
    /// Application configuration (host, port, secrets, tolerances, …).
    pub config: Config,

    /// MySQL connection pool.
    ///
    /// `None` during startup until the pool is successfully created, or when
    /// the database is not yet available (e.g. in unit tests that don't need
    /// a real DB).  Health checks use this to determine readiness.
    pub db: Option<MySqlPool>,

    /// In-process nonce store for HMAC replay prevention.
    ///
    /// Keyed by `(app_id, nonce)`.  Entries expire after
    /// `2 × hmac_timestamp_tolerance_secs`.
    pub nonce_store: NonceStore,
}

impl AppState {
    /// Create a new [`AppState`] with a live database pool.
    pub fn new(config: Config, db: MySqlPool) -> Arc<Self> {
        let tolerance = config.hmac_timestamp_tolerance_secs;
        Arc::new(Self {
            config,
            db: Some(db),
            nonce_store: NonceStore::new(tolerance),
        })
    }

    /// Create a minimal [`AppState`] without a database pool.
    ///
    /// Useful in tests that only exercise request routing or authentication
    /// without touching the database.
    #[cfg(test)]
    pub fn without_db(config: Config) -> Arc<Self> {
        let tolerance = config.hmac_timestamp_tolerance_secs;
        Arc::new(Self {
            nonce_store: NonceStore::new(tolerance),
            config,
            db: None,
        })
    }
}

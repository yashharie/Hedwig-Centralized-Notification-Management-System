//! Application configuration loaded from environment variables.
//!
//! Call [`Config::from_env`] once at startup after loading `.env`.
//! The returned [`Config`] is stored in [`AppState`](crate::state::AppState)
//! and shared across all request handlers.

use std::env;

/// All runtime configuration for Hedwig.
///
/// Values come from environment variables (or a `.env` file loaded by
/// [`dotenvy`]).  Defaults are provided where safe; secrets are required
/// explicitly so the server refuses to start with missing credentials.
#[derive(Debug, Clone)]
pub struct Config {
    /// IP address the server binds to.
    pub host: String,

    /// TCP port the server listens on.
    pub port: u16,

    /// Full MySQL connection URL.
    /// Format: `mysql://user:password@host:port/database`
    pub database_url: String,

    /// HMAC-SHA256 secret shared with client applications.
    /// Required — the server will not start without it.
    pub hmac_secret: String,

    /// How many seconds of clock skew to tolerate in HMAC timestamps.
    /// Requests whose `X-Hedwig-Timestamp` differs from `now()` by more than
    /// this value are rejected as expired or future-dated.
    pub hmac_timestamp_tolerance_secs: i64,
}

impl Config {
    /// Load configuration from the process environment.
    ///
    /// # Errors
    ///
    /// Returns a descriptive error string when a required variable is missing
    /// or a value cannot be parsed.  The caller should treat this as fatal.
    pub fn from_env() -> Result<Self, String> {
        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .map_err(|e| format!("Invalid PORT value: {e}"))?;

        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL is required but was not set".to_string())?;

        let hmac_secret = env::var("HMAC_SECRET")
            .map_err(|_| "HMAC_SECRET is required but was not set".to_string())?;

        if hmac_secret.len() < 32 {
            return Err(
                "HMAC_SECRET must be at least 32 characters long".to_string()
            );
        }

        let hmac_timestamp_tolerance_secs = env::var("HMAC_TIMESTAMP_TOLERANCE_SECS")
            .unwrap_or_else(|_| "300".to_string())
            .parse::<i64>()
            .map_err(|e| format!("Invalid HMAC_TIMESTAMP_TOLERANCE_SECS value: {e}"))?;

        Ok(Self {
            host,
            port,
            database_url,
            hmac_secret,
            hmac_timestamp_tolerance_secs,
        })
    }

    /// Returns the socket address string (`host:port`) for binding.
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

use std::env;

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub from: String,
    pub use_tls: bool,
}

impl SmtpConfig {
    pub fn from_env() -> Self {
        Self {
            host: env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".into()),
            port: env::var("SMTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(1025),
            username: env::var("SMTP_USERNAME").ok(),
            password: env::var("SMTP_PASSWORD").ok(),
            from: env::var("SMTP_FROM")
                .unwrap_or_else(|_| "Hedwig <hedwig@example.com>".into()),
            use_tls: env::var("SMTP_USE_TLS")
                .map(|v| v == "true")
                .unwrap_or(false),
        }
    }
}
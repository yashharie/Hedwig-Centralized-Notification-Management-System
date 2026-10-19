use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmailError {
    /// Retry pannalam (server busy, timeout, connection problem)
    #[error("temporary email error: {0}")]
    Temporary(String),

    /// Retry vendaam (invalid address, auth failed)
    #[error("permanent email error: {0}")]
    Permanent(String),
}

impl EmailError {
    pub fn should_retry(&self) -> bool {
        matches!(self, EmailError::Temporary(_))
    }
}
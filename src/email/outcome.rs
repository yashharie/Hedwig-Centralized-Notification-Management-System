use std::time::{SystemTime, UNIX_EPOCH};

use super::{error::EmailError, job::EmailJob};

#[derive(Debug, Clone, PartialEq)]
pub enum OutcomeStatus {
    /// SMTP server mail-a accept panniduchu (inbox-ku pogum nu guarantee illa)
    Accepted,
    /// Retry pannalam
    TemporaryFailure,
    /// Retry vendaam
    PermanentFailure,
}

#[derive(Debug, Clone)]
pub struct DeliveryOutcome {
    pub notification_id: u64,
    pub attempt_number: u32,
    pub status: OutcomeStatus,
    pub smtp_code: Option<String>,
    pub message: String,
    pub timestamp_secs: u64,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl DeliveryOutcome {
    pub fn accepted(job: &EmailJob, attempt_number: u32, code: String, message: String) -> Self {
        Self {
            notification_id: job.notification_id,
            attempt_number,
            status: OutcomeStatus::Accepted,
            smtp_code: Some(code),
            message,
            timestamp_secs: now_secs(),
        }
    }

    pub fn failed(
        job: &EmailJob,
        attempt_number: u32,
        error: &EmailError,
        smtp_code: Option<String>,
    ) -> Self {
        let status = if error.should_retry() {
            OutcomeStatus::TemporaryFailure
        } else {
            OutcomeStatus::PermanentFailure
        };

        Self {
            notification_id: job.notification_id,
            attempt_number,
            status,
            smtp_code,
            message: error.to_string(),
            timestamp_secs: now_secs(),
        }
    }
}
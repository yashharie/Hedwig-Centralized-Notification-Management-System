use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct Notification {
    pub id: u64,
    pub application_id: u64,
    pub batch_id: Option<u64>,
    pub idempotency_key: Option<String>,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct NewNotification {
    pub application_id: u64,
    pub batch_id: Option<u64>,
    pub idempotency_key: Option<String>,
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
}
#[derive(Debug)]
pub struct NewDeliveryJob {
    pub notification_id: u64,
    pub channel: String,
    pub max_attempts: u32,

}

#[derive(Debug, FromRow)]
pub struct NotificationBatch {
    pub id: u64,
    pub application_id: u64,
    pub status: String,
    pub total_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct NewDeliveryAttempt {
    pub job_id: u64,
    pub outcome: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct ClaimedJob {
    pub id: u64,
    pub notification_id: u64,
    pub channel: String,
    pub attempt_count: u32,
    pub max_attempts: u32,
    pub lease_token: Option<String>,
}
use chrono::NaiveDateTime;
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct BatchRecord {
    pub id: u64,
    pub application_id: u64,
    pub status: String,
    pub total_count: u32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct BatchNotification {
    pub id: u64,
    pub batch_id: Option<u64>,
    pub recipient: String,
    pub subject: Option<String>,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DeliveryHistoryRecord {
    pub id: u64,
    pub job_id: u64,
    pub attempt_number: u32,
    pub outcome: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub finished_at: Option<NaiveDateTime>,
}

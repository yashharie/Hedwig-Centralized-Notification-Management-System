use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Email,
    Webhook,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BodyType {
    Plain,
    Html,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebhookMethod {
    Post,
    Put,
    Patch,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmailPayload {
    /// Recipient email addresses (1–50).
    pub to: Vec<String>,
    /// Email subject line (1–998 characters).
    pub subject: String,
    /// Email body content.
    pub body: String,
    /// Body content type. Defaults to "plain" if omitted.
    pub body_type: Option<BodyType>,
    /// Sender address override. Uses system default if omitted.
    pub from: Option<String>,
    /// Reply-To address.
    pub reply_to: Option<String>,
    /// CC recipients (0–50).
    pub cc: Option<Vec<String>>,
    /// BCC recipients (0–50).
    pub bcc: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WebhookPayload {
    /// Destination HTTPS URL.
    pub url: String,
    /// JSON payload to deliver.
    pub payload: serde_json::Value,
    /// HTTP method. Defaults to POST.
    pub method: Option<WebhookMethod>,
    /// Custom headers to include (0–20 entries).
    pub headers: Option<HashMap<String, String>>,
    /// Request timeout in seconds (1–30). Defaults to 10.
    pub timeout_secs: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubmitNotificationRequest {
    /// Delivery channel.
    pub channel: Channel,
    /// Email-specific fields. Required when channel is "email".
    pub email: Option<EmailPayload>,
    /// Webhook-specific fields. Required when channel is "webhook".
    pub webhook: Option<WebhookPayload>,
    /// Arbitrary string metadata (max 20 keys).
    pub metadata: Option<HashMap<String, String>>,
    /// Idempotency key (max 128 chars). Prevents duplicate submissions.
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationStatus {
    Pending,
    Processing,
    Delivered,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct NotificationResponse {
    /// Unique notification identifier.
    pub id: Uuid,
    /// Delivery channel.
    pub channel: Channel,
    /// Current delivery status.
    pub status: NotificationStatus,
    /// Number of delivery attempts made.
    pub attempts: u32,
    /// Last error message if status is failed.
    pub last_error: Option<String>,
    /// When the notification was created.
    pub created_at: DateTime<Utc>,
    /// When the notification was last updated.
    pub updated_at: DateTime<Utc>,
}

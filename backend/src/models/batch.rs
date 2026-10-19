use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::notification::{NotificationResponse, SubmitNotificationRequest};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubmitBatchRequest {
    /// List of notifications to submit (1–100 items).
    pub notifications: Vec<SubmitNotificationRequest>,
    /// Arbitrary string metadata (max 20 keys).
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    Pending,
    Processing,
    Completed,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchResponse {
    /// Unique batch identifier.
    pub id: Uuid,
    /// Aggregate batch status.
    pub status: BatchStatus,
    /// Total number of notifications in the batch.
    pub total: u32,
    /// Count with status pending.
    pub pending: u32,
    /// Count with status processing.
    pub processing: u32,
    /// Count with status delivered.
    pub delivered: u32,
    /// Count with status failed.
    pub failed: u32,
    /// Count with status cancelled.
    pub cancelled: u32,
    /// When the batch was created.
    pub created_at: DateTime<Utc>,
    /// When the batch was last updated.
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BatchNotificationsQuery {
    /// Page number (default 1).
    pub page: Option<u32>,
    /// Items per page (default 20, max 100).
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Pagination {
    /// Total number of items.
    pub total: u64,
    /// Current page number.
    pub page: u32,
    /// Items per page.
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PaginatedNotifications {
    /// Notification items on this page.
    pub data: Vec<NotificationResponse>,
    /// Pagination metadata.
    pub pagination: Pagination,
}

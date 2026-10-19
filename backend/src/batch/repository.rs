use sqlx::MySqlPool;

use super::model::{
    BatchNotification,
    BatchRecord,
    DeliveryHistoryRecord,
};

pub async fn get_batch(
    pool: &MySqlPool,
    batch_id: u64,
) -> Result<Option<BatchRecord>, sqlx::Error> {
    sqlx::query_as::<_, BatchRecord>(
        "SELECT id, application_id, status, total_count, created_at, updated_at
         FROM notification_batches
         WHERE id = ?",
    )
    .bind(batch_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_batch_notifications(
    pool: &MySqlPool,
    batch_id: u64,
    limit: u32,
    offset: u64,
) -> Result<Vec<BatchNotification>, sqlx::Error> {
    sqlx::query_as::<_, BatchNotification>(
        "SELECT id, batch_id, recipient, subject, status, created_at, updated_at
         FROM notifications
         WHERE batch_id = ?
         ORDER BY id
         LIMIT ? OFFSET ?",
    )
    .bind(batch_id)
    .bind(limit.clamp(1, 100))
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn get_delivery_history(
    pool: &MySqlPool,
    notification_id: u64,
) -> Result<Vec<DeliveryHistoryRecord>, sqlx::Error> {
    sqlx::query_as::<_, DeliveryHistoryRecord>(
        "SELECT da.id, da.job_id, da.attempt_number, da.outcome,
                da.error_code, da.error_message, da.finished_at
         FROM delivery_attempts da
         INNER JOIN delivery_jobs dj ON dj.id = da.job_id
         WHERE dj.notification_id = ?
         ORDER BY da.attempt_number",
    )
    .bind(notification_id)
    .fetch_all(pool)
    .await
}

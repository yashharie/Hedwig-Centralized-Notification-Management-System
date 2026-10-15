use super::models::NotificationBatch;
use super::DbError;
use sqlx::MySqlPool;

/// Fetches one batch, or DbError::NotFound.
pub async fn get_batch_by_id(
    pool: &MySqlPool,
    id: u64,
) -> Result<NotificationBatch, DbError> {
    sqlx::query_as::<_, NotificationBatch>(
        "SELECT * FROM notification_batches WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::NotFound)
}
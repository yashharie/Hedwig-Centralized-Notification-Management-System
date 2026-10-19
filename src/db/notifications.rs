use super::models::{NewNotification, Notification};
use super::DbError;
use sqlx::MySqlPool;

/// Creates a notification and returns its new id.
pub async fn create_notification(
    pool: &MySqlPool,
    new: &NewNotification,
) -> Result<u64, DbError> {
    let result = sqlx::query(
        "INSERT INTO notifications
         (application_id, batch_id, idempotency_key, recipient, subject, body)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(new.application_id)
    .bind(new.batch_id)
    .bind(&new.idempotency_key)
    .bind(&new.recipient)
    .bind(&new.subject)
    .bind(&new.body)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id())
}

/// Fetches one notification, or DbError::NotFound.
pub async fn get_notification_by_id(
    pool: &MySqlPool,
    id: u64,
) -> Result<Notification, DbError> {
    sqlx::query_as::<_, Notification>("SELECT * FROM notifications WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(DbError::NotFound)
}
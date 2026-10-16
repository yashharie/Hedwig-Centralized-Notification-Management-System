use super::models::NewDeliveryJob;
use super::DbError;
use sqlx::MySqlPool;

/// Creates a delivery job (status 'queued') and returns its id.
pub async fn create_delivery_job(
    pool: &MySqlPool,
    new: &NewDeliveryJob,
) -> Result<u64, DbError> {
    let result = sqlx::query(
        "INSERT INTO delivery_jobs (notification_id, channel, max_attempts)
         VALUES (?, ?, ?)",
    )
    .bind(new.notification_id)
    .bind(&new.channel)
    .bind(new.max_attempts)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id())
}
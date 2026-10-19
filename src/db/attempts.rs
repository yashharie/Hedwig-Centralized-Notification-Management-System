use super::models::NewDeliveryAttempt;
use super::DbError;
use sqlx::MySqlPool;

/// Records one delivery attempt in the history table.
/// The attempt number is attempt_count + 1 (the attempt being made).
/// It does NOT change attempt_count: fail_job() does that.
pub async fn record_delivery_attempt(
    pool: &MySqlPool,
    new: &NewDeliveryAttempt,
) -> Result<u64, DbError> {
    let mut tx = pool.begin().await?;

    let current: Option<(u32,)> = sqlx::query_as(
        "SELECT attempt_count FROM delivery_jobs WHERE id = ? FOR UPDATE",
    )
    .bind(new.job_id)
    .fetch_optional(&mut *tx)
    .await?;

    let attempt_number = match current {
        Some((count,)) => count + 1,
        None => return Err(DbError::NotFound),
    };

    let result = sqlx::query(
        "INSERT INTO delivery_attempts
         (job_id, attempt_number, outcome, error_code, error_message, finished_at)
         VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)",
    )
    .bind(new.job_id)
    .bind(attempt_number)
    .bind(&new.outcome)
    .bind(&new.error_code)
    .bind(&new.error_message)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(result.last_insert_id())
}
use super::models::ClaimedJob;
use super::DbError;
use sqlx::MySqlPool;
use uuid::Uuid;

/// Atomically claims one eligible job for `worker_id`.
/// Returns Ok(None) when no job is available.
pub async fn claim_next_job(
    pool: &MySqlPool,
    worker_id: &str,
    lease_seconds: i64,
) -> Result<Option<ClaimedJob>, DbError> {
    let mut tx = pool.begin().await?;

    let row: Option<(u64,)> = sqlx::query_as(
        "SELECT id FROM delivery_jobs
         WHERE status = 'queued' AND next_attempt_at <= CURRENT_TIMESTAMP
         ORDER BY next_attempt_at, id
         LIMIT 1
         FOR UPDATE SKIP LOCKED",
    )
    .fetch_optional(&mut *tx)
    .await?;

    let job_id = match row {
        Some((id,)) => id,
        None => {
            tx.commit().await?;
            return Ok(None);
        }
    };

    let token = Uuid::new_v4().to_string();

    sqlx::query(
        "UPDATE delivery_jobs
         SET status = 'processing',
             locked_by = ?,
             lease_token = ?,
             lease_expires_at = DATE_ADD(CURRENT_TIMESTAMP, INTERVAL ? SECOND)
         WHERE id = ?",
    )
    .bind(worker_id)
    .bind(&token)
    .bind(lease_seconds)
    .bind(job_id)
    .execute(&mut *tx)
    .await?;

    let claimed = sqlx::query_as::<_, ClaimedJob>(
        "SELECT id, notification_id, channel, attempt_count, max_attempts, lease_token
         FROM delivery_jobs WHERE id = ?",
    )
    .bind(job_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Some(claimed))
}

/// Moves jobs whose lease has expired back to 'queued'.
/// Returns how many jobs were recovered.
pub async fn recover_expired_leases(pool: &MySqlPool) -> Result<u64, DbError> {
    let result = sqlx::query(
        "UPDATE delivery_jobs
         SET status = 'queued',
             locked_by = NULL,
             lease_token = NULL,
             lease_expires_at = NULL
         WHERE status = 'processing'
           AND lease_expires_at IS NOT NULL
           AND lease_expires_at < CURRENT_TIMESTAMP",
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

/// Marks a job as succeeded, but only if the caller still holds the lease.
/// Returns false when the lease token no longer matches (stale worker).
pub async fn complete_job(
    pool: &MySqlPool,
    job_id: u64,
    lease_token: &str,
) -> Result<bool, DbError> {
    let result = sqlx::query(
        "UPDATE delivery_jobs
         SET status = 'succeeded',
             locked_by = NULL,
             lease_token = NULL,
             lease_expires_at = NULL
         WHERE id = ? AND status = 'processing' AND lease_token = ?",
    )
    .bind(job_id)
    .bind(lease_token)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() == 1)
}

/// Backoff: base * 2^(attempt-1), capped at max_secs.
pub fn backoff_seconds(attempt: u32, base_secs: u64, max_secs: u64) -> u64 {
    let exp = attempt.saturating_sub(1).min(20);
    base_secs.saturating_mul(1u64 << exp).min(max_secs)
}

/// Reports a failed delivery. Only works if caller holds the lease.
/// - retryable and attempts left -> back to 'queued' with a future next_attempt_at
/// - otherwise -> 'dead'
/// Returns the new status, or None if the lease was stale.
pub async fn fail_job(
    pool: &MySqlPool,
    job_id: u64,
    lease_token: &str,
    retryable: bool,
    error: &str,
) -> Result<Option<String>, DbError> {
    let mut tx = pool.begin().await?;

    let row: Option<(u32, u32)> = sqlx::query_as(
        "SELECT attempt_count, max_attempts FROM delivery_jobs
         WHERE id = ? AND status = 'processing' AND lease_token = ?
         FOR UPDATE",
    )
    .bind(job_id)
    .bind(lease_token)
    .fetch_optional(&mut *tx)
    .await?;

    let (attempt_count, max_attempts) = match row {
        Some(r) => r,
        None => {
            tx.commit().await?;
            return Ok(None);
        }
    };

    // This failure counts as one more attempt.
    let attempts_used = attempt_count + 1;

    if retryable && attempts_used < max_attempts {
        let delay = backoff_seconds(attempts_used, 30, 3600) as i64;
        sqlx::query(
            "UPDATE delivery_jobs
             SET status = 'queued', attempt_count = ?, last_error = ?,
                 next_attempt_at = DATE_ADD(CURRENT_TIMESTAMP, INTERVAL ? SECOND),
                 locked_by = NULL, lease_token = NULL, lease_expires_at = NULL
             WHERE id = ?",
        )
        .bind(attempts_used)
        .bind(error)
        .bind(delay)
        .bind(job_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some("queued".to_string()))
    } else {
        sqlx::query(
            "UPDATE delivery_jobs
             SET status = 'dead', attempt_count = ?, last_error = ?,
                 locked_by = NULL, lease_token = NULL, lease_expires_at = NULL
             WHERE id = ?",
        )
        .bind(attempts_used)
        .bind(error)
        .bind(job_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some("dead".to_string()))
    }
}
use sqlx::MySqlPool;

use super::model::BatchRecord;
use super::repository;

pub async fn get_batch_status(
    pool: &MySqlPool,
    batch_id: u64,
) -> Result<Option<BatchRecord>, sqlx::Error> {
    repository::get_batch(pool, batch_id).await
}

pub async fn get_batch_notifications(
    pool: &MySqlPool,
    batch_id: u64,
    limit: u32,
    offset: u64,
) -> Result<Vec<super::model::BatchNotification>, sqlx::Error> {
    repository::get_batch_notifications(pool, batch_id, limit, offset).await
}

pub async fn get_delivery_history(
    pool: &MySqlPool,
    notification_id: u64,
) -> Result<Vec<super::model::DeliveryHistoryRecord>, sqlx::Error> {
    repository::get_delivery_history(pool, notification_id).await
}

pub async fn cancel_notification(
    pool: &MySqlPool,
    notification_id: u64,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let result = sqlx::query(
        "UPDATE notifications
         SET status = 'cancelled', updated_at = CURRENT_TIMESTAMP
         WHERE id = ? AND status = 'queued'"
    )
    .bind(notification_id)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(false);
    }

    sqlx::query(
        "UPDATE delivery_jobs
         SET status = 'cancelled', updated_at = CURRENT_TIMESTAMP
         WHERE notification_id = ? AND status = 'queued'"
    )
    .bind(notification_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(true)
}

pub async fn get_notification_status(
    pool: &MySqlPool,
    notification_id: u64,
) -> Result<Option<super::model::BatchNotification>, sqlx::Error> {
    sqlx::query_as::<_, super::model::BatchNotification>(
        "SELECT id, batch_id, recipient, subject, status, created_at, updated_at
         FROM notifications WHERE id = ?"
    )
    .bind(notification_id)
    .fetch_optional(pool)
    .await
}

#[derive(Debug, serde::Deserialize)]
pub struct NewNotificationInput {
    pub recipient: String,
    pub subject: Option<String>,
    pub body: String,
    pub channel: String,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct SubmittedBatch {
    pub batch_id: u64,
    pub notification_ids: Vec<u64>,
    pub status: String,
}

pub async fn submit_batch(
    pool: &MySqlPool,
    application_id: u64,
    notifications: &[NewNotificationInput],
) -> Result<SubmittedBatch, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let batch_result = sqlx::query(
        "INSERT INTO notification_batches (application_id, status, total_count)
         VALUES (?, 'queued', ?)"
    )
    .bind(application_id)
    .bind(notifications.len() as u32)
    .execute(&mut *tx)
    .await?;

    let batch_id = batch_result.last_insert_id();
    let mut notification_ids = Vec::new();

    for item in notifications {
        let result = sqlx::query(
            "INSERT INTO notifications
             (application_id, batch_id, idempotency_key, recipient, subject, body)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(application_id)
        .bind(batch_id)
        .bind(&item.idempotency_key)
        .bind(&item.recipient)
        .bind(&item.subject)
        .bind(&item.body)
        .execute(&mut *tx)
        .await?;

        let notification_id = result.last_insert_id();

        sqlx::query(
            "INSERT INTO delivery_jobs (notification_id, channel, status)
             VALUES (?, ?, 'queued')"
        )
        .bind(notification_id)
        .bind(&item.channel)
        .execute(&mut *tx)
        .await?;

        notification_ids.push(notification_id);
    }

    tx.commit().await?;

    Ok(SubmittedBatch {
        batch_id,
        notification_ids,
        status: "queued".to_string(),
    })
}


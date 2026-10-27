use std::time::Duration;

use sqlx::MySqlPool;

use crate::db::{
    attempts::record_delivery_attempt,
    models::NewDeliveryAttempt,
    notifications::get_notification_by_id,
    queue::{claim_next_job, complete_job, fail_job, recover_expired_leases},
    DbError,
};

use super::{
    config::SmtpConfig,
    job::EmailJob,
    outcome::{DeliveryOutcome, OutcomeStatus},
    sender::deliver,
};

/// attempts table-la save panra "outcome" text.
/// Migration-la allowed values vera irundha, inga mattum maathunga.
fn outcome_label(status: &OutcomeStatus) -> &'static str {
    match status {
        OutcomeStatus::Accepted => "succeeded",
        OutcomeStatus::TemporaryFailure => "temporary_failure",
        OutcomeStatus::PermanentFailure => "permanent_failure",
    }
}

fn to_attempt(job_id: u64, outcome: &DeliveryOutcome) -> NewDeliveryAttempt {
    let failed = outcome.status != OutcomeStatus::Accepted;
    NewDeliveryAttempt {
        job_id,
        outcome: outcome_label(&outcome.status).to_string(),
        error_code: if failed { outcome.smtp_code.clone() } else { None },
        error_message: if failed { Some(outcome.message.clone()) } else { None },
    }
}

/// Oru job-a process pannum. Job kidaichaa Ok(true), queue empty na Ok(false).
pub async fn process_one(
    pool: &MySqlPool,
    cfg: &SmtpConfig,
    worker_id: &str,
    lease_seconds: i64,
) -> Result<bool, DbError> {
    // 1. Job claim pannu
    let job = match claim_next_job(pool, worker_id, lease_seconds).await? {
        Some(j) => j,
        None => return Ok(false),
    };

    // Email illaadha job-a thodaadhe. Lease expire aanadhum thirumba queue-ku pogum.
    // TODO: Dev 2 claim_next_job-la channel filter serthaa idhu thevai illa.
    if job.channel != "email" {
        eprintln!("job {} is '{}', not email. Skipping.", job.id, job.channel);
        return Ok(true);
    }

    let token = match job.lease_token.clone() {
        Some(t) => t,
        None => {
            eprintln!("job {} has no lease token", job.id);
            return Ok(true);
        }
    };

    // 2. Notification-a eduthukko (recipient, subject, body)
    let notification = match get_notification_by_id(pool, job.notification_id).await {
        Ok(n) => n,
        Err(DbError::NotFound) => {
            fail_job(pool, job.id, &token, false, "notification not found").await?;
            return Ok(true);
        }
        Err(e) => return Err(e),
    };

    // 3. EmailJob build pannu
    let email = EmailJob {
        notification_id: job.notification_id,
        to: notification.recipient,
        subject: notification
            .subject
            .unwrap_or_else(|| "(no subject)".to_string()),
        body: notification.body,
        attempt_count: job.attempt_count,
    };

    // 4. Email anuppu
    let outcome = deliver(cfg, &email).await;

    // 5. Attempt-a MUDHALLA save pannu (fail_job attempt_count-a maathum)
    record_delivery_attempt(pool, &to_attempt(job.id, &outcome)).await?;

    // 6. Job status update pannu
    match outcome.status {
        OutcomeStatus::Accepted => {
            if !complete_job(pool, job.id, &token).await? {
                eprintln!("job {}: lease lost before complete", job.id);
            }
        }
        OutcomeStatus::TemporaryFailure => {
            fail_job(pool, job.id, &token, true, &outcome.message).await?;
        }
        OutcomeStatus::PermanentFailure => {
            fail_job(pool, job.id, &token, false, &outcome.message).await?;
        }
    }

    println!("job {} -> {:?}", job.id, outcome.status);
    Ok(true)
}

/// Eppavum odum worker loop
pub async fn run_worker(pool: &MySqlPool, cfg: &SmtpConfig, worker_id: &str) {
    // Lease, namma max SMTP wait-a vida perusa irukkanum (double email thavirkka)
    let lease_seconds = cfg.timeout_secs as i64 + 90;

    loop {
        match process_one(pool, cfg, worker_id, lease_seconds).await {
            Ok(true) => {}
            Ok(false) => {
                // Job illa: crash aana jobs-a recover pannitu konjam thoongu
                if let Err(e) = recover_expired_leases(pool).await {
                    eprintln!("recover error: {e}");
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(e) => {
                eprintln!("worker error: {e}");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
mod db;
mod email;

use db::models::NewDeliveryAttempt;
use std::env;

#[tokio::main]
async fn main() -> Result<(), db::DbError> {
    dotenvy::dotenv().ok();
    let url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let pool = db::connect(&url).await?;

    // Claim until we get job 3 (max_attempts = 2).
    let mut parked = Vec::new();
    let job = loop {
        let j = db::queue::claim_next_job(&pool, "worker-X", 60)
            .await?
            .expect("job 3 not found");
        if j.id == 3 {
            break j;
        }
        parked.push(j.id);
    };
    println!("claimed job 3 (parked jobs: {:?})", parked);
    let tok = job.lease_token.clone().unwrap();

    for round in 1..=2 {
        let attempt = NewDeliveryAttempt {
            job_id: job.id,
            outcome: "retryable_error".to_string(),
            error_code: Some("TIMEOUT".to_string()),
            error_message: Some(format!("timeout {}", round)),
        };
        let attempt_id = db::attempts::record_delivery_attempt(&pool, &attempt).await?;
        println!("round {}: recorded attempt id {}", round, attempt_id);

        let s = db::queue::fail_job(&pool, job.id, &tok, true, "timeout").await?;
        println!("round {}: fail_job -> {:?}", round, s);

        if round == 1 {
            // Bring job 3 back to eligible and claim it again.
            sqlx::query("UPDATE delivery_jobs SET next_attempt_at = CURRENT_TIMESTAMP WHERE id = 3")
                .execute(&pool)
                .await?;
            let again = loop {
                let j = db::queue::claim_next_job(&pool, "worker-X", 60)
                    .await?
                    .expect("job 3 should be claimable again");
                if j.id == 3 {
                    break j;
                }
            };
            let _ = again; // new token is read from DB below
            let new_tok: (String,) = sqlx::query_as(
                "SELECT lease_token FROM delivery_jobs WHERE id = 3",
            )
            .fetch_one(&pool)
            .await?;
            // shadow token for round 2
            return finish_round2(&pool, new_tok.0).await;
        }
    }
    Ok(())
}

async fn finish_round2(pool: &sqlx::MySqlPool, tok: String) -> Result<(), db::DbError> {
    let attempt = NewDeliveryAttempt {
        job_id: 3,
        outcome: "retryable_error".to_string(),
        error_code: Some("TIMEOUT".to_string()),
        error_message: Some("timeout 2".to_string()),
    };
    let attempt_id = db::attempts::record_delivery_attempt(pool, &attempt).await?;
    println!("round 2: recorded attempt id {}", attempt_id);

    let s = db::queue::fail_job(pool, 3, &tok, true, "timeout 2").await?;
    println!("round 2: fail_job -> {:?} (expected Some(\"dead\"))", s);
    Ok(())
}
#[path = "../db/mod.rs"]
mod db;

use db::models::{NewDeliveryJob, NewNotification};
use std::env;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let pool = db::connect(&url).await.expect("cannot connect to database");

    // Ethavadhu oru application venum (notifications table-ku foreign key)
    let app: Option<(u64,)> = sqlx::query_as("SELECT id FROM applications LIMIT 1")
        .fetch_optional(&pool)
        .await
        .expect("cannot read applications");

    let app_id = match app {
        Some((id,)) => id,
        None => {
            println!("applications table empty. Mudhalla oru application row insert pannunga.");
            return;
        }
    };

    // (recipient, subject, body)
    let cases: Vec<(&str, Option<&str>, &str)> = vec![
        ("student@example.com", Some("Hedwig test 1"), "Hello from the Hedwig email worker!"),
        ("not-an-email", Some("Hedwig test 2 (invalid address)"), "This one must fail permanently."),
        ("student2@example.com", None, "This notification has no subject."),
    ];

    for (to, subject, body) in cases {
        let notification_id = db::notifications::create_notification(
            &pool,
            &NewNotification {
                application_id: app_id,
                batch_id: None,
                idempotency_key: None,
                recipient: to.to_string(),
                subject: subject.map(|s| s.to_string()),
                body: body.to_string(),
            },
        )
        .await
        .expect("create_notification failed");

        let job_id = db::jobs::create_delivery_job(
            &pool,
            &NewDeliveryJob {
                notification_id,
                channel: "email".to_string(),
                max_attempts: 3,
            },
        )
        .await
        .expect("create_delivery_job failed");

        println!("created notification {notification_id} -> email job {job_id} ({to})");
    }
}
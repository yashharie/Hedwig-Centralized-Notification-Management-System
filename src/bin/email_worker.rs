#[path = "../db/mod.rs"]
mod db;
#[path = "../email/mod.rs"]
mod email;

use std::env;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let pool = db::connect(&url).await.expect("cannot connect to database");

    let cfg = email::config::SmtpConfig::from_env();
    println!("Email worker started (SMTP {}:{})", cfg.host, cfg.port);

    email::worker::run_worker(&pool, &cfg, "email-worker-1").await;
}
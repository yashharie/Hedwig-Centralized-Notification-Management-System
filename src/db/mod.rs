pub mod attempts;
pub mod batches;
pub mod jobs;
pub mod models;
pub mod notifications;
pub mod queue;

use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("record not found")]
    NotFound,
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

pub async fn connect(database_url: &str) -> Result<MySqlPool, DbError> {
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    Ok(pool)
}
#[derive(Debug, Clone)]
pub struct EmailJob {
    pub notification_id: u64,
    pub to: String,
    pub subject: String,
    pub body: String,
    pub attempt_count: u32,
}
use lettre::{
    message::{header::ContentType, Mailbox},
    Message,
};

use super::{config::SmtpConfig, error::EmailError, job::EmailJob};

pub fn build_message(cfg: &SmtpConfig, job: &EmailJob) -> Result<Message, EmailError> {
    let from: Mailbox = cfg
        .from
        .parse()
        .map_err(|e| EmailError::Permanent(format!("invalid from address: {e}")))?;

    let to: Mailbox = job
        .to
        .parse()
        .map_err(|e| EmailError::Permanent(format!("invalid recipient: {e}")))?;

    Message::builder()
        .from(from)
        .to(to)
        .subject(job.subject.clone())
        .header(ContentType::TEXT_PLAIN)
        .body(job.body.clone())
        .map_err(|e| EmailError::Permanent(format!("cannot build message: {e}")))
}
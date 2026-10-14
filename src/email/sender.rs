use lettre::{
    transport::smtp::{authentication::Credentials, Error as SmtpError},
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};

use super::{
    config::SmtpConfig, error::EmailError, job::EmailJob, message::build_message,
};

fn build_mailer(cfg: &SmtpConfig) -> Result<AsyncSmtpTransport<Tokio1Executor>, EmailError> {
    let builder = if cfg.use_tls {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)
            .map_err(|e| EmailError::Permanent(format!("bad SMTP config: {e}")))?
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.host)
    };

    let mut builder = builder.port(cfg.port);

    if let (Some(user), Some(pass)) = (&cfg.username, &cfg.password) {
        builder = builder.credentials(Credentials::new(user.clone(), pass.clone()));
    }

    Ok(builder.build())
}

fn classify(err: SmtpError) -> EmailError {
    if err.is_permanent() {
        EmailError::Permanent(err.to_string())
    } else {
        EmailError::Temporary(err.to_string())
    }
}

pub async fn send_email(cfg: &SmtpConfig, job: &EmailJob) -> Result<(), EmailError> {
    let message = build_message(cfg, job)?;
    let mailer = build_mailer(cfg)?;

    mailer.send(message).await.map(|_| ()).map_err(classify)
}
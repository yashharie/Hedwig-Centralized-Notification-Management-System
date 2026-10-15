use std::time::Duration;

use lettre::{
    transport::smtp::{authentication::Credentials, Error as SmtpError},
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};

use super::{
    config::SmtpConfig, error::EmailError, job::EmailJob, message::build_message,
    outcome::DeliveryOutcome,
};

fn build_mailer(cfg: &SmtpConfig) -> Result<AsyncSmtpTransport<Tokio1Executor>, EmailError> {
    let builder = if cfg.use_tls {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&cfg.host)
            .map_err(|e| EmailError::Permanent(format!("bad SMTP config: {e}")))?
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.host)
    };

    let mut builder = builder
        .port(cfg.port)
        .timeout(Some(Duration::from_secs(cfg.timeout_secs)));

    if let (Some(user), Some(pass)) = (&cfg.username, &cfg.password) {
        builder = builder.credentials(Credentials::new(user.clone(), pass.clone()));
    }

    Ok(builder.build())
}

fn classify(err: SmtpError) -> EmailError {
    if err.is_permanent() {
        EmailError::Permanent(err.to_string())
    } else {
        // transient, timeout, connection errors ellam retry pannalam
        EmailError::Temporary(err.to_string())
    }
}

/// Oru email attempt pannum, epavume DeliveryOutcome return pannum (panic / Err illa)
pub async fn deliver(cfg: &SmtpConfig, job: &EmailJob) -> DeliveryOutcome {
    let attempt_number = job.attempt_count + 1;

    let message = match build_message(cfg, job) {
        Ok(m) => m,
        Err(e) => return DeliveryOutcome::failed(job, attempt_number, &e, None),
    };

    let mailer = match build_mailer(cfg) {
        Ok(m) => m,
        Err(e) => return DeliveryOutcome::failed(job, attempt_number, &e, None),
    };

    // Safety net: Lettre timeout-ku mela, namma own overall timeout
    let overall = Duration::from_secs(cfg.timeout_secs + 5);

    match tokio::time::timeout(overall, mailer.send(message)).await {
        Err(_) => {
            let err = EmailError::Temporary(format!(
                "SMTP request timed out after {} seconds",
                overall.as_secs()
            ));
            DeliveryOutcome::failed(job, attempt_number, &err, None)
        }
        Ok(Ok(response)) => {
            let code = response.code().to_string();
            let text = response.message().collect::<Vec<_>>().join(" ");
            DeliveryOutcome::accepted(job, attempt_number, code, text)
        }
        Ok(Err(e)) => {
            let code = e.status().map(|c| c.to_string());
            let err = classify(e);
            DeliveryOutcome::failed(job, attempt_number, &err, code)
        }
    }
}
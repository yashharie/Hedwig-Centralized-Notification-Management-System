use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use super::{
    config::SmtpConfig,
    error::EmailError,
    job::EmailJob,
    message::build_message,
    outcome::{DeliveryOutcome, OutcomeStatus},
    sender::deliver,
};

// ---------- Helpers ----------

fn test_cfg(port: u16, timeout_secs: u64) -> SmtpConfig {
    SmtpConfig {
        host: "127.0.0.1".into(),
        port,
        username: None,
        password: None,
        from: "Hedwig <hedwig@example.com>".into(),
        use_tls: false,
        timeout_secs,
    }
}

fn test_job(to: &str) -> EmailJob {
    EmailJob {
        notification_id: 1,
        to: to.into(),
        subject: "Test subject".into(),
        body: "Test body".into(),
        attempt_count: 0,
    }
}


async fn start_fake_smtp(rcpt_reply: &'static str) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => return,
            };
            tokio::spawn(handle_client(stream, rcpt_reply));
        }
    });
    port
}

async fn handle_client(stream: TcpStream, rcpt_reply: &'static str) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let _ = writer.write_all(b"220 fake.smtp ready\r\n").await;

    let mut line = String::new();
    let mut in_data = false;

    loop {
        line.clear();
        if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
            return;
        }

        if in_data {
            if line == ".\r\n" {
                in_data = false;
                let _ = writer.write_all(b"250 queued\r\n").await;
            }
            continue;
        }

        let cmd = line.trim().to_uppercase();
        let reply = if cmd.starts_with("EHLO") || cmd.starts_with("HELO") {
            "250 fake.smtp\r\n".to_string()
        } else if cmd.starts_with("MAIL") {
            "250 OK\r\n".to_string()
        } else if cmd.starts_with("RCPT") {
            format!("{rcpt_reply}\r\n")
        } else if cmd == "DATA" {
            in_data = true;
            "354 go ahead\r\n".to_string()
        } else if cmd.starts_with("QUIT") {
            let _ = writer.write_all(b"221 bye\r\n").await;
            return;
        } else {
            "250 OK\r\n".to_string()
        };
        let _ = writer.write_all(reply.as_bytes()).await;
    }
}

/// Connection accept pannum, aana edhuvum respond pannaadhu (timeout test)
async fn start_silent_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            tokio::time::sleep(Duration::from_secs(30)).await;
            drop(stream);
        }
    });
    port
}

// ---------- Error type tests ----------

#[test]
fn temporary_errors_should_retry() {
    assert!(EmailError::Temporary("busy".into()).should_retry());
}

#[test]
fn permanent_errors_should_not_retry() {
    assert!(!EmailError::Permanent("bad address".into()).should_retry());
}

// ---------- Message building tests ----------

#[test]
fn builds_valid_message() {
    let cfg = test_cfg(1025, 5);
    assert!(build_message(&cfg, &test_job("student@example.com")).is_ok());
}

#[test]
fn invalid_recipient_is_permanent_error() {
    let cfg = test_cfg(1025, 5);
    let err = build_message(&cfg, &test_job("not-an-email")).unwrap_err();
    assert!(!err.should_retry());
}

#[test]
fn invalid_from_address_is_permanent_error() {
    let mut cfg = test_cfg(1025, 5);
    cfg.from = "not-an-email".into();
    let err = build_message(&cfg, &test_job("student@example.com")).unwrap_err();
    assert!(!err.should_retry());
}

// ---------- Outcome tests ----------

#[test]
fn failed_outcome_maps_error_type_to_status() {
    let job = test_job("student@example.com");

    let temp = DeliveryOutcome::failed(&job, 1, &EmailError::Temporary("x".into()), None);
    assert_eq!(temp.status, OutcomeStatus::TemporaryFailure);

    let perm = DeliveryOutcome::failed(&job, 1, &EmailError::Permanent("x".into()), None);
    assert_eq!(perm.status, OutcomeStatus::PermanentFailure);
}

#[test]
fn accepted_outcome_keeps_smtp_code() {
    let job = test_job("student@example.com");
    let ok = DeliveryOutcome::accepted(&job, 2, "250".into(), "queued".into());
    assert_eq!(ok.status, OutcomeStatus::Accepted);
    assert_eq!(ok.attempt_number, 2);
    assert_eq!(ok.smtp_code.as_deref(), Some("250"));
}

// ---------- SMTP delivery tests (fake server) ----------

#[tokio::test]
async fn accepted_when_server_says_250() {
    let port = start_fake_smtp("250 OK").await;
    let outcome = deliver(&test_cfg(port, 5), &test_job("student@example.com")).await;
    assert_eq!(outcome.status, OutcomeStatus::Accepted);
    assert_eq!(outcome.attempt_number, 1);
}

#[tokio::test]
async fn permanent_failure_when_server_says_550() {
    let port = start_fake_smtp("550 no such user").await;
    let outcome = deliver(&test_cfg(port, 5), &test_job("nobody@example.com")).await;
    assert_eq!(outcome.status, OutcomeStatus::PermanentFailure);
}

#[tokio::test]
async fn temporary_failure_when_server_says_451() {
    let port = start_fake_smtp("451 try again later").await;
    let outcome = deliver(&test_cfg(port, 5), &test_job("student@example.com")).await;
    assert_eq!(outcome.status, OutcomeStatus::TemporaryFailure);
}

#[tokio::test]
async fn temporary_failure_when_connection_refused() {
    // Oru port eduthu udane close pannidrom, so yaarum listen pannala
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let outcome = deliver(&test_cfg(port, 5), &test_job("student@example.com")).await;
    assert_eq!(outcome.status, OutcomeStatus::TemporaryFailure);
}

#[tokio::test]
async fn temporary_failure_when_server_never_responds() {
    let port = start_silent_server().await;
    let outcome = deliver(&test_cfg(port, 1), &test_job("student@example.com")).await;
    assert_eq!(outcome.status, OutcomeStatus::TemporaryFailure);
}

#[tokio::test]
async fn invalid_recipient_fails_without_contacting_server() {
   
    let outcome = deliver(&test_cfg(1, 5), &test_job("not-an-email")).await;
    assert_eq!(outcome.status, OutcomeStatus::PermanentFailure);
}
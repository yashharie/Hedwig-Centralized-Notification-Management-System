use crate::client::{ClientConfig, build_client_with_resolver, build_production_client};
use crate::sender::deliver;
use crate::types::{FailureKind, WebhookOutcome};
use reqwest::Client;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// TEST ONLY: resolver without the SSRF check, so loopback mock servers work.
struct PassResolver;

async fn lookup(host: String) -> Result<Vec<SocketAddr>, BoxError> {
    Ok(tokio::net::lookup_host((host.as_str(), 0)).await?.collect())
}

impl Resolve for PassResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move { lookup(host).await.map(|v| Box::new(v.into_iter()) as Addrs) })
    }
}

fn test_client(total_ms: u64) -> Client {
    let cfg = ClientConfig {
        connect_timeout: Duration::from_millis(500),
        total_timeout: Duration::from_millis(total_ms),
    };
    build_client_with_resolver(&cfg, Arc::new(PassResolver)).unwrap()
}

fn http_response(status_line: &str, extra_headers: &str) -> String {
    format!(
        "HTTP/1.1 {status_line}\r\n{extra_headers}Content-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

fn header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

async fn read_request(sock: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        let n = match sock.read(&mut tmp).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = header_end(&buf) {
            let head = String::from_utf8_lossy(&buf[..pos]).to_lowercase();
            let len = head
                .lines()
                .find_map(|l| l.strip_prefix("content-length:"))
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if buf.len() >= pos + 4 + len {
                break;
            }
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

/// Mock server. `Some(resp)` replies with resp; `None` never replies (timeout test).
async fn start_server(response: Option<String>) -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_srv = seen.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let seen = seen_srv.clone();
            let response = response.clone();
            tokio::spawn(async move {
                let req = read_request(&mut sock).await;
                seen.lock().unwrap().push(req);
                match response {
                    Some(r) => {
                        let _ = sock.write_all(r.as_bytes()).await;
                        let _ = sock.shutdown().await;
                    }
                    None => std::future::pending::<()>().await,
                }
            });
        }
    });
    (port, seen)
}

fn local_url(port: u16) -> Url {
    Url::parse(&format!("http://127.0.0.1:{port}/hook")).unwrap()
}

#[tokio::test]
async fn success_200_is_delivered() {
    let (port, _) = start_server(Some(http_response("200 OK", ""))).await;
    let out = deliver(&test_client(2000), &local_url(port), &serde_json::json!({})).await;
    assert!(matches!(out, WebhookOutcome::Delivered { status: 200 }));
}

#[tokio::test]
async fn status_404_is_draft_not_retryable() {
    let (port, _) = start_server(Some(http_response("404 Not Found", ""))).await;
    let out = deliver(&test_client(2000), &local_url(port), &serde_json::json!({})).await;
    assert!(matches!(
        out,
        WebhookOutcome::Failed {
            kind: FailureKind::HttpStatus,
            status: Some(404),
            retryable: false,
            ..
        }
    ));
}

#[tokio::test]
async fn status_429_keeps_retry_after() {
    let resp = http_response("429 Too Many Requests", "Retry-After: 7\r\n");
    let (port, _) = start_server(Some(resp)).await;
    let out = deliver(&test_client(2000), &local_url(port), &serde_json::json!({})).await;
    assert!(matches!(
        out,
        WebhookOutcome::Failed {
            status: Some(429),
            retryable: true,
            retry_after_secs: Some(7),
            ..
        }
    ));
}

#[tokio::test]
async fn status_503_is_draft_retryable() {
    let (port, _) = start_server(Some(http_response("503 Service Unavailable", ""))).await;
    let out = deliver(&test_client(2000), &local_url(port), &serde_json::json!({})).await;
    assert!(matches!(
        out,
        WebhookOutcome::Failed {
            status: Some(503),
            retryable: true,
            ..
        }
    ));
}

#[tokio::test]
async fn redirect_is_not_followed() {
    let resp = http_response("302 Found", "Location: http://127.0.0.1:1/\r\n");
    let (port, _) = start_server(Some(resp)).await;
    let out = deliver(&test_client(2000), &local_url(port), &serde_json::json!({})).await;
    assert!(matches!(
        out,
        WebhookOutcome::Failed {
            kind: FailureKind::HttpStatus,
            status: Some(302),
            retryable: false,
            ..
        }
    ));
}

#[tokio::test]
async fn slow_server_times_out() {
    let (port, _) = start_server(None).await;
    let out = deliver(&test_client(200), &local_url(port), &serde_json::json!({})).await;
    assert!(matches!(
        out,
        WebhookOutcome::Failed {
            kind: FailureKind::Timeout,
            retryable: true,
            ..
        }
    ));
}

#[tokio::test]
async fn closed_port_is_connection_failure() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let out = deliver(&test_client(2000), &local_url(port), &serde_json::json!({})).await;
    assert!(matches!(
        out,
        WebhookOutcome::Failed {
            kind: FailureKind::Connection | FailureKind::Timeout,
            retryable: true,
            ..
        }
    ));
}

#[tokio::test]
async fn sends_json_body_and_content_type() {
    let (port, seen) = start_server(Some(http_response("200 OK", ""))).await;
    let payload = serde_json::json!({"hello": "world"});
    let out = deliver(&test_client(2000), &local_url(port), &payload).await;
    assert!(matches!(out, WebhookOutcome::Delivered { status: 200 }));
    let raw = seen.lock().unwrap()[0].clone();
    assert!(
        raw.to_lowercase()
            .contains("content-type: application/json")
    );
    assert!(raw.ends_with("{\"hello\":\"world\"}"));
}

#[tokio::test]
async fn safe_resolver_blocks_localhost() {
    let client = build_production_client(&ClientConfig::default()).unwrap();
    let url = Url::parse("http://localhost:9/").unwrap();
    let out = deliver(&client, &url, &serde_json::json!({})).await;
    assert!(matches!(
        out,
        WebhookOutcome::Failed {
            kind: FailureKind::BlockedDestination,
            retryable: false,
            ..
        }
    ));
}

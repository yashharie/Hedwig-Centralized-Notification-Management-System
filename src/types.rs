use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Input from the queue/worker layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookRequest {
    pub job_id: u64,
    pub url: String,
    pub payload: Value,
}

/// Why a webhook delivery failed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureKind {
    InvalidUrl,
    BlockedDestination,
    Timeout,
    Connection,
    HttpStatus,
}

/// Structured result returned to the queue/worker layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookOutcome {
    Delivered {
        status: u16,
    },
    Failed {
        kind: FailureKind,
        status: Option<u16>,
        retryable: bool,
        retry_after_secs: Option<u64>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn can_build_request_and_outcome() {
        let req = WebhookRequest {
            job_id: 1,
            url: "https://example.com/hook".to_string(),
            payload: json!({ "message": "hello" }),
        };
        assert_eq!(req.job_id, 1);

        let ok = WebhookOutcome::Delivered { status: 200 };
        assert_eq!(ok, WebhookOutcome::Delivered { status: 200 });
    }
}

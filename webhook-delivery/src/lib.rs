pub mod client;
pub mod host_policy;
pub mod ip_policy;
pub mod resolver;
pub mod sender;
pub mod types;
pub mod validate;

#[cfg(test)]
mod mock_tests;

// Convenience re-exports so downstream crates can use the crate root.
pub use client::{ClientConfig, build_production_client};
pub use sender::{RetryConfig, send_webhook, send_webhook_with_retry};
pub use types::{FailureKind, WebhookOutcome, WebhookRequest};

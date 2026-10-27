use crate::resolver::SafeResolver;
use reqwest::{Client, dns::Resolve, redirect};
use std::sync::Arc;
use std::time::Duration;

/// Configuration for the webhook HTTP client.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Maximum time to wait for a TCP connection to be established.
    pub connect_timeout: Duration,
    /// Maximum total time for the entire request (connection + transfer).
    pub total_timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            total_timeout: Duration::from_secs(30),
        }
    }
}

/// Resolver is injectable so tests can use a loopback-friendly resolver.
/// Production code must go through `build_production_client`.
pub fn build_client_with_resolver<R: Resolve + 'static>(
    cfg: &ClientConfig,
    resolver: Arc<R>,
) -> Result<Client, reqwest::Error> {
    Client::builder()
        .dns_resolver(resolver)
        .redirect(redirect::Policy::none())
        .no_proxy()
        .connect_timeout(cfg.connect_timeout)
        .timeout(cfg.total_timeout)
        .build()
}

/// Production client: always uses the SSRF-safe resolver.
pub fn build_production_client(cfg: &ClientConfig) -> Result<Client, reqwest::Error> {
    build_client_with_resolver(cfg, Arc::new(SafeResolver))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_client_builds() {
        assert!(build_production_client(&ClientConfig::default()).is_ok());
    }

    #[test]
    fn default_config_has_expected_timeouts() {
        let cfg = ClientConfig::default();
        assert_eq!(cfg.connect_timeout, Duration::from_secs(5));
        assert_eq!(cfg.total_timeout, Duration::from_secs(30));
    }
}

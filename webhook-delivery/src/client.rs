use crate::resolver::SafeResolver;
use reqwest::{Client, dns::Resolve, redirect};
use std::sync::Arc;
use std::time::Duration;

/// PLACEHOLDER values. Not agreed with the team yet.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub connect_timeout: Duration,
    pub total_timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            // PLACEHOLDER: confirm with the team
            connect_timeout: Duration::from_secs(5),
            // PLACEHOLDER: confirm with the team
            total_timeout: Duration::from_secs(10),
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
}

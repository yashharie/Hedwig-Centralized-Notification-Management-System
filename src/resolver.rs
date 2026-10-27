use crate::ip_policy::is_blocked_ip;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::net::SocketAddr;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Pure check: every resolved IP must be allowed, otherwise reject all.
pub fn validate_resolved(addrs: Vec<SocketAddr>) -> Result<Vec<SocketAddr>, BoxError> {
    if addrs.is_empty() {
        return Err("no addresses resolved".into());
    }
    if addrs.iter().any(|a| is_blocked_ip(a.ip())) {
        return Err("blocked destination".into());
    }
    Ok(addrs)
}

async fn resolve_checked(host: String) -> Result<Vec<SocketAddr>, BoxError> {
    let found: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
    validate_resolved(found)
}

/// DNS resolver that refuses to hand reqwest any blocked IP.
pub struct SafeResolver;

impl Resolve for SafeResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            resolve_checked(host)
                .await
                .map(|v| Box::new(v.into_iter()) as Addrs)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sock(ip: &str) -> SocketAddr {
        format!("{ip}:0").parse().unwrap()
    }

    #[test]
    fn allows_public_ip() {
        assert!(validate_resolved(vec![sock("8.8.8.8")]).is_ok());
    }

    #[test]
    fn rejects_loopback() {
        assert!(validate_resolved(vec![sock("127.0.0.1")]).is_err());
    }

    #[test]
    fn rejects_private() {
        assert!(validate_resolved(vec![sock("10.0.0.1")]).is_err());
    }

    #[test]
    fn rejects_if_any_ip_is_blocked() {
        assert!(validate_resolved(vec![sock("8.8.8.8"), sock("169.254.169.254")]).is_err());
    }

    #[test]
    fn rejects_empty() {
        assert!(validate_resolved(vec![]).is_err());
    }
}

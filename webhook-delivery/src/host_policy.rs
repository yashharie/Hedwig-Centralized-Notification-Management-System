use std::net::IpAddr;

use url::{Host, Url};

use crate::ip_policy::is_blocked_ip;

/// Why a webhook host was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostError {
    BlockedIp,
    BlockedHostname,
}

/// Checks the host part of an already parsed URL.
/// IP literals are checked with the IP policy; domain names are checked against
/// a small list of internal names (DRAFT policy: to be confirmed with the team).
/// Domain names that RESOLVE to blocked IPs must be handled at connection time.
pub fn check_host(url: &Url) -> Result<(), HostError> {
    match url.host() {
        Some(Host::Ipv4(ip)) => check_ip(IpAddr::V4(ip)),
        Some(Host::Ipv6(ip)) => check_ip(IpAddr::V6(ip)),
        Some(Host::Domain(name)) => check_domain(name),
        None => Err(HostError::BlockedHostname),
    }
}

fn check_ip(ip: IpAddr) -> Result<(), HostError> {
    if is_blocked_ip(ip) {
        Err(HostError::BlockedIp)
    } else {
        Ok(())
    }
}

fn check_domain(name: &str) -> Result<(), HostError> {
    let name = name.trim_end_matches('.').to_ascii_lowercase();

    let blocked = name == "localhost"
        || name.ends_with(".localhost")
        || name.ends_with(".local")
        || name.ends_with(".internal");

    if blocked {
        Err(HostError::BlockedHostname)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::parse_webhook_url;

    fn check(s: &str) -> Result<(), HostError> {
        let url = parse_webhook_url(s).unwrap();
        check_host(&url)
    }

    #[test]
    fn blocks_ipv4_literals() {
        assert_eq!(check("https://127.0.0.1/hook"), Err(HostError::BlockedIp));
        assert_eq!(check("https://10.0.0.1/hook"), Err(HostError::BlockedIp));
        assert_eq!(check("https://192.168.1.1/hook"), Err(HostError::BlockedIp));
        assert_eq!(
            check("https://169.254.169.254/latest/meta-data"),
            Err(HostError::BlockedIp)
        );
    }

    #[test]
    fn blocks_encoded_ipv4_literals() {
        assert_eq!(check("https://2130706433/hook"), Err(HostError::BlockedIp));
        assert_eq!(check("https://0x7f.0.0.1/hook"), Err(HostError::BlockedIp));
    }

    #[test]
    fn blocks_ipv6_literals() {
        assert_eq!(check("https://[::1]/hook"), Err(HostError::BlockedIp));
        assert_eq!(
            check("https://[::ffff:127.0.0.1]/hook"),
            Err(HostError::BlockedIp)
        );
        assert_eq!(check("https://[fe80::1]/hook"), Err(HostError::BlockedIp));
    }

    #[test]
    fn blocks_localhost_names() {
        assert_eq!(
            check("https://localhost/hook"),
            Err(HostError::BlockedHostname)
        );
        assert_eq!(
            check("https://LOCALHOST./hook"),
            Err(HostError::BlockedHostname)
        );
        assert_eq!(
            check("https://api.localhost/hook"),
            Err(HostError::BlockedHostname)
        );
    }

    #[test]
    fn blocks_internal_suffixes() {
        assert_eq!(
            check("https://printer.local/hook"),
            Err(HostError::BlockedHostname)
        );
        assert_eq!(
            check("https://db.internal/hook"),
            Err(HostError::BlockedHostname)
        );
    }

    #[test]
    fn allows_public_hosts() {
        assert_eq!(check("https://example.com/hook"), Ok(()));
        assert_eq!(check("https://8.8.8.8/hook"), Ok(()));
    }
}

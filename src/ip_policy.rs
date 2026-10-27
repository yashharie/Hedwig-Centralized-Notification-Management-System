use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Returns true if the IP address must NOT be contacted by webhook delivery.
pub fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(v4),
        IpAddr::V6(v6) => is_blocked_ipv6(v6),
    }
}

fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();

    a == 0                                   // 0.0.0.0/8 "this network"
        || a == 10                           // 10.0.0.0/8 private
        || (a == 100 && (b & 0xC0) == 64)    // 100.64.0.0/10 shared address space
        || a == 127                          // 127.0.0.0/8 loopback
        || (a == 169 && b == 254)            // 169.254.0.0/16 link-local (cloud metadata)
        || (a == 172 && (b & 0xF0) == 16)    // 172.16.0.0/12 private
        || (a == 192 && b == 0 && c == 0)    // 192.0.0.0/24 protocol assignments
        || (a == 192 && b == 0 && c == 2)    // 192.0.2.0/24 documentation
        || (a == 192 && b == 168)            // 192.168.0.0/16 private
        || (a == 198 && (b & 0xFE) == 18)    // 198.18.0.0/15 benchmarking
        || (a == 198 && b == 51 && c == 100) // 198.51.100.0/24 documentation
        || (a == 203 && b == 0 && c == 113)  // 203.0.113.0/24 documentation
        || a >= 224 // multicast, reserved, broadcast
}

fn is_blocked_ipv6(ip: Ipv6Addr) -> bool {
    // IPv4-mapped addresses (::ffff:a.b.c.d) must follow the IPv4 rules.
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_blocked_ipv4(v4);
    }

    let s = ip.segments();

    ip.is_unspecified()                       // ::
        || ip.is_loopback()                   // ::1
        || ip.is_multicast()                  // ff00::/8
        || (s[0] & 0xffc0) == 0xfe80          // fe80::/10 link-local
        || (s[0] & 0xfe00) == 0xfc00          // fc00::/7 unique local
        || (s[0] == 0x2001 && s[1] == 0x0db8) // 2001:db8::/32 documentation
        || (s[0] == 0x0064 && s[1] == 0xff9b) // 64:ff9b::/32 NAT64 (conservative)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocked(s: &str) -> bool {
        is_blocked_ip(s.parse().unwrap())
    }

    #[test]
    fn blocks_loopback_v4() {
        assert!(blocked("127.0.0.1"));
        assert!(blocked("127.255.255.254"));
    }

    #[test]
    fn blocks_private_v4() {
        assert!(blocked("10.0.0.1"));
        assert!(blocked("172.16.0.1"));
        assert!(blocked("172.31.255.255"));
        assert!(blocked("192.168.1.10"));
        assert!(!blocked("172.15.255.255"));
        assert!(!blocked("172.32.0.1"));
    }

    #[test]
    fn blocks_link_local_and_metadata() {
        assert!(blocked("169.254.0.1"));
        assert!(blocked("169.254.169.254"));
    }

    #[test]
    fn blocks_unspecified_and_shared() {
        assert!(blocked("0.0.0.0"));
        assert!(blocked("100.64.0.1"));
        assert!(blocked("100.127.255.255"));
        assert!(!blocked("100.63.255.255"));
        assert!(!blocked("100.128.0.1"));
    }

    #[test]
    fn blocks_multicast_and_reserved() {
        assert!(blocked("224.0.0.1"));
        assert!(blocked("255.255.255.255"));
    }

    #[test]
    fn allows_public_v4() {
        assert!(!blocked("8.8.8.8"));
        assert!(!blocked("1.1.1.1"));
    }

    #[test]
    fn blocks_special_v6() {
        assert!(blocked("::"));
        assert!(blocked("::1"));
        assert!(blocked("fe80::1"));
        assert!(blocked("fc00::1"));
        assert!(blocked("fd12:3456::1"));
        assert!(blocked("ff02::1"));
        assert!(blocked("2001:db8::1"));
    }

    #[test]
    fn blocks_v4_mapped_v6_by_inner_address() {
        assert!(blocked("::ffff:127.0.0.1"));
        assert!(blocked("::ffff:10.0.0.1"));
        assert!(blocked("::ffff:169.254.169.254"));
        assert!(!blocked("::ffff:8.8.8.8"));
    }

    #[test]
    fn allows_public_v6() {
        assert!(!blocked("2606:4700:4700::1111"));
    }
}

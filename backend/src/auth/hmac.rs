//! HMAC-SHA256 signature computation and verification.
//!
//! This module is pure logic — no HTTP types, no Axum, no I/O.
//! It can be unit-tested in complete isolation.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq; // constant-time comparison

type HmacSha256 = Hmac<Sha256>;

/// All the pieces extracted from a request needed to verify its signature.
pub struct SigningInput<'a> {
    /// HTTP method, upper-case (e.g. `"POST"`).
    pub method: &'a str,
    /// Full request path including query string (e.g. `"/api/v1/notifications"`).
    pub path_and_query: &'a str,
    /// Value of the `X-Hedwig-App-Id` header.
    pub app_id: &'a str,
    /// Value of the `X-Hedwig-Timestamp` header (raw string, e.g. `"1713000000"`).
    pub timestamp: &'a str,
    /// Value of the `X-Hedwig-Nonce` header.
    pub nonce: &'a str,
    /// Raw bytes of the request body (empty slice if there is no body).
    pub body: &'a [u8],
}

/// Build the canonical string that is signed/verified.
///
/// Format (newline-separated):
/// ```text
/// {METHOD}
/// {/path?query}
/// {app_id}
/// {timestamp}
/// {nonce}
/// {lowercase-hex SHA-256 of body}
/// ```
pub fn canonical_string(input: &SigningInput<'_>) -> String {
    let body_hash = hex::encode(Sha256::digest(input.body));
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        input.method,
        input.path_and_query,
        input.app_id,
        input.timestamp,
        input.nonce,
        body_hash,
    )
}

/// Compute the HMAC-SHA256 signature for the given input.
///
/// Returns a lowercase hex string.
///
/// # Panics
///
/// Never — `Hmac<Sha256>` accepts keys of any length.
pub fn compute_signature(secret: &[u8], input: &SigningInput<'_>) -> String {
    let canonical = canonical_string(input);
    let mut mac = HmacSha256::new_from_slice(secret)
        .expect("HMAC accepts keys of any length");
    mac.update(canonical.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Verify that `provided_sig` matches the expected signature for `input`.
///
/// Uses constant-time byte comparison to prevent timing-based secret leakage.
/// Both values are compared as raw bytes after hex-decoding; if `provided_sig`
/// is not valid hex the comparison fails without panicking.
pub fn verify_signature(secret: &[u8], input: &SigningInput<'_>, provided_sig: &str) -> bool {
    let expected = compute_signature(secret, input);

    // Decode both to bytes for constant-time comparison.
    // If provided_sig is not valid hex, hex::decode returns Err and we return false.
    match (hex::decode(&expected), hex::decode(provided_sig)) {
        (Ok(exp_bytes), Ok(got_bytes)) => exp_bytes.ct_eq(&got_bytes).into(),
        _ => false,
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input<'a>(body: &'a [u8]) -> SigningInput<'a> {
        SigningInput {
            method: "POST",
            path_and_query: "/api/v1/notifications",
            app_id: "test-app",
            timestamp: "1713000000",
            nonce: "abc123def456xyz7",
            body,
        }
    }

    #[test]
    fn canonical_string_is_deterministic() {
        let input = sample_input(b"{}");
        let s1 = canonical_string(&input);
        let s2 = canonical_string(&input);
        assert_eq!(s1, s2);
    }

    #[test]
    fn canonical_string_contains_all_fields() {
        let input = sample_input(b"hello");
        let s = canonical_string(&input);
        assert!(s.contains("POST"));
        assert!(s.contains("/api/v1/notifications"));
        assert!(s.contains("test-app"));
        assert!(s.contains("1713000000"));
        assert!(s.contains("abc123def456xyz7"));
        // SHA-256 of b"hello"
        assert!(s.contains("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"));
    }

    #[test]
    fn empty_body_uses_empty_hash() {
        let input = sample_input(b"");
        let s = canonical_string(&input);
        // SHA-256 of empty bytes
        assert!(s.contains("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"));
    }

    #[test]
    fn verify_signature_accepts_correct_sig() {
        let secret = b"super-secret-key-for-testing-only";
        let input = sample_input(b"{\"channel\":\"email\"}");
        let sig = compute_signature(secret, &input);
        assert!(verify_signature(secret, &input, &sig));
    }

    #[test]
    fn verify_signature_rejects_wrong_sig() {
        let secret = b"super-secret-key-for-testing-only";
        let input = sample_input(b"{\"channel\":\"email\"}");
        assert!(!verify_signature(secret, &input, "deadbeef"));
    }

    #[test]
    fn verify_signature_rejects_wrong_secret() {
        let input = sample_input(b"body");
        let sig = compute_signature(b"correct-secret-that-is-long-enough", &input);
        assert!(!verify_signature(b"wrong-secret-that-is-also-long---", &input, &sig));
    }

    #[test]
    fn verify_signature_rejects_invalid_hex() {
        let secret = b"super-secret-key-for-testing-only";
        let input = sample_input(b"body");
        assert!(!verify_signature(secret, &input, "not-hex!!"));
    }

    #[test]
    fn different_body_produces_different_signature() {
        let secret = b"super-secret-key-for-testing-only";
        let sig1 = compute_signature(secret, &sample_input(b"body1"));
        let sig2 = compute_signature(secret, &sample_input(b"body2"));
        assert_ne!(sig1, sig2);
    }
}

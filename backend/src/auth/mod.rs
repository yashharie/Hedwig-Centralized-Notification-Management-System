//! Authentication module for Hedwig.
//!
//! # How it works
//!
//! Every protected API request must include three HTTP headers:
//!
//! | Header               | Format                    | Purpose                        |
//! |----------------------|---------------------------|--------------------------------|
//! | `X-Hedwig-App-Id`    | arbitrary string          | Identifies the calling app     |
//! | `X-Hedwig-Timestamp` | Unix seconds (UTC), ASCII | Replay-window check            |
//! | `X-Hedwig-Nonce`     | random string (≥16 chars) | Uniqueness within the window   |
//! | `X-Hedwig-Signature` | lowercase hex HMAC-SHA256 | Proves knowledge of the secret |
//!
//! # Canonical string
//!
//! The signature covers a newline-joined string of:
//!
//! ```text
//! {HTTP_METHOD}\n
//! {/path/with?query}\n
//! {X-Hedwig-App-Id value}\n
//! {X-Hedwig-Timestamp value}\n
//! {X-Hedwig-Nonce value}\n
//! {lowercase hex SHA-256 of the raw request body}
//! ```
//!
//! The body hash is always included; for requests with no body use the
//! SHA-256 of an empty byte string.
//!
//! # Replay protection
//!
//! Requests whose timestamp differs from `now()` by more than
//! `HMAC_TIMESTAMP_TOLERANCE_SECS` (default 300 s) are rejected.
//!
//! Within the acceptance window a nonce cache (in-process `DashMap`) ensures
//! the same `(app_id, nonce)` pair is not accepted twice.  The nonce entry
//! expires after `2 × tolerance` seconds.
//!
//! > **Production note:** the in-process nonce store is local to one server
//! > instance.  For a multi-instance deployment, replace [`NonceStore`] with a
//! > shared cache (e.g. Redis) so replay attempts routed to a different
//! > instance are still caught.

pub mod hmac;
pub mod middleware;
pub mod nonce;

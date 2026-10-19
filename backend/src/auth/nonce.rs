//! In-process nonce store for replay-attack prevention.
//!
//! A nonce (`X-Hedwig-Nonce`) combined with an `app_id` and `timestamp` must
//! be unique within the replay-protection window.  This store remembers seen
//! `(app_id, nonce)` pairs and rejects re-use.
//!
//! ## Expiry
//!
//! Each entry is stamped with the Unix timestamp at insertion.  Entries older
//! than `2 × tolerance_secs` are pruned lazily on every `check_and_insert`
//! call so the map does not grow unboundedly.
//!
//! ## Multi-instance caveat
//!
//! This store is **local to one process**.  In a horizontally-scaled deployment
//! a replay routed to a different instance will not be caught.  Replace this
//! with a shared Redis-backed store for production multi-instance setups.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

/// Key that uniquely identifies one nonce within a replay window.
#[derive(Hash, PartialEq, Eq)]
struct NonceKey {
    app_id: String,
    nonce: String,
}

/// Thread-safe, in-process nonce store.
pub struct NonceStore {
    /// Maps `(app_id, nonce)` → Unix timestamp at insertion.
    inner: Mutex<HashMap<NonceKey, i64>>,
    /// Entries older than this many seconds are expired.
    tolerance_secs: i64,
}

impl NonceStore {
    /// Create a new store.  `tolerance_secs` should equal (or be a small
    /// multiple of) the timestamp tolerance used by the middleware.
    pub fn new(tolerance_secs: i64) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            tolerance_secs,
        }
    }

    /// Attempt to record `(app_id, nonce)` as seen.
    ///
    /// Returns `true` if this is the first time the pair has been seen within
    /// the current acceptance window (i.e. the request is NOT a replay).
    ///
    /// Returns `false` if the pair has already been seen (replay detected).
    pub fn check_and_insert(&self, app_id: &str, nonce: &str) -> bool {
        let now = current_unix_secs();
        let expiry_threshold = now - 2 * self.tolerance_secs;

        let mut map = self.inner.lock().expect("nonce store lock poisoned");

        // Lazy expiry: remove stale entries to bound memory usage.
        map.retain(|_, inserted_at| *inserted_at > expiry_threshold);

        let key = NonceKey {
            app_id: app_id.to_string(),
            nonce: nonce.to_string(),
        };

        if map.contains_key(&key) {
            // Already seen — this is a replay.
            return false;
        }

        map.insert(key, now);
        true
    }
}

fn current_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before Unix epoch")
        .as_secs() as i64
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_nonce_is_accepted() {
        let store = NonceStore::new(300);
        assert!(store.check_and_insert("app1", "nonce-abc"));
    }

    #[test]
    fn duplicate_nonce_is_rejected() {
        let store = NonceStore::new(300);
        assert!(store.check_and_insert("app1", "nonce-abc"));
        assert!(!store.check_and_insert("app1", "nonce-abc"));
    }

    #[test]
    fn same_nonce_different_app_is_accepted() {
        let store = NonceStore::new(300);
        assert!(store.check_and_insert("app1", "nonce-abc"));
        // Different app_id — should be accepted.
        assert!(store.check_and_insert("app2", "nonce-abc"));
    }

    #[test]
    fn different_nonces_same_app_are_accepted() {
        let store = NonceStore::new(300);
        assert!(store.check_and_insert("app1", "nonce-1"));
        assert!(store.check_and_insert("app1", "nonce-2"));
    }
}

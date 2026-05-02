// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Brute-force and credential-stuffing protection for the login endpoint
//! (ASVS V2.2.1 / V6.3.1).
//!
//! [`LoginRateLimiter`] is registered as Rocket managed state and consulted by
//! the `POST /login` handler **before** any credential lookup.
//!
//! ## Architecture
//!
//! The limiter is split into two layers:
//!
//! * [`store::RateLimitStore`] — a small storage trait that owns per-subject
//!   `(failures, locked_until)` records and provides atomic read-modify-write
//!   access through [`store::RateLimitStore::with_record`].
//! * [`LoginRateLimiter`] — the policy layer that knows about thresholds and
//!   delays.  It holds an [`Arc<dyn RateLimitStore>`] and is therefore
//!   trivially swappable in tests or when running behind an HA proxy.
//!
//! The default store is [`store::InMemoryRateLimitStore`], which mirrors the
//! original two-`Mutex<HashMap>` design.  A future Postgres-backed store can
//! be plugged in via [`LoginRateLimiter::with_store`] without touching the
//! login handler or any tests.
//!
//! ## Policy
//!
//! | Trigger | Action |
//! |---|---|
//! | ≥ `DELAY_THRESHOLD` consecutive failures (per username) | Progressive server-side delay: `2 s × (failures − threshold + 1)`, capped at 30 s |
//! | ≥ `USERNAME_LOCKOUT_THRESHOLD` consecutive failures (per username) | 15-minute account lockout |
//! | ≥ `IP_LOCKOUT_THRESHOLD` consecutive failures (per IP) | 15-minute IP lockout |
//! | Successful login | All counters for username **and** IP are reset |
//!
//! Delays are applied with `std::thread::sleep`, which is safe inside Rocket's
//! synchronous route thread pool.

pub mod store;

use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub use store::{AttemptRecord, InMemoryRateLimitStore, RateLimitStore, Scope};

// ---------------------------------------------------------------------------
// Policy constants
// ---------------------------------------------------------------------------

/// Consecutive failures (per username) before a progressive delay begins.
const DELAY_THRESHOLD: u32 = 3;
/// Base seconds added per extra failure above the threshold.
const DELAY_BASE_SECS: u64 = 2;
/// Maximum delay imposed on a single attempt in seconds.
const MAX_DELAY_SECS: u64 = 30;
/// Consecutive per-username failures that trigger an account lockout.
const USERNAME_LOCKOUT_THRESHOLD: u32 = 10;
/// Consecutive per-IP failures that trigger an IP-level lockout.
const IP_LOCKOUT_THRESHOLD: u32 = 20;
/// Lockout duration in seconds (15 minutes).
const LOCKOUT_DURATION_SECS: u64 = 900;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Outcome returned by [`LoginRateLimiter::check_and_delay`].
pub enum RateLimitOutcome {
    /// The attempt may proceed (any applicable delay has already been applied).
    Allowed,
    /// The subject is locked out.  The inner `Duration` is the approximate
    /// remaining lock time (useful for user-facing messages).
    Locked(Duration),
}

/// Rocket managed state that enforces brute-force protections on login.
///
/// The limiter is a thin policy wrapper around a [`RateLimitStore`].  Use
/// [`LoginRateLimiter::new`] for the in-memory default, or
/// [`LoginRateLimiter::with_store`] to plug in a custom backend (e.g. a
/// future Postgres-backed store for HA deployments).
pub struct LoginRateLimiter {
    store: Arc<dyn RateLimitStore>,
}

impl LoginRateLimiter {
    /// Create a new limiter backed by [`InMemoryRateLimitStore`].
    pub fn new() -> Self {
        Self::with_store(Arc::new(InMemoryRateLimitStore::new()))
    }

    /// Create a new limiter backed by a custom store.
    pub fn with_store(store: Arc<dyn RateLimitStore>) -> Self {
        Self { store }
    }

    /// Check whether a login attempt is allowed, applying a blocking delay if
    /// required by progressive-delay policy.
    ///
    /// Returns [`RateLimitOutcome::Locked`] immediately (without sleeping) when
    /// the username or IP is in a lockout period.
    pub fn check_and_delay(&self, username: &str, ip: Option<IpAddr>) -> RateLimitOutcome {
        let now = Instant::now();

        // --- Per-IP check (fast path: reject without touching username state) ---
        if let Some(ip_addr) = ip {
            let mut locked_remaining = None;
            self.store.with_record(Scope::Ip(ip_addr), &mut |rec| {
                if let Some(locked_until) = rec.locked_until {
                    if locked_until > now {
                        locked_remaining = Some(locked_until - now);
                    } else {
                        // Lock expired – reset so it doesn't keep triggering.
                        *rec = AttemptRecord::default();
                    }
                }
            });
            if let Some(remaining) = locked_remaining {
                return RateLimitOutcome::Locked(remaining);
            }
        }

        // --- Per-username check with progressive delay ---
        let mut delay = Duration::ZERO;
        let mut locked_remaining = None;
        self.store
            .with_record(Scope::Username(username), &mut |rec| {
                if let Some(locked_until) = rec.locked_until {
                    if locked_until > now {
                        locked_remaining = Some(locked_until - now);
                        return;
                    }
                    *rec = AttemptRecord::default();
                }
                if rec.failures >= DELAY_THRESHOLD {
                    let extra = (rec.failures - DELAY_THRESHOLD + 1) as u64;
                    delay = Duration::from_secs((extra * DELAY_BASE_SECS).min(MAX_DELAY_SECS));
                }
            });

        if let Some(remaining) = locked_remaining {
            return RateLimitOutcome::Locked(remaining);
        }

        if !delay.is_zero() {
            std::thread::sleep(delay);
        }

        RateLimitOutcome::Allowed
    }

    /// Record a failed login attempt for `username` and/or `ip`.
    pub fn record_failure(&self, username: &str, ip: Option<IpAddr>) {
        let now = Instant::now();

        self.store
            .with_record(Scope::Username(username), &mut |rec| {
                if rec.locked_until.map(|t| t > now).unwrap_or(false) {
                    return;
                }
                rec.failures += 1;
                if rec.failures >= USERNAME_LOCKOUT_THRESHOLD {
                    rec.locked_until = Some(now + Duration::from_secs(LOCKOUT_DURATION_SECS));
                }
            });

        if let Some(ip_addr) = ip {
            self.store.with_record(Scope::Ip(ip_addr), &mut |rec| {
                if rec.locked_until.map(|t| t > now).unwrap_or(false) {
                    return;
                }
                rec.failures += 1;
                if rec.failures >= IP_LOCKOUT_THRESHOLD {
                    rec.locked_until = Some(now + Duration::from_secs(LOCKOUT_DURATION_SECS));
                }
            });
        }
    }

    /// Clear all failure/lockout state for `username` and `ip` after a
    /// successful authentication.
    pub fn record_success(&self, username: &str, ip: Option<IpAddr>) {
        self.store.clear(Scope::Username(username));
        if let Some(ip_addr) = ip {
            self.store.clear(Scope::Ip(ip_addr));
        }
    }
}

impl Default for LoginRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_first_attempt() {
        let limiter = LoginRateLimiter::new();
        let result = limiter.check_and_delay("alice", None);
        assert!(matches!(result, RateLimitOutcome::Allowed));
    }

    #[test]
    fn allows_attempts_below_delay_threshold() {
        let limiter = LoginRateLimiter::new();
        for _ in 0..DELAY_THRESHOLD - 1 {
            limiter.record_failure("alice", None);
        }
        let result = limiter.check_and_delay("alice", None);
        assert!(matches!(result, RateLimitOutcome::Allowed));
    }

    #[test]
    fn locks_after_username_threshold() {
        let limiter = LoginRateLimiter::new();
        for _ in 0..USERNAME_LOCKOUT_THRESHOLD {
            limiter.record_failure("alice", None);
        }
        let result = limiter.check_and_delay("alice", None);
        assert!(matches!(result, RateLimitOutcome::Locked(_)));
    }

    #[test]
    fn success_clears_username_failures() {
        let limiter = LoginRateLimiter::new();
        for _ in 0..5 {
            limiter.record_failure("alice", None);
        }
        limiter.record_success("alice", None);
        let result = limiter.check_and_delay("alice", None);
        assert!(matches!(result, RateLimitOutcome::Allowed));
    }

    #[test]
    fn ip_lockout_after_threshold() {
        let limiter = LoginRateLimiter::new();
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        for i in 0..IP_LOCKOUT_THRESHOLD {
            limiter.record_failure(&format!("user{}", i), Some(ip));
        }
        // Different username – should still be blocked at IP level.
        let result = limiter.check_and_delay("new_user", Some(ip));
        assert!(matches!(result, RateLimitOutcome::Locked(_)));
    }

    #[test]
    fn success_clears_ip_failures() {
        let limiter = LoginRateLimiter::new();
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        for _ in 0..5 {
            limiter.record_failure("alice", Some(ip));
        }
        limiter.record_success("alice", Some(ip));
        let result = limiter.check_and_delay("alice", Some(ip));
        assert!(matches!(result, RateLimitOutcome::Allowed));
    }

    #[test]
    fn locked_duration_is_positive() {
        let limiter = LoginRateLimiter::new();
        for _ in 0..USERNAME_LOCKOUT_THRESHOLD {
            limiter.record_failure("bob", None);
        }
        match limiter.check_and_delay("bob", None) {
            RateLimitOutcome::Locked(remaining) => {
                assert!(remaining > Duration::ZERO);
                assert!(remaining <= Duration::from_secs(LOCKOUT_DURATION_SECS));
            }
            RateLimitOutcome::Allowed => panic!("expected lockout"),
        }
    }

    /// Custom store proves the trait seam works end-to-end without relying on
    /// the in-memory default.
    #[test]
    fn limiter_works_with_custom_store() {
        use std::sync::Mutex;

        #[derive(Default)]
        struct CountingStore {
            inner: InMemoryRateLimitStore,
            with_record_calls: Mutex<u32>,
            clear_calls: Mutex<u32>,
        }

        impl RateLimitStore for CountingStore {
            fn with_record(&self, scope: Scope<'_>, f: &mut dyn FnMut(&mut AttemptRecord)) {
                *self.with_record_calls.lock().unwrap() += 1;
                self.inner.with_record(scope, f);
            }
            fn clear(&self, scope: Scope<'_>) {
                *self.clear_calls.lock().unwrap() += 1;
                self.inner.clear(scope);
            }
        }

        let store = Arc::new(CountingStore::default());
        let limiter = LoginRateLimiter::with_store(store.clone());
        limiter.record_failure("alice", None);
        limiter.record_success("alice", None);

        assert!(*store.with_record_calls.lock().unwrap() >= 1);
        assert!(*store.clear_calls.lock().unwrap() >= 1);
    }
}

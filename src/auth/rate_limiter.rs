// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Brute-force and credential-stuffing protection for the login endpoint
//! (ASVS V2.2.1 / V6.3.1).
//!
//! [`LoginRateLimiter`] is registered as Rocket managed state and consulted by
//! the `POST /login` handler **before** any credential lookup.
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

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

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
// Internal tracking record
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct AttemptRecord {
    /// Consecutive failure count (zeroed on success).
    failures: u32,
    /// When this subject is locked out until, if at all.
    locked_until: Option<Instant>,
}

impl AttemptRecord {
    fn new() -> Self {
        Self {
            failures: 0,
            locked_until: None,
        }
    }
}

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
pub struct LoginRateLimiter {
    by_username: Mutex<HashMap<String, AttemptRecord>>,
    by_ip: Mutex<HashMap<IpAddr, AttemptRecord>>,
}

impl LoginRateLimiter {
    /// Create a new, empty limiter.
    pub fn new() -> Self {
        Self {
            by_username: Mutex::new(HashMap::new()),
            by_ip: Mutex::new(HashMap::new()),
        }
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
            let mut map = self.by_ip.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(rec) = map.get_mut(&ip_addr) {
                if let Some(locked_until) = rec.locked_until {
                    if locked_until > now {
                        return RateLimitOutcome::Locked(locked_until - now);
                    }
                    // Lock expired – reset so it doesn't keep triggering.
                    *rec = AttemptRecord::new();
                }
            }
        }

        // --- Per-username check with progressive delay ---
        let delay = {
            let mut map = self.by_username.lock().unwrap_or_else(|p| p.into_inner());
            match map.get_mut(username) {
                Some(rec) => {
                    if let Some(locked_until) = rec.locked_until {
                        if locked_until > now {
                            return RateLimitOutcome::Locked(locked_until - now);
                        }
                        // Lock expired – reset.
                        *rec = AttemptRecord::new();
                        Duration::ZERO
                    } else if rec.failures >= DELAY_THRESHOLD {
                        let extra = (rec.failures - DELAY_THRESHOLD + 1) as u64;
                        Duration::from_secs((extra * DELAY_BASE_SECS).min(MAX_DELAY_SECS))
                    } else {
                        Duration::ZERO
                    }
                }
                None => Duration::ZERO,
            }
        };

        if !delay.is_zero() {
            std::thread::sleep(delay);
        }

        RateLimitOutcome::Allowed
    }

    /// Record a failed login attempt for `username` and/or `ip`.
    pub fn record_failure(&self, username: &str, ip: Option<IpAddr>) {
        let now = Instant::now();

        {
            let mut map = self.by_username.lock().unwrap_or_else(|p| p.into_inner());
            let rec = map
                .entry(username.to_string())
                .or_insert_with(AttemptRecord::new);
            // Do not increment if still within an active lockout window.
            if rec.locked_until.map(|t| t > now).unwrap_or(false) {
                return;
            }
            rec.failures += 1;
            if rec.failures >= USERNAME_LOCKOUT_THRESHOLD {
                rec.locked_until = Some(now + Duration::from_secs(LOCKOUT_DURATION_SECS));
            }
        }

        if let Some(ip_addr) = ip {
            let mut map = self.by_ip.lock().unwrap_or_else(|p| p.into_inner());
            let rec = map.entry(ip_addr).or_insert_with(AttemptRecord::new);
            if rec.locked_until.map(|t| t > now).unwrap_or(false) {
                return;
            }
            rec.failures += 1;
            if rec.failures >= IP_LOCKOUT_THRESHOLD {
                rec.locked_until = Some(now + Duration::from_secs(LOCKOUT_DURATION_SECS));
            }
        }
    }

    /// Clear all failure/lockout state for `username` and `ip` after a
    /// successful authentication.
    pub fn record_success(&self, username: &str, ip: Option<IpAddr>) {
        {
            let mut map = self.by_username.lock().unwrap_or_else(|p| p.into_inner());
            map.remove(username);
        }
        if let Some(ip_addr) = ip {
            let mut map = self.by_ip.lock().unwrap_or_else(|p| p.into_inner());
            map.remove(&ip_addr);
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
}

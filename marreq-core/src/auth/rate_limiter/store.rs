// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Storage backends for [`super::LoginRateLimiter`].
//!
//! The [`RateLimitStore`] trait abstracts the per-subject `(failures,
//! locked_until)` state used by the brute-force limiter so that the policy
//! layer can run unchanged against:
//!
//! * an in-process `Mutex<HashMap>` ([`InMemoryRateLimitStore`], default),
//!   suitable for single-instance deployments; and
//! * a future shared backend (e.g. Postgres advisory locks + a TTL table),
//!   wired in via [`super::LoginRateLimiter::with_store`] without touching
//!   any callers.
//!
//! ## Atomicity contract
//!
//! Implementations **must** make [`RateLimitStore::with_record`] atomic with
//! respect to other concurrent calls for the same [`Scope`].  Callers rely on
//! reading and updating the record inside the closure as one logical step
//! (otherwise two failed logins racing would each see an old counter and
//! under-count).

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Instant;

/// Mutable per-subject record.  `failures` is a monotonically increasing
/// counter, reset to zero on a successful login (via
/// [`RateLimitStore::clear`]) or on natural lockout expiry.
#[derive(Debug, Default, Clone)]
pub struct AttemptRecord {
    /// Consecutive failure count.
    pub failures: u32,
    /// When this subject is locked out until, if at all.
    pub locked_until: Option<Instant>,
}

/// Identifies *what* is being rate-limited.  Username and IP scopes have
/// disjoint key spaces.
#[derive(Debug, Clone, Copy)]
pub enum Scope<'a> {
    Username(&'a str),
    Ip(IpAddr),
}

/// Backend for the brute-force limiter.  See module docs for the atomicity
/// contract.
pub trait RateLimitStore: Send + Sync {
    /// Atomically read-modify-write the record for `scope`.
    ///
    /// The closure receives a mutable borrow of the record (creating it if
    /// missing).  On return, the new value is persisted.
    fn with_record(&self, scope: Scope<'_>, f: &mut dyn FnMut(&mut AttemptRecord));

    /// Remove any record for `scope` (i.e. reset to "no failures").
    fn clear(&self, scope: Scope<'_>);
}

/// In-process store backed by two mutex-protected hash maps.
///
/// This is the default and is appropriate for single-instance deployments.
/// For HA deployments a shared backend (e.g. Postgres) should be used so
/// brute-force counters survive across replicas.
pub struct InMemoryRateLimitStore {
    by_username: Mutex<HashMap<String, AttemptRecord>>,
    by_ip: Mutex<HashMap<IpAddr, AttemptRecord>>,
}

impl InMemoryRateLimitStore {
    pub fn new() -> Self {
        Self {
            by_username: Mutex::new(HashMap::new()),
            by_ip: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryRateLimitStore {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimitStore for InMemoryRateLimitStore {
    fn with_record(&self, scope: Scope<'_>, f: &mut dyn FnMut(&mut AttemptRecord)) {
        match scope {
            Scope::Username(name) => {
                let mut map = self.by_username.lock().unwrap_or_else(|p| p.into_inner());
                let rec = map.entry(name.to_string()).or_default();
                f(rec);
            }
            Scope::Ip(ip) => {
                let mut map = self.by_ip.lock().unwrap_or_else(|p| p.into_inner());
                let rec = map.entry(ip).or_default();
                f(rec);
            }
        }
    }

    fn clear(&self, scope: Scope<'_>) {
        match scope {
            Scope::Username(name) => {
                let mut map = self.by_username.lock().unwrap_or_else(|p| p.into_inner());
                map.remove(name);
            }
            Scope::Ip(ip) => {
                let mut map = self.by_ip.lock().unwrap_or_else(|p| p.into_inner());
                map.remove(&ip);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_record_creates_entry_on_first_use() {
        let store = InMemoryRateLimitStore::new();
        store.with_record(Scope::Username("alice"), &mut |rec| {
            assert_eq!(rec.failures, 0);
            assert!(rec.locked_until.is_none());
            rec.failures = 7;
        });
        store.with_record(Scope::Username("alice"), &mut |rec| {
            assert_eq!(rec.failures, 7);
        });
    }

    #[test]
    fn clear_removes_entry() {
        let store = InMemoryRateLimitStore::new();
        store.with_record(Scope::Username("alice"), &mut |rec| {
            rec.failures = 3;
        });
        store.clear(Scope::Username("alice"));
        store.with_record(Scope::Username("alice"), &mut |rec| {
            assert_eq!(rec.failures, 0);
        });
    }

    #[test]
    fn username_and_ip_scopes_are_independent() {
        let store = InMemoryRateLimitStore::new();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        store.with_record(Scope::Username("alice"), &mut |rec| rec.failures = 4);
        store.with_record(Scope::Ip(ip), &mut |rec| rec.failures = 9);
        store.with_record(Scope::Username("alice"), &mut |rec| {
            assert_eq!(rec.failures, 4);
        });
        store.with_record(Scope::Ip(ip), &mut |rec| {
            assert_eq!(rec.failures, 9);
        });
    }
}

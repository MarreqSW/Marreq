// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Shared PostgreSQL backend for the login rate limiter (issue #286).
//!
//! Every backend replica reads and updates the same `login_rate_limits` rows,
//! so failed-login counters and lockouts are coordinated across instances and
//! survive restarts. The policy layer ([`super::LoginRateLimiter`]) is
//! unchanged:
//!
//! * **Atomicity.** [`RateLimitStore::with_record`] runs in one transaction
//!   that locks the subject's row (`SELECT … FOR UPDATE`), so concurrent
//!   updates from any replica are serialised per [`Scope`].
//! * **Time.** [`AttemptRecord::locked_until`] is a process-local `Instant`.
//!   It is stored as an absolute `timestamptz` and converted through the
//!   *database* clock (`now()` in the same transaction), so clock skew between
//!   replicas does not matter.
//! * **Expiry.** A row is deleted when the subject logs in successfully;
//!   [`PostgresRateLimitStore::purge_expired`] (run by [`RateLimitSweep`])
//!   removes records idle for longer than the retention whose lock, if any,
//!   has passed.
//! * **Failures.** The trait has no error channel. A database error is logged
//!   and the record is left as it was, which lets the attempt through
//!   ("fail open"): the login itself needs the same database, and failing
//!   closed would lock every user out after an outage.

use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use diesel::prelude::*;
use diesel::sql_types::{Double, Integer, Nullable, Text};

use super::store::{AttemptRecord, RateLimitStore, Scope};
use crate::repository::diesel_repo::ConnectionPool;

/// How often [`RateLimitSweep`] removes expired records.
pub const SWEEP_INTERVAL: Duration = Duration::from_secs(10 * 60);

/// Attempts at locking a row that a concurrent `clear` keeps deleting.
const LOCK_ATTEMPTS: usize = 3;

#[derive(QueryableByName)]
struct StoredRecord {
    #[diesel(sql_type = Integer)]
    failures: i32,
    /// `locked_until - now()` in seconds (negative once passed).
    #[diesel(sql_type = Nullable<Double>)]
    lock_secs: Option<f64>,
}

/// `(scope_kind, scope_key)` of a subject. Usernames arrive canonical.
fn scope_key(scope: Scope<'_>) -> (&'static str, String) {
    match scope {
        Scope::Username(name) => ("username", name.to_string()),
        Scope::Ip(ip) => ("ip", ip_key(ip)),
    }
}

fn ip_key(ip: IpAddr) -> String {
    ip.to_string()
}

/// A stored lock (seconds until it ends, by the database clock) as an
/// `Instant` of this process. A passed lock lies in the past by how long ago
/// it ended (at least a second), so it is also expired for the policy, which
/// read its own `now` *before* calling the store.
fn instant_from_remaining(now: Instant, lock_secs: Option<f64>) -> Option<Instant> {
    let secs = lock_secs?;
    if secs.is_finite() && secs > 0.0 {
        return Some(now + Duration::from_secs_f64(secs));
    }
    let ago = if secs.is_finite() {
        (-secs).max(1.0)
    } else {
        1.0
    };
    Some(
        now.checked_sub(Duration::from_secs_f64(ago))
            .or_else(|| now.checked_sub(Duration::from_millis(1)))
            .unwrap_or(now),
    )
}

/// An `Instant` lock as seconds from now, to store as `now() + secs`.
fn remaining_from_instant(now: Instant, locked_until: Option<Instant>) -> Option<f64> {
    locked_until.map(|until| until.saturating_duration_since(now).as_secs_f64())
}

fn is_default(record: &AttemptRecord) -> bool {
    record.failures == 0 && record.locked_until.is_none()
}

/// [`RateLimitStore`] on the `login_rate_limits` table.
pub struct PostgresRateLimitStore {
    pool: Arc<ConnectionPool>,
}

impl PostgresRateLimitStore {
    pub fn new(pool: Arc<ConnectionPool>) -> Self {
        Self { pool }
    }

    /// On the application's shared connection pool (initialised at startup).
    pub fn from_shared_pool() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::new(crate::repository::diesel_repo::shared_pool()?))
    }

    fn try_with_record(
        &self,
        scope: Scope<'_>,
        f: &mut dyn FnMut(&mut AttemptRecord),
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (kind, key) = scope_key(scope);
        let mut conn = self.pool.get()?;
        for _ in 0..LOCK_ATTEMPTS {
            let done = conn.transaction::<bool, diesel::result::Error, _>(|conn| {
                diesel::sql_query(
                    "INSERT INTO login_rate_limits (scope_kind, scope_key) VALUES ($1, $2) \
                     ON CONFLICT DO NOTHING",
                )
                .bind::<Text, _>(kind)
                .bind::<Text, _>(&key)
                .execute(conn)?;
                let Some(stored) = diesel::sql_query(
                    "SELECT failures, EXTRACT(EPOCH FROM (locked_until - now()))::float8 AS lock_secs \
                     FROM login_rate_limits WHERE scope_kind = $1 AND scope_key = $2 FOR UPDATE",
                )
                .bind::<Text, _>(kind)
                .bind::<Text, _>(&key)
                .get_result::<StoredRecord>(conn)
                .optional()?
                else {
                    // A concurrent `clear` deleted the row: start over.
                    return Ok(false);
                };

                let now = Instant::now();
                let mut record = AttemptRecord {
                    failures: u32::try_from(stored.failures).unwrap_or(0),
                    locked_until: instant_from_remaining(now, stored.lock_secs),
                };
                f(&mut record);

                // Nothing to remember (e.g. a check on a clean subject): no row.
                if is_default(&record) {
                    diesel::sql_query(
                        "DELETE FROM login_rate_limits WHERE scope_kind = $1 AND scope_key = $2",
                    )
                    .bind::<Text, _>(kind)
                    .bind::<Text, _>(&key)
                    .execute(conn)?;
                    return Ok(true);
                }
                let failures = i32::try_from(record.failures).unwrap_or(i32::MAX);
                diesel::sql_query(
                    "UPDATE login_rate_limits SET failures = $3, \
                     locked_until = CASE WHEN $4::float8 IS NULL THEN NULL \
                                         ELSE now() + make_interval(secs => $4::float8) END, \
                     updated_at = now() \
                     WHERE scope_kind = $1 AND scope_key = $2",
                )
                .bind::<Text, _>(kind)
                .bind::<Text, _>(&key)
                .bind::<Integer, _>(failures)
                .bind::<Nullable<Double>, _>(remaining_from_instant(now, record.locked_until))
                .execute(conn)?;
                Ok(true)
            })?;
            if done {
                return Ok(());
            }
        }
        Err("the record kept being removed concurrently".into())
    }

    /// Delete records idle for longer than `retention` whose lock, if any,
    /// has passed. Returns the number removed. Blocking.
    pub fn purge_expired(&self, retention: Duration) -> Result<usize, String> {
        let mut conn = self.pool.get().map_err(|e| e.to_string())?;
        let removed = diesel::sql_query(
            "DELETE FROM login_rate_limits \
             WHERE updated_at < now() - make_interval(secs => $1) \
               AND (locked_until IS NULL OR locked_until <= now())",
        )
        .bind::<Double, _>(retention.as_secs_f64())
        .execute(&mut conn)
        .map_err(|e| e.to_string())?;
        Ok(removed)
    }
}

impl RateLimitStore for PostgresRateLimitStore {
    fn with_record(&self, scope: Scope<'_>, f: &mut dyn FnMut(&mut AttemptRecord)) {
        if let Err(e) = self.try_with_record(scope, f) {
            eprintln!(
                "[marreq] login rate limit store: could not update a record ({e}); allowing the attempt"
            );
        }
    }

    fn clear(&self, scope: Scope<'_>) {
        let (kind, key) = scope_key(scope);
        let result = self
            .pool
            .get()
            .map_err(|e| e.to_string())
            .and_then(|mut conn| {
                diesel::sql_query(
                    "DELETE FROM login_rate_limits WHERE scope_kind = $1 AND scope_key = $2",
                )
                .bind::<Text, _>(kind)
                .bind::<Text, _>(&key)
                .execute(&mut conn)
                .map_err(|e| e.to_string())
            });
        if let Err(e) = result {
            eprintln!("[marreq] login rate limit store: could not clear a record ({e})");
        }
    }
}

/// Liftoff fairing that removes expired rate-limit records every
/// [`SWEEP_INTERVAL`] (on every replica; the delete is idempotent).
pub struct RateLimitSweep {
    store: Arc<PostgresRateLimitStore>,
    retention: Duration,
}

impl RateLimitSweep {
    pub fn new(store: Arc<PostgresRateLimitStore>, retention: Duration) -> Self {
        Self { store, retention }
    }
}

#[rocket::async_trait]
impl rocket::fairing::Fairing for RateLimitSweep {
    fn info(&self) -> rocket::fairing::Info {
        rocket::fairing::Info {
            name: "Login rate limit sweep",
            kind: rocket::fairing::Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, _rocket: &rocket::Rocket<rocket::Orbit>) {
        let store = Arc::clone(&self.store);
        let retention = self.retention;
        rocket::tokio::spawn(async move {
            let mut tick = rocket::tokio::time::interval(SWEEP_INTERVAL);
            loop {
                tick.tick().await;
                let store = Arc::clone(&store);
                let swept =
                    rocket::tokio::task::spawn_blocking(move || store.purge_expired(retention))
                        .await;
                if let Ok(Err(e)) = swept {
                    eprintln!("[marreq] login rate limit sweep failed: {e}");
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_keys_keep_username_and_ip_apart() {
        assert_eq!(
            scope_key(Scope::Username("alice")),
            ("username", "alice".to_string())
        );
        let ip: IpAddr = "10.0.0.7".parse().unwrap();
        assert_eq!(scope_key(Scope::Ip(ip)), ("ip", "10.0.0.7".to_string()));
        let v6: IpAddr = "2001:db8::1".parse().unwrap();
        assert_eq!(scope_key(Scope::Ip(v6)).1, "2001:db8::1");
    }

    #[test]
    fn stored_locks_become_instants_relative_to_now() {
        let now = Instant::now();
        assert_eq!(instant_from_remaining(now, None), None);
        assert_eq!(
            instant_from_remaining(now, Some(90.0)),
            Some(now + Duration::from_secs(90))
        );
        // A passed lock lies in the past, before any `now` the policy read
        // earlier: the policy sees it as expired.
        assert_eq!(
            instant_from_remaining(now, Some(-5.0)),
            Some(now - Duration::from_secs(5))
        );
        assert_eq!(
            instant_from_remaining(now, Some(0.0)),
            Some(now - Duration::from_secs(1))
        );
        assert_eq!(
            instant_from_remaining(now, Some(f64::NAN)),
            Some(now - Duration::from_secs(1))
        );
    }

    #[test]
    fn instants_become_seconds_from_now() {
        let now = Instant::now();
        assert_eq!(remaining_from_instant(now, None), None);
        assert_eq!(
            remaining_from_instant(now, Some(now + Duration::from_secs(900))),
            Some(900.0)
        );
        assert_eq!(remaining_from_instant(now, Some(now)), Some(0.0));
    }

    #[test]
    fn round_trip_keeps_the_remaining_lock() {
        let now = Instant::now();
        let until = now + Duration::from_millis(1500);
        let secs = remaining_from_instant(now, Some(until));
        assert_eq!(instant_from_remaining(now, secs), Some(until));
    }
}

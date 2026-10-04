// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! The shared PostgreSQL login rate-limit store across replicas (issue #286).
//! Each "replica" is a `LoginRateLimiter` on its own connection pool, as two
//! server processes would be. Set `MARREQ_TEST_DATABASE_URL` to a migrated,
//! disposable database; the test truncates `login_rate_limits`.
//!
//! The policy thresholds are the documented ones: lockout after 10 failures
//! per username or 20 per IP, for 15 minutes.

#![cfg(not(feature = "test-helpers"))]

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use marreq_core::auth::rate_limiter::postgres::PostgresRateLimitStore;
use marreq_core::auth::rate_limiter::{LoginRateLimiter, RateLimitOutcome};

const USERNAME_LOCKOUT: usize = 10;
const IP_LOCKOUT: usize = 20;

type PgPool = Pool<ConnectionManager<PgConnection>>;

fn pool(url: &str) -> Arc<PgPool> {
    Arc::new(
        Pool::builder()
            .max_size(4)
            .min_idle(Some(0))
            .connection_timeout(Duration::from_secs(5))
            .build(ConnectionManager::new(url))
            .expect("test pool"),
    )
}

struct Replica {
    limiter: LoginRateLimiter,
    store: Arc<PostgresRateLimitStore>,
}

fn replica(url: &str) -> Replica {
    let store = Arc::new(PostgresRateLimitStore::new(pool(url)));
    Replica {
        limiter: LoginRateLimiter::with_store(store.clone()),
        store,
    }
}

#[derive(QueryableByName)]
struct Row {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    failures: i32,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    locked: bool,
}

/// `(failures, locked)` of a subject, or `None` without a row.
fn row(conn: &mut PgConnection, kind: &str, key: &str) -> Option<(i32, bool)> {
    diesel::sql_query(
        "SELECT failures, COALESCE(locked_until > now(), false) AS locked \
         FROM login_rate_limits WHERE scope_kind = $1 AND scope_key = $2",
    )
    .bind::<diesel::sql_types::Text, _>(kind)
    .bind::<diesel::sql_types::Text, _>(key)
    .get_result::<Row>(conn)
    .optional()
    .unwrap()
    .map(|r| (r.failures, r.locked))
}

fn locked(outcome: RateLimitOutcome) -> Option<Duration> {
    match outcome {
        RateLimitOutcome::Locked(remaining) => Some(remaining),
        RateLimitOutcome::Allowed => None,
    }
}

#[test]
fn replicas_share_login_rate_limits() {
    let Ok(url) = std::env::var("MARREQ_TEST_DATABASE_URL") else {
        eprintln!(
            "skipping PostgreSQL login rate limit test: set MARREQ_TEST_DATABASE_URL to a disposable migrated database to run it"
        );
        return;
    };
    let mut conn = PgConnection::establish(&url).expect("test PostgreSQL connection");
    conn.batch_execute("TRUNCATE login_rate_limits").unwrap();
    let a = replica(&url);
    let b = replica(&url);

    // Failures split across replicas lock the username on both.
    for i in 0..USERNAME_LOCKOUT {
        let r = if i % 2 == 0 { &a } else { &b };
        r.limiter.record_failure("ana", None);
    }
    assert_eq!(row(&mut conn, "username", "ana"), Some((10, true)));
    for r in [&a, &b] {
        let remaining = locked(r.limiter.check_and_delay("ana", None)).expect("locked");
        assert!(remaining > Duration::from_secs(890) && remaining <= Duration::from_secs(900));
    }

    // A success on one replica clears the record for all.
    a.limiter.record_failure("ben", None);
    a.limiter.record_failure("ben", None);
    assert_eq!(row(&mut conn, "username", "ben"), Some((2, false)));
    b.limiter.record_success("ben", None);
    assert_eq!(row(&mut conn, "username", "ben"), None);

    // The IP scope is shared too (different usernames, one address).
    let ip: IpAddr = "10.0.0.9".parse().unwrap();
    for i in 0..IP_LOCKOUT {
        let r = if i % 2 == 0 { &a } else { &b };
        r.limiter.record_failure(&format!("user{i}"), Some(ip));
    }
    assert!(locked(b.limiter.check_and_delay("someone-else", Some(ip))).is_some());
    assert!(locked(a.limiter.check_and_delay("someone-else", Some(ip))).is_some());

    // Concurrent updates from both replicas are not lost.
    let replicas = Arc::new((a, b));
    let hammer = |per_thread: usize| {
        let handles: Vec<_> = (0..8)
            .map(|t| {
                let replicas = Arc::clone(&replicas);
                std::thread::spawn(move || {
                    for _ in 0..per_thread {
                        let r = if t % 2 == 0 { &replicas.0 } else { &replicas.1 };
                        r.limiter.record_failure("cara", None);
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    };
    hammer(1);
    assert_eq!(
        row(&mut conn, "username", "cara"),
        Some((8, false)),
        "8 concurrent failures"
    );
    hammer(3);
    assert_eq!(
        row(&mut conn, "username", "cara"),
        Some((10, true)),
        "counting stops at the lockout"
    );
    let (a, b) = match Arc::try_unwrap(replicas) {
        Ok(pair) => pair,
        Err(_) => panic!("threads still hold the replicas"),
    };

    // A passed lock lets the user in again and resets the record.
    conn.batch_execute(
        "UPDATE login_rate_limits SET locked_until = now() - interval '1 second' \
         WHERE scope_kind = 'username' AND scope_key = 'ana'",
    )
    .unwrap();
    assert!(locked(b.limiter.check_and_delay("ana", None)).is_none());
    assert_eq!(row(&mut conn, "username", "ana"), None);

    // The sweep removes idle records, but never a running lock or a recent one.
    conn.batch_execute(
        "TRUNCATE login_rate_limits;
         INSERT INTO login_rate_limits (scope_kind, scope_key, failures, locked_until, updated_at) VALUES
           ('username', 'old-idle', 2, NULL, now() - interval '48 hours'),
           ('username', 'old-lock-passed', 10, now() - interval '1 hour', now() - interval '48 hours'),
           ('username', 'old-but-locked', 10, now() + interval '10 minutes', now() - interval '48 hours'),
           ('ip', '10.0.0.1', 1, NULL, now());",
    )
    .unwrap();
    assert_eq!(a.store.purge_expired(Duration::from_secs(24 * 3600)), Ok(2));
    assert_eq!(row(&mut conn, "username", "old-idle"), None);
    assert_eq!(row(&mut conn, "username", "old-lock-passed"), None);
    assert_eq!(
        row(&mut conn, "username", "old-but-locked"),
        Some((10, true))
    );
    assert_eq!(row(&mut conn, "ip", "10.0.0.1"), Some((1, false)));

    // A check on a clean subject leaves no row behind.
    assert!(locked(a.limiter.check_and_delay("dora", Some(ip_of("10.0.0.2")))).is_none());
    assert_eq!(row(&mut conn, "username", "dora"), None);
    assert_eq!(row(&mut conn, "ip", "10.0.0.2"), None);

    // Without a database the store fails open instead of locking users out.
    let unreachable = Pool::builder()
        .max_size(1)
        .min_idle(Some(0))
        .connection_timeout(Duration::from_millis(500))
        .build_unchecked(ConnectionManager::<PgConnection>::new(
            "postgres://nobody:nothing@127.0.0.1:1/none",
        ));
    let offline =
        LoginRateLimiter::with_store(Arc::new(PostgresRateLimitStore::new(Arc::new(unreachable))));
    offline.record_failure("erin", None);
    assert!(locked(offline.check_and_delay("erin", None)).is_none());

    conn.batch_execute("TRUNCATE login_rate_limits").unwrap();
}

fn ip_of(text: &str) -> IpAddr {
    text.parse().unwrap()
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Session lifetimes against PostgreSQL (issue #285): the absolute and idle
//! conditions of `find_active_session`, the conditional `touch_session` and
//! `purge_expired_sessions`. Set `MARREQ_TEST_DATABASE_URL` to a migrated,
//! disposable database; the test truncates application data.

#![cfg(not(feature = "test-helpers"))]

use chrono::{Duration, NaiveDateTime, Utc};
use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use marreq_core::auth::session_config::SessionConfig;
use marreq_core::repository::{DieselRepo, SessionRepository};

/// Insert a session `created` and `seen` hours ago (UTC, like the server).
fn insert(conn: &mut PgConnection, token: &str, created_h: i64, seen_h: i64, expires_h: i64) {
    conn.batch_execute(&format!(
        "INSERT INTO sessions (token_hash, user_id, created_at, last_seen_at, expires_at)
         VALUES (rpad('{token}', 64, '0'), 1,
                 (now() AT TIME ZONE 'utc') - interval '{created_h} hours',
                 (now() AT TIME ZONE 'utc') - interval '{seen_h} hours',
                 (now() AT TIME ZONE 'utc') + interval '{expires_h} hours')"
    ))
    .expect("insert session");
}

fn hash(token: &str) -> String {
    format!("{token:0<64}")
}

fn last_seen(conn: &mut PgConnection, token: &str) -> Option<NaiveDateTime> {
    use marreq_core::schema::sessions::dsl;
    dsl::sessions
        .filter(dsl::token_hash.eq(hash(token)))
        .select(dsl::last_seen_at)
        .first(conn)
        .optional()
        .unwrap()
}

#[test]
fn session_limits_touch_and_purge() {
    let Ok(database_url) = std::env::var("MARREQ_TEST_DATABASE_URL") else {
        eprintln!(
            "skipping PostgreSQL session expiry test: set MARREQ_TEST_DATABASE_URL to a disposable migrated database to run it"
        );
        return;
    };
    // SAFETY: this is the only test in its binary and it runs before any
    // database pool or other thread that reads the environment exists.
    unsafe {
        std::env::set_var("DATABASE_URL", &database_url);
    }
    let mut conn = PgConnection::establish(&database_url).expect("test PostgreSQL connection");
    conn.batch_execute(
        "TRUNCATE users RESTART IDENTITY CASCADE;
         INSERT INTO users (id,username,name,email,password_hash,is_admin) VALUES (1,'rita','Rita','rita@example.test','!',false);",
    )
    .expect("seed");
    insert(&mut conn, "active", 24, 1, 100);
    insert(&mut conn, "idle", 24, 9, 100);
    insert(&mut conn, "old", 31 * 24, 1, 100);
    insert(&mut conn, "expired", 24, 1, -1);

    marreq_core::repository::diesel_repo::init_connection_pool().expect("pool");
    let mut repo = DieselRepo::new().expect("repo");
    let config = SessionConfig::default(); // 30 days, 8 hours idle
    let cutoffs = config.cutoffs(Utc::now().naive_utc());
    let found = |repo: &DieselRepo, token: &str| {
        repo.find_active_session(&hash(token), &cutoffs)
            .unwrap()
            .is_some()
    };

    // Lookup enforces every limit in SQL.
    assert!(found(&repo, "active"));
    assert!(!found(&repo, "idle"), "idle for 9 h");
    assert!(!found(&repo, "old"), "past the 30-day limit");
    assert!(!found(&repo, "expired"), "past expires_at");
    let no_idle = SessionConfig {
        idle: None,
        ..config.clone()
    }
    .cutoffs(Utc::now().naive_utc());
    assert!(
        repo.find_active_session(&hash("idle"), &no_idle)
            .unwrap()
            .is_some(),
        "no idle limit"
    );

    // The touch is conditional: no write inside the throttle, a write outside.
    let now = Utc::now().naive_utc();
    let seen = last_seen(&mut conn, "active").unwrap();
    assert!(
        !repo
            .touch_session(&hash("active"), now, seen - Duration::seconds(1))
            .unwrap()
    );
    assert_eq!(last_seen(&mut conn, "active"), Some(seen));
    assert!(
        repo.touch_session(&hash("active"), now, now - Duration::seconds(60))
            .unwrap()
    );
    // PostgreSQL keeps microseconds.
    let stored = last_seen(&mut conn, "active").unwrap();
    assert!(
        (stored - now).num_microseconds().unwrap().abs() < 1,
        "{stored} vs {now}"
    );

    // The sweep removes every session past a limit and keeps the active one.
    assert_eq!(repo.purge_expired_sessions(&cutoffs).unwrap(), 3);
    assert!(last_seen(&mut conn, "active").is_some());
    for token in ["idle", "old", "expired"] {
        assert!(last_seen(&mut conn, token).is_none(), "{token} purged");
    }

    conn.batch_execute("TRUNCATE users RESTART IDENTITY CASCADE")
        .unwrap();
    wait_for_quiet_pool(&repo);
}

/// r2d2 opens connections in the background to keep `min_idle` after
/// checkouts. Exiting while one is still being established can crash in
/// libpq / OpenSSL teardown (seen in CI as SIGSEGV after the test passed), so
/// wait until every connection the pool counts is established and idle.
fn wait_for_quiet_pool(repo: &DieselRepo) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while std::time::Instant::now() < deadline {
        let s = repo.pool_stats();
        if s.available == s.current_size && s.available >= s.min_idle {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

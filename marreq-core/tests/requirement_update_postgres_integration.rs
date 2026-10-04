// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Custom-field values across requirement versions against PostgreSQL
//! (issue #369): an update that does not send custom fields keeps them on the
//! new version; one that sends them replaces them. Set
//! `MARREQ_TEST_DATABASE_URL` to a migrated, disposable database; the test
//! truncates application data.

#![cfg(not(feature = "test-helpers"))]

use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use marreq_core::models::{CustomFieldValueInput, NewRequirement};
use marreq_core::repository::{CustomFieldRepository, DieselRepo, RequirementsRepository};

#[test]
fn requirement_update_keeps_or_replaces_custom_field_values() {
    let Ok(database_url) = std::env::var("MARREQ_TEST_DATABASE_URL") else {
        eprintln!(
            "skipping PostgreSQL requirement update test: set MARREQ_TEST_DATABASE_URL to a disposable migrated database to run it"
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
        "TRUNCATE users, projects RESTART IDENTITY CASCADE;
         INSERT INTO users (id,username,name,email,password_hash,is_admin) VALUES (1,'rita','Rita','rita@example.test','!',false);
         INSERT INTO projects (id,name,description,status,owner_id,slug) VALUES (1,'P','','active',1,'p');
         INSERT INTO requirement_status (id,title,description,tag,project_id) VALUES (1,'Draft','','D',1);
         INSERT INTO categories (id,title,description,tag,project_id) VALUES (1,'General','','G',1);
         INSERT INTO applicability (id,title,description,tag,project_id) VALUES (1,'All','','A',1);
         INSERT INTO custom_field_definitions (id,project_id,label,field_type,sort_order) VALUES (1,1,'Priority','text',0), (2,1,'Owner','text',1);
         INSERT INTO requirements (id,project_id,stable_code) VALUES (1,1,'REQ-1');
         INSERT INTO requirement_versions (id,requirement_id,title,description,status_id,author_id,reviewer_id,category_id,applicability_id,created_at,approval_state)
           VALUES (10,1,'Power','The system shall.',1,1,1,1,1,now(),'draft');
         UPDATE requirements SET current_version_id = 10 WHERE id = 1;
         INSERT INTO custom_field_values (requirement_version_id,custom_field_definition_id,value) VALUES (10,1,'High'), (10,2,NULL);",
    )
    .expect("seed");

    marreq_core::repository::diesel_repo::init_connection_pool().expect("pool");
    let mut repo = DieselRepo::new().expect("repo");
    let edit = |justification: &str| NewRequirement {
        id: Some(1),
        title: "Power".into(),
        description: "The system shall.".into(),
        author_id: 1,
        category_id: 1,
        status_id: 1,
        reference_code: "REQ-1".into(),
        reviewer_id: 1,
        applicability_id: 1,
        justification: Some(justification.into()),
        project_id: 1,
    };
    let values_of = |repo: &DieselRepo, version_id: i32| {
        let mut values: Vec<(i32, Option<String>)> = repo
            .get_custom_field_values_for_version(version_id)
            .expect("custom field values")
            .into_iter()
            .map(|v| (v.field_id, v.value))
            .collect();
        values.sort();
        values
    };

    // No custom fields in the update: the new version keeps them, NULLs included.
    let updated = repo
        .update_requirement_atomic(
            1,
            &edit("Budget"),
            &[],
            None,
            None,
            "Requirement updated",
            1,
        )
        .expect("update without custom fields");
    let v2 = updated.current_version_id.expect("new version");
    assert_ne!(v2, 10);
    assert_eq!(
        values_of(&repo, v2),
        vec![(1, Some("High".into())), (2, None)]
    );
    assert_eq!(
        values_of(&repo, 10),
        vec![(1, Some("High".into())), (2, None)],
        "the old version keeps its own values"
    );

    // Custom fields in the update replace them.
    let replaced = [CustomFieldValueInput {
        field_id: 2,
        value: Some("Rita".into()),
    }];
    let updated = repo
        .update_requirement_atomic(
            1,
            &edit("Budget 2"),
            &[],
            Some(&replaced),
            None,
            "Requirement updated",
            1,
        )
        .expect("update with custom fields");
    let v3 = updated.current_version_id.expect("new version");
    assert_eq!(values_of(&repo, v3), vec![(2, Some("Rita".into()))]);

    // An explicitly empty list clears them.
    let updated = repo
        .update_requirement_atomic(
            1,
            &edit("Budget 3"),
            &[],
            Some(&[]),
            None,
            "Requirement updated",
            1,
        )
        .expect("update clearing custom fields");
    assert!(values_of(&repo, updated.current_version_id.unwrap()).is_empty());

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

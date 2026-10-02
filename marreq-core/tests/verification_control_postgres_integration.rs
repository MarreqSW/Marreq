// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Verification control data against PostgreSQL (issue #353): upserts of
//! `verification_control` and `requirement_compliance`, their constraints and
//! the `verification_status.outcome` default. Set `MARREQ_TEST_DATABASE_URL`
//! to a migrated, disposable database; the test truncates application data.

#![cfg(not(feature = "test-helpers"))]

use chrono::Utc;
use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use marreq_core::models::{NewVerificationStatus, RequirementCompliance, VerificationControl};
use marreq_core::repository::{DieselRepo, LookupRepository, VerificationControlRepository};

#[test]
fn verification_control_and_compliance_round_trip() {
    let Ok(database_url) = std::env::var("MARREQ_TEST_DATABASE_URL") else {
        eprintln!(
            "skipping PostgreSQL verification control test: set MARREQ_TEST_DATABASE_URL to a disposable migrated database to run it"
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
         INSERT INTO verification_status (id,title,description,tag,project_id) VALUES (100,'Pending','','P',1);
         INSERT INTO categories (id,title,description,tag,project_id) VALUES (1,'General','','G',1);
         INSERT INTO requirements (id,project_id,stable_code) VALUES (1,1,'REQ-1');
         INSERT INTO verifications (id,name,reference_code,description,source,status_id,project_id,author_id,reviewer_id)
           VALUES (1,'Thermal vacuum','VER-1','','',100,1,1,1);",
    )
    .expect("seed");

    marreq_core::repository::diesel_repo::init_connection_pool().expect("pool");
    let mut repo = DieselRepo::new().expect("repo");
    let now = Utc::now().naive_utc();

    // Status outcome: SQL default, explicit value and inference on create.
    let pending = repo.get_verification_status_by_id(100).expect("status");
    assert_eq!(pending.outcome, "not_run");
    let passed = repo
        .create_verification_status(&NewVerificationStatus {
            id: None,
            title: "Passed".into(),
            description: String::new(),
            tag: "OK".into(),
            project_id: 1,
            is_system: false,
            tag_color: None,
            outcome: None,
        })
        .expect("create status");
    assert_eq!(
        repo.get_verification_status_by_id(passed).unwrap().outcome,
        "passed"
    );

    // Verification control: insert, then replace (nulls clear fields).
    assert_eq!(repo.get_verification_control(1).unwrap(), None);
    let mut control = VerificationControl {
        verification_id: 1,
        project_id: 1,
        verification_level: Some("Subsystem".into()),
        verification_stage: Some("QUAL".into()),
        evidence_reference: Some("TR-001".into()),
        updated_by: Some(1),
        updated_at: now,
    };
    repo.upsert_verification_control(&control)
        .expect("insert control");
    control.verification_level = None;
    control.evidence_reference = Some("TR-002".into());
    let stored = repo
        .upsert_verification_control(&control)
        .expect("replace control");
    assert_eq!(stored.verification_level, None, "None is written as NULL");
    assert_eq!(stored.evidence_reference.as_deref(), Some("TR-002"));
    assert_eq!(
        repo.list_verification_control_by_project(1).unwrap().len(),
        1
    );
    assert!(
        repo.list_verification_control_by_project(2)
            .unwrap()
            .is_empty()
    );

    // Requirement compliance: set, replace, clear.
    let mut compliance = RequirementCompliance {
        requirement_id: 1,
        project_id: 1,
        compliance: "PC".into(),
        note: Some("RFW-1".into()),
        set_by: Some(1),
        set_at: now,
    };
    repo.set_requirement_compliance(&compliance).expect("set");
    compliance.compliance = "C".into();
    compliance.note = None;
    let stored = repo
        .set_requirement_compliance(&compliance)
        .expect("replace");
    assert_eq!(stored.compliance.trim(), "C");
    assert_eq!(stored.note, None);
    assert_eq!(
        repo.list_requirement_compliance_by_project(1)
            .unwrap()
            .len(),
        1
    );
    assert!(repo.clear_requirement_compliance(1).unwrap());
    assert!(!repo.clear_requirement_compliance(1).unwrap());
    assert_eq!(repo.get_requirement_compliance(1).unwrap(), None);

    // The database rejects values the API would never send.
    compliance.compliance = "X".into();
    assert!(repo.set_requirement_compliance(&compliance).is_err());
    let bad_outcome =
        conn.batch_execute("UPDATE verification_status SET outcome = 'maybe' WHERE id = 100");
    assert!(bad_outcome.is_err());

    // Rows follow their verification / requirement.
    repo.set_requirement_compliance(&RequirementCompliance {
        compliance: "NC".into(),
        ..compliance
    })
    .expect("set again");
    conn.batch_execute("DELETE FROM matrix; DELETE FROM verifications WHERE id = 1; DELETE FROM requirements WHERE id = 1;")
        .expect("delete parents");
    assert_eq!(repo.get_verification_control(1).unwrap(), None);
    assert_eq!(repo.get_requirement_compliance(1).unwrap(), None);
}

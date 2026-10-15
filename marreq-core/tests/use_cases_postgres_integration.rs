// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Use cases against PostgreSQL (issue #398): the unique code per project,
//! the draft's revision check, publishing and version numbers, approval
//! transitions and cascading deletes. Set `MARREQ_TEST_DATABASE_URL` to a
//! migrated, disposable database; the test truncates application data.

#![cfg(not(feature = "test-helpers"))]

use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use marreq_core::models::{NewUseCase, NewUseCaseLink, NewUseCaseVersion};
use marreq_core::repository::errors::RepoError;
use marreq_core::repository::{
    DieselRepo, MatrixRepository, RequirementsRepository, UseCasesRepository,
};
use serde_json::json;

#[test]
fn use_case_drafts_versions_and_approval() {
    let Ok(database_url) = std::env::var("MARREQ_TEST_DATABASE_URL") else {
        eprintln!(
            "skipping PostgreSQL use case test: set MARREQ_TEST_DATABASE_URL to a disposable migrated database to run it"
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
         INSERT INTO projects (id,name,description,status,owner_id,slug) VALUES (1,'P','','active',1,'p'),(2,'Q','','active',1,'q');
         INSERT INTO requirement_status (id,title,description,tag,project_id) VALUES (1,'Draft','','D',1);
         INSERT INTO categories (id,title,description,tag,project_id) VALUES (1,'General','','G',1);
         INSERT INTO applicability (id,title,description,tag,project_id) VALUES (1,'All','','A',1);
         INSERT INTO requirements (id,project_id,stable_code) VALUES (1,1,'REQ-1');
         INSERT INTO requirement_versions (id,requirement_id,title,description,status_id,author_id,reviewer_id,category_id,applicability_id)
           VALUES (1,1,'Export','text',1,1,1,1,1);
         UPDATE requirements SET current_version_id = 1 WHERE id = 1;
         INSERT INTO verification_status (id,title,description,tag,project_id) VALUES (1,'Pending','','P',1);
         INSERT INTO verifications (id,name,reference_code,description,source,status_id,project_id,author_id,reviewer_id)
           VALUES (1,'Round trip','VER-1','','',1,1,1,1);",
    )
    .expect("seed");

    marreq_core::repository::diesel_repo::init_connection_pool().expect("pool");
    let mut repo = DieselRepo::new().expect("repo");

    let new = |project_id: i32| NewUseCase {
        project_id,
        reference_code: "UC-001".into(),
        created_by: Some(1),
    };
    let doc = json!({ "schema": "marreq.use-case.v1", "title": "Export" });
    let (use_case, draft) = repo.create_use_case(&new(1), &doc).expect("create");
    assert_eq!(draft.revision, 1);
    assert_eq!(draft.use_case_id, use_case.id);
    assert!(matches!(
        repo.create_use_case(&new(1), &doc),
        Err(RepoError::Duplicate(_))
    ));
    repo.create_use_case(&new(2), &doc)
        .expect("the same code in another project");

    // The draft only moves forward from its current revision.
    let doc2 = json!({ "schema": "marreq.use-case.v1", "title": "Export v2" });
    let saved = repo
        .save_use_case_draft(use_case.id, Some(1), None, &doc2, 1)
        .expect("save")
        .expect("revision 1 is current");
    assert_eq!(saved.revision, 2);
    assert!(
        repo.save_use_case_draft(use_case.id, Some(1), None, &doc2, 1)
            .expect("stale save")
            .is_none()
    );
    assert!(
        repo.save_use_case_draft(use_case.id, None, None, &doc2, 1)
            .expect("second draft")
            .is_none(),
        "there is already a draft"
    );

    // Publishing turns the draft into version 1 and removes it.
    let version = |title: &str| NewUseCaseVersion {
        use_case_id: use_case.id,
        document: json!({ "title": title }),
        title: title.into(),
        primary_actor: Some("Engineer".into()),
        change_note: None,
        created_by: 1,
    };
    assert!(
        repo.publish_use_case_draft(1, &version("Export v2"))
            .expect("stale publish")
            .is_none()
    );
    let v1 = repo
        .publish_use_case_draft(2, &version("Export v2"))
        .expect("publish")
        .expect("revision 2 is current");
    assert_eq!(
        (v1.version_number, v1.approval_state.as_str()),
        (1, "draft")
    );
    assert!(repo.get_use_case_draft(use_case.id).unwrap().is_none());
    assert_eq!(
        repo.get_use_case(use_case.id).unwrap().current_version_id,
        Some(v1.id)
    );

    // A new draft based on version 1, then version 2.
    repo.save_use_case_draft(use_case.id, None, Some(v1.id), &doc2, 1)
        .unwrap()
        .expect("new draft");
    let v2 = repo
        .publish_use_case_draft(1, &version("Export v3"))
        .unwrap()
        .expect("published");
    assert_eq!(v2.version_number, 2);
    let numbers: Vec<i32> = repo
        .list_use_case_versions(use_case.id)
        .unwrap()
        .iter()
        .map(|v| v.version_number)
        .collect();
    assert_eq!(numbers, vec![2, 1]);
    assert_eq!(
        repo.list_current_use_case_versions(1).unwrap(),
        vec![v2.clone()]
    );

    // Approval: draft → reviewed → approved; skipping a step is refused.
    assert!(matches!(
        repo.set_use_case_version_approval(v2.id, "approved", 1),
        Err(RepoError::BadInput(_))
    ));
    let reviewed = repo
        .set_use_case_version_approval(v2.id, "reviewed", 1)
        .unwrap();
    assert_eq!(reviewed.reviewed_by, Some(1));
    let approved = repo
        .set_use_case_version_approval(v2.id, "approved", 1)
        .unwrap();
    assert_eq!(approved.approval_state, "approved");
    assert!(approved.approved_at.is_some());

    // Links: one per element and target, suspect when the requirement changes.
    let link = |element: Option<&str>, requirement: Option<i32>, verification: Option<i32>| {
        NewUseCaseLink {
            project_id: 1,
            use_case_id: use_case.id,
            element_id: element.map(str::to_string),
            requirement_id: requirement,
            verification_id: verification,
            linked_requirement_version_id: requirement.map(|_| 1),
            rationale: None,
            created_by: Some(1),
        }
    };
    let step_link = repo
        .create_use_case_link(&link(Some("s_A"), Some(1), None))
        .expect("step link");
    repo.create_use_case_link(&link(None, Some(1), None))
        .expect("use case link to the same requirement");
    repo.create_use_case_link(&link(Some("s_A"), None, Some(1)))
        .expect("verification link");
    assert!(matches!(
        repo.create_use_case_link(&link(Some("s_A"), Some(1), None)),
        Err(RepoError::Duplicate(_))
    ));
    assert!(
        repo.create_use_case_link(&link(Some("s_B"), Some(1), Some(1)))
            .is_err(),
        "exactly one target"
    );
    repo.mark_links_suspect_for_requirement(1, "Requirement updated", Some(1), Some(1))
        .expect("mark");
    let links = repo.list_use_case_links(use_case.id).unwrap();
    assert_eq!(
        links.iter().map(|l| l.suspect).collect::<Vec<_>>(),
        vec![true, true, false],
        "only requirement links go suspect"
    );
    assert!(
        repo.clear_use_case_link_suspect(step_link.id, 1, Some(1))
            .unwrap()
    );
    assert!(
        !repo
            .clear_use_case_link_suspect(step_link.id, 1, Some(1))
            .unwrap()
    );
    repo.set_requirement_version_approval(1, "reviewed", 1)
        .expect("approval");
    assert!(repo.get_use_case_link(step_link.id).unwrap().suspect);
    assert_eq!(
        repo.list_use_case_links_for_requirement(1).unwrap().len(),
        2
    );
    assert_eq!(
        repo.list_use_case_links_for_verification(1).unwrap().len(),
        1
    );
    let removed = repo
        .delete_use_case_links_outside(use_case.id, &["s_B".to_string()])
        .unwrap();
    assert_eq!(
        removed.len(),
        2,
        "the s_A links go, the use case link stays"
    );
    assert_eq!(repo.list_use_case_links_by_project(1).unwrap().len(), 1);

    // Deleting the use case removes its versions and draft.
    repo.save_use_case_draft(use_case.id, None, Some(v2.id), &doc2, 1)
        .unwrap();
    assert!(repo.delete_use_case(use_case.id).unwrap());
    assert!(matches!(
        repo.get_use_case(use_case.id),
        Err(RepoError::NotFound)
    ));
    assert!(repo.list_use_case_versions(use_case.id).unwrap().is_empty());
    assert!(repo.get_use_case_draft(use_case.id).unwrap().is_none());
    assert!(repo.list_use_case_links_by_project(1).unwrap().is_empty());
    assert_eq!(repo.list_use_cases_by_project(2).unwrap().len(), 1);
}

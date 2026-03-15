// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

use marreq::api::document_imports;
use marreq::app::AppState;
use marreq::auth::session::SESSION_COOKIE;
use marreq::models::*;
use marreq::repository::{
    diesel_repo_mock::DieselRepoMock, CacheRepository, MatrixRepository, RequirementsRepository,
    VerificationsRepository,
};
use marreq::services::{CommitRequest, DocumentImportService, ImportSession, ReviewPatch};
use marreq::status_enums::ProjectStatus;
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;
use rocket::routes;
use serde_json::{json, Value};
use std::io::Write;
use std::sync::{Arc, RwLock};
use zip::write::SimpleFileOptions;

type TestState = AppState<CacheRepository<DieselRepoMock>>;

fn timestamp() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

fn managed_state(repo: DieselRepoMock) -> TestState {
    AppState {
        repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
    }
}

fn session_cookie(user_id: i32) -> Cookie<'static> {
    let mut cookie = Cookie::new(SESSION_COOKIE, user_id.to_string());
    cookie.set_path("/");
    cookie
}

fn base_repo() -> DieselRepoMock {
    let mut repo = DieselRepoMock::default();

    let mut admin = DieselRepoMock::make_user(1, "admin", "password");
    admin.is_admin = true;
    repo.users.insert(1, admin);

    let user = DieselRepoMock::make_user(2, "importer", "password");
    repo.users.insert(2, user);

    let reviewer = DieselRepoMock::make_user(3, "reviewer", "password");
    repo.users.insert(3, reviewer);

    repo.projects.insert(
        1,
        Project {
            id: 1,
            name: "Doc Import".into(),
            description: Some("Doc import project".into()),
            creation_date: Some(timestamp()),
            update_date: Some(timestamp()),
            status: ProjectStatus::Active,
            owner_id: Some(1),
        },
    );

    repo.project_members.push(ProjectMember {
        project_id: 1,
        user_id: 2,
        role: marreq::permissions::ROLE_AUTHOR,
        created_at: timestamp(),
        updated_at: timestamp(),
    });
    repo.project_members.push(ProjectMember {
        project_id: 1,
        user_id: 3,
        role: marreq::permissions::ROLE_REVIEWER,
        created_at: timestamp(),
        updated_at: timestamp(),
    });

    repo.requirement_statuses.insert(
        1,
        RequirementStatus {
            id: 1,
            title: "Draft".into(),
            description: "".into(),
            tag: "draft".into(),
            project_id: 1,
            is_system: true,
            tag_color: None,
        },
    );
    repo.verification_statuses.insert(
        2,
        VerificationStatus {
            id: 2,
            title: "Pending".into(),
            description: "".into(),
            tag: "pending".into(),
            project_id: 1,
            is_system: true,
            tag_color: None,
        },
    );
    repo.categories.insert(
        1,
        Category {
            id: 1,
            title: "Safety".into(),
            description: "".into(),
            tag: "SAFETY".into(),
            project_id: 1,
        },
    );
    repo.applicability.insert(
        1,
        Applicability {
            id: 1,
            title: "All".into(),
            description: "".into(),
            tag: "ALL".into(),
            project_id: 1,
        },
    );
    repo.verification_methods.insert(
        1,
        VerificationMethod {
            id: 1,
            title: "Analysis".into(),
            description: "".into(),
            tag: "ANALYSIS".into(),
            project_id: 1,
        },
    );

    repo
}

fn docx_bytes() -> Vec<u8> {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>REQ-001: The system shall log faults.</w:t></w:r></w:p>
    <w:p><w:r><w:t>TEST-1: Fault logging test verifies the requirement.</w:t></w:r></w:p>
    <w:p><w:r><w:t>REQ-001 is verified by TEST-1.</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = SimpleFileOptions::default();
        writer.start_file("[Content_Types].xml", options).unwrap();
        writer
            .write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
            )
            .unwrap();
        writer.start_file("_rels/.rels", options).unwrap();
        writer
            .write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
            )
            .unwrap();
        writer.start_file("word/document.xml", options).unwrap();
        writer.write_all(xml.as_bytes()).unwrap();
        writer.finish().unwrap();
    }
    cursor.into_inner()
}

async fn session_fixture() -> (Client, String, TestState) {
    let state = managed_state(base_repo());
    let service = DocumentImportService::new(&state);
    let session = service
        .create_session_from_bytes(
            1,
            &DieselRepoMock::make_user(2, "importer", ""),
            "sample.docx",
            &docx_bytes(),
            false,
        )
        .await
        .unwrap();
    let rocket = rocket::build().manage(state.clone()).mount(
        "/api",
        routes![
            document_imports::get,
            document_imports::patch,
            document_imports::commit,
            document_imports::delete
        ],
    );
    let client = Client::tracked(rocket).await.unwrap();
    (client, session.session_id, state)
}

#[rocket::async_test]
async fn get_returns_dry_run_session_without_db_writes() {
    let (client, session_id, state) = session_fixture().await;
    let response = client
        .get(format!("/api/projects/1/document_imports/{session_id}"))
        .private_cookie(session_cookie(2))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let session: ImportSession = response.into_json().await.unwrap();
    assert_eq!(session.summary.requirement_candidates, 1);
    assert_eq!(session.summary.verification_candidates, 1);

    let repo = state.repo.read().unwrap();
    assert!(repo.get_requirements_by_project(1).unwrap().is_empty());
    assert!(repo.get_verifications_by_project(1).unwrap().is_empty());
}

#[rocket::async_test]
async fn get_returns_not_found_for_missing_session() {
    let state = managed_state(base_repo());
    let rocket = rocket::build().manage(state).mount(
        "/api",
        routes![
            document_imports::get,
            document_imports::patch,
            document_imports::commit,
            document_imports::delete
        ],
    );
    let client = Client::tracked(rocket).await.unwrap();

    let response = client
        .get("/api/projects/1/document_imports/missing-session")
        .private_cookie(session_cookie(2))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn patch_persists_review_changes() {
    let (client, session_id, _state) = session_fixture().await;

    let session_before: ImportSession = client
        .get(format!("/api/projects/1/document_imports/{session_id}"))
        .private_cookie(session_cookie(2))
        .dispatch()
        .await
        .into_json()
        .await
        .unwrap();
    let requirement_id = session_before.candidates.requirements[0].id.clone();

    let patch_payload = ReviewPatch {
        defaults: Some(marreq::services::ImportDefaults {
            reviewer_id: Some(3),
            category_id: Some(1),
            applicability_id: Some(1),
            verification_status_id: Some(2),
            verification_source: Some("patched.docx".into()),
        }),
        requirements: Some(vec![marreq::services::RequirementReviewPatch {
            candidate_id: requirement_id.clone(),
            title: Some("Patched imported requirement".into()),
            include: Some(false),
            ..Default::default()
        }]),
        verifications: None,
        trace_links: None,
        requirement_links: None,
    };

    let patch_response = client
        .patch(format!("/api/projects/1/document_imports/{session_id}"))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(2))
        .body(serde_json::to_string(&patch_payload).unwrap())
        .dispatch()
        .await;
    assert_eq!(patch_response.status(), Status::Ok);

    let session_after: ImportSession = client
        .get(format!("/api/projects/1/document_imports/{session_id}"))
        .private_cookie(session_cookie(2))
        .dispatch()
        .await
        .into_json()
        .await
        .unwrap();

    assert_eq!(
        session_after
            .review_state
            .defaults
            .verification_source
            .as_deref(),
        Some("patched.docx")
    );
    let requirement = session_after
        .candidates
        .requirements
        .iter()
        .find(|candidate| candidate.id == requirement_id)
        .unwrap();
    assert_eq!(requirement.title, "Patched imported requirement");
    assert!(!requirement.include);
}

#[rocket::async_test]
async fn commit_requires_reviewer_and_confirmation() {
    let (client, session_id, _state) = session_fixture().await;

    let commit_without_patch = client
        .post(format!(
            "/api/projects/1/document_imports/{session_id}/commit"
        ))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(2))
        .body(json!(CommitRequest { confirm: true }).to_string())
        .dispatch()
        .await;
    assert_eq!(commit_without_patch.status(), Status::BadRequest);

    let patch_payload = ReviewPatch {
        defaults: Some(marreq::services::ImportDefaults {
            reviewer_id: Some(3),
            category_id: Some(1),
            applicability_id: Some(1),
            verification_status_id: Some(2),
            verification_source: Some("sample.docx".into()),
        }),
        requirements: None,
        verifications: None,
        trace_links: None,
        requirement_links: None,
    };
    let patch_response = client
        .patch(format!("/api/projects/1/document_imports/{session_id}"))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(2))
        .body(serde_json::to_string(&patch_payload).unwrap())
        .dispatch()
        .await;
    assert_eq!(patch_response.status(), Status::Ok);

    let commit_without_confirm = client
        .post(format!(
            "/api/projects/1/document_imports/{session_id}/commit"
        ))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(2))
        .body(json!(CommitRequest { confirm: false }).to_string())
        .dispatch()
        .await;
    assert_eq!(commit_without_confirm.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn commit_creates_records_after_confirm() {
    let (client, session_id, state) = session_fixture().await;

    let patch_payload = ReviewPatch {
        defaults: Some(marreq::services::ImportDefaults {
            reviewer_id: Some(3),
            category_id: Some(1),
            applicability_id: Some(1),
            verification_status_id: Some(2),
            verification_source: Some("sample.docx".into()),
        }),
        requirements: None,
        verifications: None,
        trace_links: None,
        requirement_links: None,
    };
    let patch_response = client
        .patch(format!("/api/projects/1/document_imports/{session_id}"))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(2))
        .body(serde_json::to_string(&patch_payload).unwrap())
        .dispatch()
        .await;
    assert_eq!(patch_response.status(), Status::Ok);

    let commit_response = client
        .post(format!(
            "/api/projects/1/document_imports/{session_id}/commit"
        ))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(2))
        .body(json!(CommitRequest { confirm: true }).to_string())
        .dispatch()
        .await;
    assert_eq!(commit_response.status(), Status::Ok);

    let payload: Value = commit_response.into_json().await.unwrap();
    assert_eq!(payload["status"], "ok");
    assert!(
        payload["result"]["created_requirement_ids"]
            .as_array()
            .unwrap()
            .len()
            >= 1
    );

    let repo = state.repo.read().unwrap();
    assert_eq!(repo.get_requirements_by_project(1).unwrap().len(), 1);
    assert_eq!(repo.get_verifications_by_project(1).unwrap().len(), 1);
    assert_eq!(repo.get_matrix_by_project(1).unwrap().len(), 1);
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

//! Integration tests for MCP Phase 2 (draft_write) project-scoped write endpoints.
//!
//! Tests:
//! - POST /api/projects/<id>/requirements (create_by_project)
//! - PATCH /api/projects/<id>/requirements/<id> (patch_by_project)
//! - PUT /api/projects/<id>/requirements/<req_id>/versions/<version_id>/approval (set_version_approval_by_project)
//! - POST /api/projects/<id>/baselines (create with ProjectAccessOrBearer)

use marreq::models::*;
use marreq::status_enums::ProjectStatus;
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;
use serde_json::{json, Value};

mod test_support {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use marreq::app::AppState;
    use marreq::auth::session::SESSION_COOKIE;
    use marreq::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use std::sync::{Arc, RwLock};

    pub type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

    pub fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    pub fn managed_state(repo: DieselRepoMock) -> TestAppState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    pub async fn test_client(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(managed_state(repo))
            .mount("/api", marreq::api::routes());

        Client::tracked(rocket).await.expect("rocket instance")
    }

    pub fn session_cookie(user_id: i32) -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, user_id.to_string());
        cookie.set_path("/");
        cookie
    }

    /// Repo with project 1, admin (user 1) as owner, user 2 as member with role 2 (manager), user 3 as member with role 3 (cannot approve).
    pub fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        let mut admin = DieselRepoMock::make_user(1, "admin", "password");
        admin.is_admin = true;
        repo.users.insert(1, admin);

        repo.users
            .insert(2, DieselRepoMock::make_user(2, "user2", "password"));
        repo.users
            .insert(3, DieselRepoMock::make_user(3, "user3", "password"));

        repo.projects.insert(
            1,
            Project {
                id: 1,
                name: "Test Project".into(),
                description: Some("Description".into()),
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(1),
            },
        );

        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 1,
            role: 1, // Owner
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 2,
            role: 2, // Manager (can approve)
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 3,
            role: 3, // Member (cannot approve)
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                id: 1,
                title: "Draft".into(),
                description: "".into(),
                tag: "D".into(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            },
        );

        repo.categories.insert(
            1,
            Category {
                id: 1,
                title: "Cat".into(),
                description: "".into(),
                tag: "CAT".into(),
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

        repo.verifications.insert(
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

    pub fn create_requirement_payload(project_id: i32, reference_code: &str) -> Value {
        json!({
            "title": "New Req",
            "description": "Description",
            "reference_code": reference_code,
            "author_id": 1,
            "reviewer_id": 1,
            "category_id": 1,
            "status_id": 1,
            "applicability_id": 1,
            "project_id": project_id,
            "parent_id": null,
            "justification": null,
            "verification_method_ids": [1],
            "custom_fields": []
        })
    }
}

use test_support::*;

const PROJECT_ID: i32 = 1;

// ============================================================================
// create_by_project
// ============================================================================

#[rocket::async_test]
async fn create_by_project_returns_ok_and_id() {
    let client = test_client(base_repo()).await;

    let response = client
        .post(format!("/api/projects/{PROJECT_ID}/requirements"))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(create_requirement_payload(PROJECT_ID, "REQ-MCP-001").to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let body: Value = response.into_json().await.expect("json");
    assert_eq!(body["status"], "ok");
    assert!(body["id"].as_i64().unwrap() >= 1);
}

#[rocket::async_test]
async fn create_by_project_rejects_when_payload_project_id_mismatch() {
    let client = test_client(base_repo()).await;

    let response = client
        .post(format!("/api/projects/{PROJECT_ID}/requirements"))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(create_requirement_payload(999, "REQ-001").to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn create_by_project_rejects_empty_verification_method_ids() {
    let client = test_client(base_repo()).await;

    let mut payload = create_requirement_payload(PROJECT_ID, "REQ-002");
    payload["verification_method_ids"] = json!([]);

    let response = client
        .post(format!("/api/projects/{PROJECT_ID}/requirements"))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn create_by_project_requires_auth() {
    let client = test_client(base_repo()).await;

    let response = client
        .post(format!("/api/projects/{PROJECT_ID}/requirements"))
        .header(ContentType::JSON)
        .body(create_requirement_payload(PROJECT_ID, "REQ-003").to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// patch_by_project
// ============================================================================

#[rocket::async_test]
async fn patch_by_project_updates_requirement() {
    let mut repo = base_repo();
    repo.requirements.insert(
        1,
        Requirement {
            id: 1,
            current_version_id: Some(1),
            same_as_current: None,
            title: "Original".into(),
            description: "Desc".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: "REQ-001".into(),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            applicability_id: 1,
            justification: None,
            project_id: PROJECT_ID,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );
    repo.requirement_versions.insert(
        1,
        RequirementVersion {
            id: 1,
            requirement_id: 1,
            title: "Original".into(),
            description: "Desc".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            category_id: 1,
            applicability_id: 1,
            justification: None,
            deadline_date: Some(timestamp()),
            created_at: timestamp(),
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
        },
    );
    repo.requirement_verification_methods.push((1, 1));

    let client = test_client(repo).await;

    let response = client
        .patch(format!("/api/projects/{PROJECT_ID}/requirements/1"))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(json!({ "title": "Patched Title" }).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let body: Value = response.into_json().await.expect("json");
    assert_eq!(body["success"], true);

    let get_resp = client
        .get(format!("/api/projects/{PROJECT_ID}/requirements/1"))
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;
    assert_eq!(get_resp.status(), Status::Ok);
    let req: Value = get_resp.into_json().await.expect("json");
    // get_by_project returns RequirementWithTraceSummary with flattened requirement
    assert_eq!(req["title"], "Patched Title");
}

#[rocket::async_test]
async fn patch_by_project_returns_404_when_requirement_not_in_project() {
    let mut repo = base_repo();
    repo.requirements.insert(
        1,
        Requirement {
            id: 1,
            current_version_id: Some(1),
            same_as_current: None,
            title: "Other".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: "R".into(),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: None,
            applicability_id: 1,
            justification: None,
            project_id: 999, // different project
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );
    repo.requirement_versions.insert(
        1,
        RequirementVersion {
            id: 1,
            requirement_id: 1,
            title: "Other".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            category_id: 1,
            applicability_id: 1,
            justification: None,
            deadline_date: None,
            created_at: timestamp(),
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
        },
    );

    let client = test_client(repo).await;

    let response = client
        .patch(format!("/api/projects/{PROJECT_ID}/requirements/1"))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(json!({ "title": "Patched" }).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn patch_by_project_rejects_empty_patch() {
    let mut repo = base_repo();
    repo.requirements.insert(
        1,
        Requirement {
            id: 1,
            current_version_id: Some(1),
            same_as_current: None,
            title: "R".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: "R".into(),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: None,
            applicability_id: 1,
            justification: None,
            project_id: PROJECT_ID,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );
    repo.requirement_versions.insert(
        1,
        RequirementVersion {
            id: 1,
            requirement_id: 1,
            title: "R".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            category_id: 1,
            applicability_id: 1,
            justification: None,
            deadline_date: None,
            created_at: timestamp(),
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
        },
    );

    let client = test_client(repo).await;

    let response = client
        .patch(format!("/api/projects/{PROJECT_ID}/requirements/1"))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(json!({}).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

// ============================================================================
// set_version_approval_by_project
// ============================================================================

#[rocket::async_test]
async fn set_version_approval_by_project_succeeds_for_owner() {
    let mut repo = base_repo();
    let version_id = 1;
    repo.requirements.insert(
        1,
        Requirement {
            id: 1,
            current_version_id: Some(version_id),
            same_as_current: None,
            title: "R".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: "R".into(),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: None,
            applicability_id: 1,
            justification: None,
            project_id: PROJECT_ID,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );
    repo.requirement_versions.insert(
        version_id,
        RequirementVersion {
            id: version_id,
            requirement_id: 1,
            title: "R".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            category_id: 1,
            applicability_id: 1,
            justification: None,
            deadline_date: None,
            created_at: timestamp(),
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
        },
    );

    let client = test_client(repo).await;

    let response = client
        .put(format!(
            "/api/projects/{PROJECT_ID}/requirements/1/versions/{version_id}/approval"
        ))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(json!({ "state": "reviewed" }).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let version: RequirementVersion = response.into_json().await.expect("json");
    assert_eq!(version.approval_state, "reviewed");
}

#[rocket::async_test]
async fn set_version_approval_by_project_succeeds_for_manager() {
    let mut repo = base_repo();
    let version_id = 1;
    repo.requirements.insert(
        1,
        Requirement {
            id: 1,
            current_version_id: Some(version_id),
            same_as_current: None,
            title: "R".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: "R".into(),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: None,
            applicability_id: 1,
            justification: None,
            project_id: PROJECT_ID,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );
    repo.requirement_versions.insert(
        version_id,
        RequirementVersion {
            id: version_id,
            requirement_id: 1,
            title: "R".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            category_id: 1,
            applicability_id: 1,
            justification: None,
            deadline_date: None,
            created_at: timestamp(),
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
        },
    );

    let client = test_client(repo).await;

    let response = client
        .put(format!(
            "/api/projects/{PROJECT_ID}/requirements/1/versions/{version_id}/approval"
        ))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(2))
        .body(json!({ "state": "reviewed" }).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let version: RequirementVersion = response.into_json().await.expect("json");
    assert_eq!(version.approval_state, "reviewed");
}

#[rocket::async_test]
async fn set_version_approval_by_project_returns_forbidden_for_member_without_role() {
    let mut repo = base_repo();
    let version_id = 1;
    repo.requirements.insert(
        1,
        Requirement {
            id: 1,
            current_version_id: Some(version_id),
            same_as_current: None,
            title: "R".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: "R".into(),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: None,
            applicability_id: 1,
            justification: None,
            project_id: PROJECT_ID,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );
    repo.requirement_versions.insert(
        version_id,
        RequirementVersion {
            id: version_id,
            requirement_id: 1,
            title: "R".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            category_id: 1,
            applicability_id: 1,
            justification: None,
            deadline_date: None,
            created_at: timestamp(),
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
        },
    );

    let client = test_client(repo).await;

    let response = client
        .put(format!(
            "/api/projects/{PROJECT_ID}/requirements/1/versions/{version_id}/approval"
        ))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(3))
        .body(json!({ "state": "reviewed" }).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn set_version_approval_by_project_returns_404_when_requirement_not_in_project() {
    let mut repo = base_repo();
    let version_id = 1;
    repo.requirements.insert(
        1,
        Requirement {
            id: 1,
            current_version_id: Some(version_id),
            same_as_current: None,
            title: "R".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: "R".into(),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: None,
            applicability_id: 1,
            justification: None,
            project_id: 999,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );
    repo.requirement_versions.insert(
        version_id,
        RequirementVersion {
            id: version_id,
            requirement_id: 1,
            title: "R".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            category_id: 1,
            applicability_id: 1,
            justification: None,
            deadline_date: None,
            created_at: timestamp(),
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
        },
    );

    let client = test_client(repo).await;

    let response = client
        .put(format!(
            "/api/projects/{PROJECT_ID}/requirements/1/versions/{version_id}/approval"
        ))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(json!({ "state": "reviewed" }).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn set_version_approval_by_project_rejects_invalid_state() {
    let mut repo = base_repo();
    repo.requirements.insert(
        1,
        Requirement {
            id: 1,
            current_version_id: Some(1),
            same_as_current: None,
            title: "R".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: "R".into(),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: None,
            applicability_id: 1,
            justification: None,
            project_id: PROJECT_ID,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );
    repo.requirement_versions.insert(
        1,
        RequirementVersion {
            id: 1,
            requirement_id: 1,
            title: "R".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            category_id: 1,
            applicability_id: 1,
            justification: None,
            deadline_date: None,
            created_at: timestamp(),
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
        },
    );

    let client = test_client(repo).await;

    let response = client
        .put("/api/projects/1/requirements/1/versions/1/approval")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(json!({ "state": "invalid" }).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

// ============================================================================
// Baseline create (ProjectAccessOrBearer - session still works)
// ============================================================================

#[rocket::async_test]
async fn create_baseline_project_scoped_succeeds_with_session() {
    let client = test_client(base_repo()).await;

    let response = client
        .post(format!("/api/projects/{PROJECT_ID}/baselines"))
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(json!({ "name": "MCP Baseline", "description": "From test" }).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let baseline: Value = response.into_json().await.expect("json");
    assert_eq!(baseline["name"], "MCP Baseline");
    assert!(baseline["id"].as_i64().unwrap() >= 1);
}

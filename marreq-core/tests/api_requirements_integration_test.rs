// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

//! Comprehensive integration tests for Requirements API endpoints.
//!
//! These tests verify the complete behavior of `/api/requirements` endpoints including:
//! - CRUD operations
//! - Error handling (404, 400, 401, 403)
//! - Project scoping
//! - JSON response format validation
//! - Edge cases and validation

use marreq_core::models::*;
use marreq_core::status_enums::ProjectStatus;
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;
use serde_json::{json, Value};

mod test_support {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use marreq_core::app::AppState;
    use marreq_core::auth::session::test_session_cookie_for;
    use marreq_core::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
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
        marreq_core::deployment::install_test_server_mode();
        let rocket = rocket::build()
            .manage(managed_state(repo))
            .manage(marreq_core::auth::rate_limiter::LoginRateLimiter::new())
            .mount("/api", marreq_core::api::routes());

        Client::tracked(rocket).await.expect("rocket instance")
    }

    pub fn session_cookie(client: &Client, user_id: i32) -> Cookie<'static> {
        let state = client
            .rocket()
            .state::<TestAppState>()
            .expect("managed app state");
        test_session_cookie_for(state, user_id)
    }

    pub fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        let mut admin = DieselRepoMock::make_user(1, "admin", "password");
        admin.is_admin = true;
        repo.users.insert(1, admin);

        let regular_user = DieselRepoMock::make_user(2, "user", "password");
        repo.users.insert(2, regular_user);

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
                slug: "test-project".into(),
                group_id: None,
            },
        );

        repo.projects.insert(
            2,
            Project {
                id: 2,
                name: "Other Project".into(),
                description: Some("Other Description".into()),
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(2),
                slug: "other-project".into(),
                group_id: None,
            },
        );

        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 1,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 2,
            role: 1,
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

        repo.requirement_statuses.insert(
            2,
            RequirementStatus {
                id: 2,
                title: "Accepted".into(),
                description: "".into(),
                tag: "A".into(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            },
        );

        repo.categories.insert(
            1,
            Category {
                id: 1,
                title: "Systems".into(),
                description: "".into(),
                tag: "SYS".into(),
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
        repo.verification_methods.insert(
            2,
            VerificationMethod {
                id: 2,
                title: "Test".into(),
                description: "".into(),
                tag: "TEST".into(),
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

        repo
    }

    pub fn sample_requirement(id: i32, project_id: i32, title: &str) -> Requirement {
        Requirement {
            id,
            current_version_id: None,
            same_as_current: None,
            title: title.to_string(),
            description: format!("{} description", title),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: format!("REQ-SYS-{:03}", id),
            category_id: 1,
            parent_id: Some(0),
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            applicability_id: 1,
            justification: Some("Test justification".into()),
            project_id,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        }
    }

    pub fn new_requirement_json(title: &str, project_id: i32) -> Value {
        new_requirement_json_with_verification(title, project_id, &[1])
    }

    pub fn new_requirement_json_with_verification(
        title: &str,
        project_id: i32,
        verification_method_ids: &[i32],
    ) -> Value {
        json!({
            "title": title,
            "description": format!("{} description", title),
            "verification_method_ids": verification_method_ids,
            "author_id": 1,
            "category_id": 1,
            "status_id": 1,
            "parent_id": 0,
            "reference_code": "",
            "reviewer_id": 1,
            "applicability_id": 1,
            "justification": null,
            "project_id": project_id
        })
    }
}

use test_support::*;

// ============================================================================
// GET /api/requirements - List All Requirements
// ============================================================================

#[rocket::async_test]
async fn get_requirements_returns_empty_list_when_no_requirements() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/api/requirements")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let requirements: Vec<Requirement> = response.into_json().await.expect("json");
    assert!(requirements.is_empty());
}

#[rocket::async_test]
async fn get_requirements_returns_all_requirements() {
    let mut repo = base_repo();
    repo.requirements
        .insert(1, sample_requirement(1, 1, "Requirement 1"));
    repo.requirements
        .insert(2, sample_requirement(2, 1, "Requirement 2"));
    repo.requirements
        .insert(3, sample_requirement(3, 1, "Requirement 3"));

    let client = test_client(repo).await;

    let response = client
        .get("/api/requirements")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let requirements: Vec<Requirement> = response.into_json().await.expect("json");
    assert_eq!(requirements.len(), 3);

    // Check all requirements are present (order may vary)
    let titles: Vec<&str> = requirements.iter().map(|r| r.title.as_str()).collect();
    assert!(titles.contains(&"Requirement 1"));
    assert!(titles.contains(&"Requirement 2"));
    assert!(titles.contains(&"Requirement 3"));
}

#[rocket::async_test]
async fn get_requirements_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/requirements").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// GET /api/requirements/{id} - Get Single Requirement
// ============================================================================

#[rocket::async_test]
async fn get_requirement_by_id_returns_correct_requirement() {
    let mut repo = base_repo();
    repo.requirements
        .insert(1, sample_requirement(1, 1, "Test Requirement"));

    let client = test_client(repo).await;

    let response = client
        .get("/api/requirements/1")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let requirement: Requirement = response.into_json().await.expect("json");
    assert_eq!(requirement.id, 1);
    assert_eq!(requirement.title, "Test Requirement");
    assert_eq!(requirement.reference_code, "REQ-SYS-001");
}

#[rocket::async_test]
async fn get_requirement_with_nonexistent_id_returns_404() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/api/requirements/999")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn get_requirement_requires_authentication() {
    let mut repo = base_repo();
    repo.requirements
        .insert(1, sample_requirement(1, 1, "Test Requirement"));

    let client = test_client(repo).await;

    let response = client.get("/api/requirements/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// POST /api/requirements - Create New Requirement
// ============================================================================

#[rocket::async_test]
async fn post_requirement_creates_new_requirement() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 1))
        .body(new_requirement_json("New Requirement", 1).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    assert_eq!(result["status"], "ok");
    assert_eq!(result["id"], 1);
}

#[rocket::async_test]
async fn post_requirement_with_missing_fields_returns_error() {
    let client = test_client(base_repo()).await;

    let invalid_json = json!({
        "req_title": "Incomplete Requirement"
        // Missing required fields
    });

    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 1))
        .body(invalid_json.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::UnprocessableEntity);
}

#[rocket::async_test]
async fn post_requirement_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .body(new_requirement_json("New Requirement", 1).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn post_requirement_creates_with_multiple_verification_method_ids() {
    let client = test_client(base_repo()).await;

    let body = new_requirement_json_with_verification("Multi Verification Req", 1, &[1, 2]);
    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 1))
        .body(body.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    assert_eq!(result["status"], "ok");
    assert_eq!(result["id"], 1);
}

#[rocket::async_test]
async fn post_requirement_rejects_empty_verification_method_ids() {
    let client = test_client(base_repo()).await;

    let body = new_requirement_json_with_verification("No Verification", 1, &[]);
    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 1))
        .body(body.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn post_requirement_with_invalid_json_returns_error() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 1))
        .body("{invalid json}")
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

// ============================================================================
// PATCH /api/requirements/{id} - Partial Update Requirement
// ============================================================================

#[rocket::async_test]
async fn patch_requirement_updates_title() {
    let mut repo = base_repo();
    repo.requirements
        .insert(1, sample_requirement(1, 1, "Original Title"));

    let client = test_client(repo).await;

    let patch = json!({
        "title": "Updated Title"
    });

    let response = client
        .patch("/api/requirements/1")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 1))
        .body(patch.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    assert_eq!(result["success"], true);

    // Verify the update
    let get_response = client
        .get("/api/requirements/1")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    let requirement: Requirement = get_response.into_json().await.expect("json");
    assert_eq!(requirement.title, "Updated Title");
}

#[rocket::async_test]
async fn patch_requirement_updates_multiple_fields() {
    let mut repo = base_repo();
    repo.requirements
        .insert(1, sample_requirement(1, 1, "Original"));

    let client = test_client(repo).await;

    let patch = json!({
        "title": "Updated Title",
        "description": "Updated description",
        "status_id": 2
    });

    let response = client
        .patch("/api/requirements/1")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 1))
        .body(patch.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);

    // Verify all fields were updated
    let get_response = client
        .get("/api/requirements/1")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    let requirement: Requirement = get_response.into_json().await.expect("json");
    assert_eq!(requirement.title, "Updated Title");
    assert_eq!(requirement.description, "Updated description");
    assert_eq!(requirement.status_id, 2);
}

#[rocket::async_test]
async fn patch_requirement_updates_verification_method_ids() {
    let mut repo = base_repo();
    repo.requirements
        .insert(1, sample_requirement(1, 1, "Original"));
    repo.requirement_verification_methods.push((1, 1));

    let client = test_client(repo).await;

    let patch = json!({
        "verification_method_ids": [2]
    });

    let response = client
        .patch("/api/requirements/1")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 1))
        .body(patch.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn patch_requirement_with_empty_patch_returns_bad_request() {
    let mut repo = base_repo();
    repo.requirements
        .insert(1, sample_requirement(1, 1, "Test"));

    let client = test_client(repo).await;

    let response = client
        .patch("/api/requirements/1")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 1))
        .body(json!({}).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn patch_nonexistent_requirement_returns_404() {
    let client = test_client(base_repo()).await;

    let patch = json!({
        "title": "Updated Title"
    });

    let response = client
        .patch("/api/requirements/999")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 1))
        .body(patch.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

// ============================================================================
// DELETE /api/requirements/{id} - Delete Requirement
// ============================================================================

#[rocket::async_test]
async fn delete_requirement_removes_requirement() {
    let mut repo = base_repo();
    repo.requirements
        .insert(1, sample_requirement(1, 1, "To Delete"));

    let client = test_client(repo).await;

    let response = client
        .delete("/api/requirements/1")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NoContent);

    // Verify requirement is gone
    let get_response = client
        .get("/api/requirements/1")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(get_response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn delete_nonexistent_requirement_returns_404() {
    let client = test_client(base_repo()).await;

    let response = client
        .delete("/api/requirements/999")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn delete_requirement_requires_authentication() {
    let mut repo = base_repo();
    repo.requirements
        .insert(1, sample_requirement(1, 1, "To Delete"));

    let client = test_client(repo).await;

    let response = client.delete("/api/requirements/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// Project Scoping Tests
// ============================================================================

#[rocket::async_test]
async fn get_requirements_returns_requirements_from_all_projects() {
    let mut repo = base_repo();
    repo.requirements
        .insert(1, sample_requirement(1, 1, "Project 1 Req"));
    repo.requirements
        .insert(2, sample_requirement(2, 2, "Project 2 Req"));

    let client = test_client(repo).await;

    let response = client
        .get("/api/requirements")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let requirements: Vec<Requirement> = response.into_json().await.expect("json");

    // Should return both projects' requirements
    assert_eq!(requirements.len(), 2);
}

// ============================================================================
// Edge Cases and Validation
// ============================================================================

#[rocket::async_test]
async fn create_requirement_with_very_long_title() {
    let client = test_client(base_repo()).await;

    let long_title = "A".repeat(1000);
    let req_json = new_requirement_json(&long_title, 1);

    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 1))
        .body(req_json.to_string())
        .dispatch()
        .await;

    // Should either accept or reject gracefully
    assert!(response.status() == Status::Ok || response.status() == Status::BadRequest);
}

#[rocket::async_test]
async fn create_multiple_requirements_sequentially() {
    let client = test_client(base_repo()).await;

    for i in 1..=5 {
        let response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(session_cookie(&client, 1))
            .body(new_requirement_json(&format!("Requirement {}", i), 1).to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let result: Value = response.into_json().await.expect("json");
        assert_eq!(result["id"], i);
    }

    // Verify all were created
    let list_response = client
        .get("/api/requirements")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    let requirements: Vec<Requirement> = list_response.into_json().await.expect("json");
    assert_eq!(requirements.len(), 5);
}

#[rocket::async_test]
async fn patch_requirement_preserves_unmodified_fields() {
    let mut repo = base_repo();
    let original = sample_requirement(1, 1, "Original Title");
    let original_description = original.description.clone();
    let original_status = original.status_id;
    repo.requirements.insert(1, original);

    let client = test_client(repo).await;

    // Update only title
    let patch = json!({
        "title": "New Title"
    });

    client
        .patch("/api/requirements/1")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 1))
        .body(patch.to_string())
        .dispatch()
        .await;

    // Verify other fields unchanged
    let get_response = client
        .get("/api/requirements/1")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    let requirement: Requirement = get_response.into_json().await.expect("json");
    assert_eq!(requirement.title, "New Title");
    assert_eq!(requirement.description, original_description);
    assert_eq!(requirement.status_id, original_status);
}

#[rocket::async_test]
async fn list_requirement_version_links_returns_200_and_array() {
    use test_support::*;

    let repo = base_repo();
    let client = test_client(repo).await;

    let response = client
        .get("/api/projects/1/requirement-version-links")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let links: Vec<Value> = response.into_json().await.expect("json");
    assert!(links.is_empty());
}

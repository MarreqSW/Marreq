// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

//! Comprehensive integration tests for Tests API endpoints.
//!
//! These tests verify the complete behavior of `/api/verifications` endpoints including:
//! - CRUD operations
//! - Field update API
//! - Test hierarchy (parent-child)
//! - Test status management
//! - Error handling
//! - Authentication

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
            .manage(marreq::auth::rate_limiter::LoginRateLimiter::new())
            .mount("/api", marreq::api::routes());

        Client::tracked(rocket).await.expect("rocket instance")
    }

    pub fn session_cookie(user_id: i32) -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, user_id.to_string());
        cookie.set_path("/");
        cookie
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

        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 1,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo.verification_statuses.insert(
            1,
            VerificationStatus {
                id: 1,
                title: "Not Run".into(),
                description: "".into(),
                tag: "NR".into(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            },
        );

        repo.verification_statuses.insert(
            2,
            VerificationStatus {
                id: 2,
                title: "Passed".into(),
                description: "".into(),
                tag: "P".into(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            },
        );

        repo.verification_statuses.insert(
            3,
            VerificationStatus {
                id: 3,
                title: "Failed".into(),
                description: "".into(),
                tag: "F".into(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            },
        );

        repo
    }

    pub fn sample_test(id: i32, project_id: i32, name: &str) -> Verification {
        Verification {
            id: id,
            name: name.to_string(),
            reference_code: format!("TST-{:03}", id),
            description: format!("{} description", name),
            source: "automated".into(),
            status_id: 1,
            parent_id: None,
            project_id,
            verification_method_id: None,
        }
    }

    pub fn new_test_json(name: &str, project_id: i32) -> Value {
        json!({
            "name": name,
            "reference_code": "",
            "description": format!("{} description", name),
            "source": "automated",
            "status_id": 1,
            "parent_id": null,
            "project_id": project_id
        })
    }
}

use test_support::*;

// ============================================================================
// GET /api/verifications - List All Verifications
// ============================================================================

#[rocket::async_test]
async fn get_tests_returns_empty_list_when_no_tests() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/api/verifications")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let tests: Vec<Verification> = response.into_json().await.expect("json");
    assert!(tests.is_empty());
}

#[rocket::async_test]
async fn get_tests_returns_all_tests() {
    let mut repo = base_repo();
    repo.verifications.insert(1, sample_test(1, 1, "Test 1"));
    repo.verifications.insert(2, sample_test(2, 1, "Test 2"));
    repo.verifications.insert(3, sample_test(3, 1, "Test 3"));

    let client = test_client(repo).await;

    let response = client
        .get("/api/verifications")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let tests: Vec<Verification> = response.into_json().await.expect("json");
    assert_eq!(tests.len(), 3);
}

#[rocket::async_test]
async fn get_tests_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/verifications").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// GET /api/verifications/{id} - Get Single Verification
// ============================================================================

#[rocket::async_test]
async fn get_test_by_id_returns_correct_test() {
    let mut repo = base_repo();
    repo.verifications
        .insert(1, sample_test(1, 1, "Integration Test"));

    let client = test_client(repo).await;

    let response = client
        .get("/api/verifications/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let test: Verification = response.into_json().await.expect("json");
    assert_eq!(test.id, 1);
    assert_eq!(test.name, "Integration Test");
    assert_eq!(test.reference_code, "TST-001");
}

#[rocket::async_test]
async fn get_test_with_nonexistent_id_returns_404() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/api/verifications/999")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn get_test_requires_authentication() {
    let mut repo = base_repo();
    repo.verifications.insert(1, sample_test(1, 1, "Test"));

    let client = test_client(repo).await;

    let response = client.get("/api/verifications/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// POST /api/verifications - Create New Verification
// ============================================================================

#[rocket::async_test]
async fn post_test_creates_new_test() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/verifications")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(new_test_json("Smoke Test", 1).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    assert_eq!(result["status"], "ok");
    assert_eq!(result["id"], 1);
}

#[rocket::async_test]
async fn post_test_with_missing_fields_returns_error() {
    let client = test_client(base_repo()).await;

    let invalid_json = json!({
        "name": "Incomplete Test"
        // Missing required fields
    });

    let response = client
        .post("/api/verifications")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(invalid_json.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::UnprocessableEntity);
}

#[rocket::async_test]
async fn post_test_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/verifications")
        .header(ContentType::JSON)
        .body(new_test_json("New Test", 1).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// POST /api/verifications/{id}/field - Update Verification Field
// ============================================================================

#[rocket::async_test]
async fn update_field_changes_test_name() {
    let mut repo = base_repo();
    repo.verifications
        .insert(1, sample_test(1, 1, "Original Name"));

    let client = test_client(repo).await;

    let update = json!({
        "field": "name",
        "value": "Updated Name"
    });

    let response = client
        .post("/api/verifications/1/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    assert_eq!(result["success"], true);

    // Verify the update
    let get_response = client
        .get("/api/verifications/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let test: Verification = get_response.into_json().await.expect("json");
    assert_eq!(test.name, "Updated Name");
}

#[rocket::async_test]
async fn update_field_changes_test_status() {
    let mut repo = base_repo();
    repo.verifications.insert(1, sample_test(1, 1, "Test"));

    let client = test_client(repo).await;

    let update = json!({
        "field": "status_id",
        "value": "2"
    });

    let response = client
        .post("/api/verifications/1/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);

    // Verify status was updated
    let get_response = client
        .get("/api/verifications/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let test: Verification = get_response.into_json().await.expect("json");
    assert_eq!(test.status_id, 2);
}

#[rocket::async_test]
async fn update_field_with_invalid_field_returns_error() {
    let mut repo = base_repo();
    repo.verifications.insert(1, sample_test(1, 1, "Test"));

    let client = test_client(repo).await;

    let update = json!({
        "field": "invalid_field",
        "value": "value"
    });

    let response = client
        .post("/api/verifications/1/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn update_field_with_invalid_status_value_returns_error() {
    let mut repo = base_repo();
    repo.verifications.insert(1, sample_test(1, 1, "Test"));

    let client = test_client(repo).await;

    let update = json!({
        "field": "status_id",
        "value": "invalid"
    });

    let response = client
        .post("/api/verifications/1/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

// ============================================================================
// DELETE /api/verifications/{id} - Delete Verification
// ============================================================================

#[rocket::async_test]
async fn delete_test_removes_test() {
    let mut repo = base_repo();
    repo.verifications.insert(1, sample_test(1, 1, "To Delete"));

    let client = test_client(repo).await;

    let response = client
        .delete("/api/verifications/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NoContent);

    // Verify test is gone
    let get_response = client
        .get("/api/verifications/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(get_response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn delete_nonexistent_test_returns_404() {
    let client = test_client(base_repo()).await;

    let response = client
        .delete("/api/verifications/999")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn delete_test_requires_authentication() {
    let mut repo = base_repo();
    repo.verifications.insert(1, sample_test(1, 1, "To Delete"));

    let client = test_client(repo).await;

    let response = client.delete("/api/verifications/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// Test Hierarchy Tests
// ============================================================================

#[rocket::async_test]
async fn create_test_with_parent() {
    let mut repo = base_repo();
    repo.verifications
        .insert(1, sample_test(1, 1, "Parent Test"));

    let client = test_client(repo).await;

    let mut child_json = new_test_json("Child Test", 1);
    child_json["parent_id"] = json!(1);

    let response = client
        .post("/api/verifications")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(child_json.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    let child_id = result["id"].as_i64().expect("id");

    // Verify parent relationship
    let get_response = client
        .get(format!("/api/verifications/{}", child_id))
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let child_test: Verification = get_response.into_json().await.expect("json");
    assert_eq!(child_test.parent_id, Some(1));
}

#[rocket::async_test]
async fn update_test_parent() {
    let mut repo = base_repo();
    repo.verifications
        .insert(1, sample_test(1, 1, "Parent Test"));
    repo.verifications
        .insert(2, sample_test(2, 1, "Child Test"));

    let client = test_client(repo).await;

    let update = json!({
        "field": "parent_id",
        "value": "1"
    });

    let response = client
        .post("/api/verifications/2/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);

    // Verify parent was set
    let get_response = client
        .get("/api/verifications/2")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let test: Verification = get_response.into_json().await.expect("json");
    assert_eq!(test.parent_id, Some(1));
}

// ============================================================================
// Additional Field Update Tests
// ============================================================================

#[rocket::async_test]
async fn update_test_description() {
    let mut repo = base_repo();
    repo.verifications.insert(1, sample_test(1, 1, "Test"));

    let client = test_client(repo).await;

    let update = json!({
        "field": "description",
        "value": "Updated description"
    });

    let response = client
        .post("/api/verifications/1/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);

    let get_response = client
        .get("/api/verifications/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let test: Verification = get_response.into_json().await.expect("json");
    assert_eq!(test.description, "Updated description");
}

#[rocket::async_test]
async fn update_test_source() {
    let mut repo = base_repo();
    repo.verifications.insert(1, sample_test(1, 1, "Test"));

    let client = test_client(repo).await;

    let update = json!({
        "field": "source",
        "value": "manual"
    });

    let response = client
        .post("/api/verifications/1/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);

    let get_response = client
        .get("/api/verifications/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let test: Verification = get_response.into_json().await.expect("json");
    assert_eq!(test.source, "manual");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[rocket::async_test]
async fn create_multiple_tests_sequentially() {
    let client = test_client(base_repo()).await;

    for i in 1..=5 {
        let response = client
            .post("/api/verifications")
            .header(ContentType::JSON)
            .private_cookie(session_cookie(1))
            .body(new_test_json(&format!("Test {}", i), 1).to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let result: Value = response.into_json().await.expect("json");
        assert_eq!(result["id"], i);
    }

    // Verify all were created
    let list_response = client
        .get("/api/verifications")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let tests: Vec<Verification> = list_response.into_json().await.expect("json");
    assert_eq!(tests.len(), 5);
}

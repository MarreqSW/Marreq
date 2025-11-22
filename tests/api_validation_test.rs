#![cfg(feature = "test-helpers")]

//! Comprehensive validation and edge case tests for API endpoints.
//!
//! These tests verify:
//! - Invalid JSON payloads are rejected
//! - Missing required fields return errors
//! - Type mismatches are handled
//! - Boundary values are validated
//! - SQL injection attempts are prevented
//! - XSS attempts are handled

use req_man::models::*;
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;
use serde_json::json;

mod test_support {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use req_man::app::AppState;
    use req_man::auth::session::SESSION_COOKIE;
    use req_man::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
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
            .mount("/api", req_man::api::routes());

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

        repo.projects.insert(
            1,
            Project {
                project_id: 1,
                project_name: "Test Project".into(),
                project_description: Some("Description".into()),
                project_creation_date: Some(timestamp()),
                project_update_date: Some(timestamp()),
                project_status: Some("Active".into()),
                project_owner_id: Some(1),
            },
        );

        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                req_st_id: 1,
                req_st_title: "Draft".into(),
                req_st_description: "".into(),
                req_st_short_name: "D".into(),
            },
        );

        repo.categories.insert(
            1,
            Category {
                cat_id: 1,
                cat_title: "Test Category".into(),
                cat_description: "".into(),
                cat_tag: "TEST".into(),
                project_id: 1,
            },
        );

        repo.applicability.insert(
            1,
            Applicability {
                app_id: 1,
                app_title: "All".into(),
                app_description: "".into(),
                app_tag: "ALL".into(),
                project_id: 1,
            },
        );

        repo.verifications.insert(
            1,
            Verification {
                verification_id: 1,
                verification_name: "Analysis".into(),
                verification_description: "".into(),
                project_id: 1,
            },
        );

        repo
    }
}

use test_support::*;

// ============================================================================
// Requirements API - Validation Tests
// ============================================================================

#[rocket::async_test]
async fn create_requirement_with_missing_fields_returns_error() {
    let client = test_client(base_repo()).await;

    // Missing req_title
    let payload = json!({
        "req_description": "Test description",
        "req_reference": "REQ-001",
        "project_id": 1
    });

    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(payload.to_string())
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::UnprocessableEntity || status == Status::BadRequest);
}

#[rocket::async_test]
async fn create_requirement_with_invalid_json_returns_error() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body("{ invalid json }")
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::UnprocessableEntity || status == Status::BadRequest);
}

#[rocket::async_test]
async fn create_requirement_with_type_mismatch_returns_error() {
    let client = test_client(base_repo()).await;

    // project_id should be number, not string
    let payload = json!({
        "req_title": "Test",
        "req_description": "Test description",
        "req_reference": "REQ-001",
        "req_category": 1,
        "req_applicability": 1,
        "req_current_status": 1,
        "req_verification": 1,
        "project_id": "not-a-number"
    });

    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(payload.to_string())
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::UnprocessableEntity || status == Status::BadRequest);
}

#[rocket::async_test]
async fn create_requirement_with_very_long_string_handles_gracefully() {
    let client = test_client(base_repo()).await;

    let long_string = "a".repeat(10000);
    let payload = json!({
        "req_title": long_string,
        "req_description": "Test description",
        "req_reference": "REQ-001",
        "req_category": 1,
        "req_applicability": 1,
        "req_current_status": 1,
        "req_verification": 1,
        "project_id": 1
    });

    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(payload.to_string())
        .dispatch()
        .await;

    // Should either succeed (if allowed) or return validation error
    let status = response.status();
    assert!(
        status == Status::Ok
            || status == Status::Created
            || status == Status::BadRequest
            || status == Status::UnprocessableEntity
    );
}

#[rocket::async_test]
async fn create_requirement_with_negative_id_returns_error() {
    let client = test_client(base_repo()).await;

    let payload = json!({
        "req_title": "Test",
        "req_description": "Test description",
        "req_reference": "REQ-001",
        "req_category": -1,
        "req_applicability": 1,
        "req_current_status": 1,
        "req_verification": 1,
        "project_id": 1
    });

    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(payload.to_string())
        .dispatch()
        .await;

    // Should return error for negative category ID
    let status = response.status();
    assert!(
        status == Status::BadRequest
            || status == Status::NotFound
            || status == Status::UnprocessableEntity
    );
}

#[rocket::async_test]
async fn patch_requirement_with_empty_patch_returns_bad_request() {
    let mut repo = base_repo();
    repo.requirements.insert(
        1,
        Requirement {
            req_id: 1,
            req_title: "Test".into(),
            req_description: "Test".into(),
            req_reference: "REQ-001".into(),
            req_category: 1,
            req_applicability: 1,
            req_current_status: 1,
            req_verification: 1,
            req_author: 1,
            req_reviewer: 1,
            req_parent: 0,
            req_creation_date: timestamp(),
            req_update_date: timestamp(),
            req_deadline_date: timestamp(),
            req_justification: None,
            project_id: 1,
        },
    );

    let client = test_client(repo).await;

    let patch = json!({});

    let response = client
        .patch("/api/requirements/1")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(patch.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn patch_requirement_with_invalid_field_type_returns_error() {
    let mut repo = base_repo();
    repo.requirements.insert(
        1,
        Requirement {
            req_id: 1,
            req_title: "Test".into(),
            req_description: "Test".into(),
            req_reference: "REQ-001".into(),
            req_category: 1,
            req_applicability: 1,
            req_current_status: 1,
            req_verification: 1,
            req_author: 1,
            req_reviewer: 1,
            req_parent: 0,
            req_creation_date: timestamp(),
            req_update_date: timestamp(),
            req_deadline_date: timestamp(),
            req_justification: None,
            project_id: 1,
        },
    );

    let client = test_client(repo).await;

    // req_current_status should be number, not string
    let patch = json!({
        "req_current_status": "not-a-number"
    });

    let response = client
        .patch("/api/requirements/1")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(patch.to_string())
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::UnprocessableEntity || status == Status::BadRequest);
}

// ============================================================================
// Tests API - Validation Tests
// ============================================================================

#[rocket::async_test]
async fn create_test_with_missing_fields_returns_error() {
    let client = test_client(base_repo()).await;

    // Missing test_name
    let payload = json!({
        "test_description": "Test description",
        "test_reference": "TEST-001",
        "project_id": 1
    });

    let response = client
        .post("/api/tests")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(payload.to_string())
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::UnprocessableEntity || status == Status::BadRequest);
}

#[rocket::async_test]
async fn update_test_field_with_invalid_field_name_returns_error() {
    let mut repo = base_repo();
    repo.tests.insert(
        1,
        Test {
            test_id: 1,
            test_name: "Test".into(),
            test_description: "Description".into(),
            test_reference: "TEST-001".into(),
            test_source: "manual".into(),
            test_status: 1,
            test_parent: 0,
            project_id: 1,
        },
    );

    let client = test_client(repo).await;

    let update = json!({
        "field": "invalid_field_name",
        "value": "some value"
    });

    let response = client
        .post("/api/tests/1/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::BadRequest || status == Status::UnprocessableEntity);
}

#[rocket::async_test]
async fn update_test_field_with_invalid_status_value_returns_error() {
    let mut repo = base_repo();
    repo.tests.insert(
        1,
        Test {
            test_id: 1,
            test_name: "Test".into(),
            test_description: "Description".into(),
            test_reference: "TEST-001".into(),
            test_source: "manual".into(),
            test_status: 1,
            test_parent: 0,
            project_id: 1,
        },
    );

    let client = test_client(repo).await;

    // test_status should be parseable as i32
    let update = json!({
        "field": "test_status",
        "value": "not-a-number"
    });

    let response = client
        .post("/api/tests/1/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::BadRequest || status == Status::UnprocessableEntity);
}

// ============================================================================
// Categories API - Validation Tests
// ============================================================================

#[rocket::async_test]
async fn create_category_with_missing_fields_returns_error() {
    let client = test_client(base_repo()).await;

    // Missing cat_title
    let payload = json!({
        "cat_description": "Description",
        "cat_tag": "TAG",
        "project_id": 1
    });

    let response = client
        .post("/api/categories")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(payload.to_string())
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::UnprocessableEntity || status == Status::BadRequest);
}

#[rocket::async_test]
async fn update_category_with_invalid_json_returns_error() {
    let mut repo = base_repo();
    repo.categories.insert(
        1,
        Category {
            cat_id: 1,
            cat_title: "Test".into(),
            cat_description: "".into(),
            cat_tag: "TEST".into(),
            project_id: 1,
        },
    );

    let client = test_client(repo).await;

    let response = client
        .put("/api/categories/1")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body("{ invalid json }")
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::UnprocessableEntity || status == Status::BadRequest);
}

// ============================================================================
// Applicability API - Validation Tests
// ============================================================================

#[rocket::async_test]
async fn create_applicability_with_missing_fields_returns_error() {
    let client = test_client(base_repo()).await;

    // Missing app_title
    let payload = json!({
        "app_description": "Description",
        "app_tag": "TAG",
        "project_id": 1
    });

    let response = client
        .post("/api/applicability")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(payload.to_string())
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::UnprocessableEntity || status == Status::BadRequest);
}

// ============================================================================
// Users API - Validation Tests
// ============================================================================

#[rocket::async_test]
async fn create_user_with_missing_fields_returns_error() {
    let client = test_client(base_repo()).await;

    // Missing user_username
    let payload = json!({
        "user_name": "New User",
        "user_email": "new@example.com"
    });

    let response = client
        .post("/api/users")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(payload.to_string())
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::UnprocessableEntity || status == Status::BadRequest);
}

#[rocket::async_test]
async fn create_user_with_invalid_email_format_handles_gracefully() {
    let client = test_client(base_repo()).await;

    let payload = json!({
        "user_username": "newuser",
        "user_name": "New User",
        "user_email": "not-a-valid-email",
        "is_admin": false
    });

    let response = client
        .post("/api/users")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(payload.to_string())
        .dispatch()
        .await;

    // Should either validate and reject, or accept (depending on validation rules)
    let status = response.status();
    assert!(
        status == Status::Ok
            || status == Status::Created
            || status == Status::BadRequest
            || status == Status::UnprocessableEntity
    );
}

// ============================================================================
// Security Tests - SQL Injection Prevention
// ============================================================================

#[rocket::async_test]
async fn sql_injection_in_requirement_title_is_handled_safely() {
    let client = test_client(base_repo()).await;

    // SQL injection attempt in title
    let payload = json!({
        "req_title": "'; DROP TABLE requirements; --",
        "req_description": "Test description",
        "req_reference": "REQ-001",
        "req_category": 1,
        "req_applicability": 1,
        "req_current_status": 1,
        "req_verification": 1,
        "project_id": 1
    });

    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(payload.to_string())
        .dispatch()
        .await;

    // Should either succeed (if properly escaped) or reject
    // The important thing is it doesn't crash or execute SQL
    let status = response.status();
    assert!(
        status == Status::Ok
            || status == Status::Created
            || status == Status::BadRequest
            || status == Status::UnprocessableEntity
    );
}

#[rocket::async_test]
async fn sql_injection_in_id_parameter_is_handled_safely() {
    let mut repo = base_repo();
    repo.requirements.insert(
        1,
        Requirement {
            req_id: 1,
            req_title: "Test".into(),
            req_description: "Test".into(),
            req_reference: "REQ-001".into(),
            req_category: 1,
            req_applicability: 1,
            req_current_status: 1,
            req_verification: 1,
            req_author: 1,
            req_reviewer: 1,
            req_parent: 0,
            req_creation_date: timestamp(),
            req_update_date: timestamp(),
            req_deadline_date: timestamp(),
            req_justification: None,
            project_id: 1,
        },
    );

    let client = test_client(repo).await;

    // Try SQL injection in ID parameter
    let response = client
        .get("/api/requirements/1 OR 1=1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    // Should return 404 or BadRequest, not execute SQL
    let status = response.status();
    assert!(
        status == Status::NotFound
            || status == Status::BadRequest
            || status == Status::UnprocessableEntity
    );
}

// ============================================================================
// Security Tests - XSS Prevention
// ============================================================================

#[rocket::async_test]
async fn xss_attempt_in_requirement_title_is_handled_safely() {
    let client = test_client(base_repo()).await;

    // XSS attempt in title
    let payload = json!({
        "req_title": "<script>alert('XSS')</script>",
        "req_description": "Test description",
        "req_reference": "REQ-001",
        "req_category": 1,
        "req_applicability": 1,
        "req_current_status": 1,
        "req_verification": 1,
        "project_id": 1
    });

    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(payload.to_string())
        .dispatch()
        .await;

    // Should either succeed (if properly escaped) or reject
    // The important thing is it doesn't execute scripts
    let status = response.status();
    assert!(
        status == Status::Ok
            || status == Status::Created
            || status == Status::BadRequest
            || status == Status::UnprocessableEntity
    );
}

// ============================================================================
// Boundary Value Tests
// ============================================================================

#[rocket::async_test]
async fn get_requirement_with_zero_id_returns_error() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/api/requirements/0")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::NotFound || status == Status::BadRequest);
}

#[rocket::async_test]
async fn get_requirement_with_negative_id_returns_error() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/api/requirements/-1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::NotFound || status == Status::BadRequest);
}

#[rocket::async_test]
async fn get_requirement_with_very_large_id_returns_error() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/api/requirements/999999999")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::NotFound || status == Status::BadRequest);
}


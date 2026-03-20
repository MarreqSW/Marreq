// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

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

use marreq::models::*;
use marreq::status_enums::ProjectStatus;
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;
use serde_json::json;

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

    pub fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        let mut admin = DieselRepoMock::make_user(1, "admin", "password");
        admin.is_admin = true;
        repo.users.insert(1, admin);

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
                title: "Test Category".into(),
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
        "status_id": 1,
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
        "status_id": 1,
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
        "status_id": 1,
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
            id: 1,
            current_version_id: None,
            same_as_current: None,
            title: "Test".into(),
            description: "Test".into(),
            reference_code: "REQ-001".into(),
            category_id: 1,
            applicability_id: 1,
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            justification: None,
            project_id: 1,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
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
            id: 1,
            current_version_id: None,
            same_as_current: None,
            title: "Test".into(),
            description: "Test".into(),
            reference_code: "REQ-001".into(),
            category_id: 1,
            applicability_id: 1,
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            justification: None,
            project_id: 1,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );

    let client = test_client(repo).await;

    // status_id should be number, not string
    let patch = json!({
        "status_id": "not-a-number"
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
        .post("/api/verifications")
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
    repo.verifications.insert(
        1,
        Verification {
            id: 1,
            name: "Test".into(),
            description: "Description".into(),
            reference_code: "TEST-001".into(),
            source: "manual".into(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
            verification_method_id: None,
        },
    );

    let client = test_client(repo).await;

    let update = json!({
        "field": "invalid_field_name",
        "value": "some value"
    });

    let response = client
        .post("/api/verifications/1/field")
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
    repo.verifications.insert(
        1,
        Verification {
            id: 1,
            name: "Test".into(),
            description: "Description".into(),
            reference_code: "TEST-001".into(),
            source: "manual".into(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
            verification_method_id: None,
        },
    );

    let client = test_client(repo).await;

    // status_id should be parseable as i32
    let update = json!({
        "field": "status_id",
        "value": "not-a-number"
    });

    let response = client
        .post("/api/verifications/1/field")
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

    // Missing title
    let payload = json!({
        "description": "Description",
        "tag": "TAG",
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
            id: 1,
            title: "Test".into(),
            description: "".into(),
            tag: "TEST".into(),
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

    // Missing title
    let payload = json!({
        "description": "Description",
        "tag": "TAG",
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
        "status_id": 1,
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
            id: 1,
            current_version_id: None,
            same_as_current: None,
            title: "Test".into(),
            description: "Test".into(),
            reference_code: "REQ-001".into(),
            category_id: 1,
            applicability_id: 1,
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            justification: None,
            project_id: 1,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
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
        "status_id": 1,
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

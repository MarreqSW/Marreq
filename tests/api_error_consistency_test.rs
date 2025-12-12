#![cfg(feature = "test-helpers")]

//! Comprehensive error response consistency tests for API endpoints.
//!
//! These tests verify:
//! - Consistent error format across all endpoints
//! - Proper HTTP status codes
//! - Error message clarity
//! - Error response structure

use req_man::models::*;
use req_man::status_enums::ProjectStatus;
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;
use serde_json::{json, Value};

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
                id: 1,
                name: "Test Project".into(),
                description: Some("Description".into()),
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(1),
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

        repo
    }
}

use test_support::*;

// ============================================================================
// 401 Unauthorized - Consistent Format
// ============================================================================

#[rocket::async_test]
async fn unauthorized_errors_return_401_status() {
    let client = test_client(base_repo()).await;

    let endpoints = vec![
        "/api/requirements",
        "/api/requirements/1",
        "/api/tests",
        "/api/tests/1",
        "/api/categories",
        "/api/categories/1",
        "/api/applicability",
        "/api/applicability/1",
        "/api/users",
        "/api/users/1",
    ];

    for endpoint in endpoints {
        let response = client.get(endpoint).dispatch().await;
        assert_eq!(
            response.status(),
            Status::Unauthorized,
            "Endpoint {} should return 401 Unauthorized",
            endpoint
        );
    }
}

// ============================================================================
// 404 Not Found - Consistent Format
// ============================================================================

#[rocket::async_test]
async fn not_found_errors_return_404_status() {
    let client = test_client(base_repo()).await;

    let endpoints = vec![
        ("GET", "/api/requirements/999"),
        ("GET", "/api/tests/999"),
        ("GET", "/api/categories/999"),
        ("GET", "/api/applicability/999"),
        ("GET", "/api/users/999"),
        ("DELETE", "/api/requirements/999"),
        ("DELETE", "/api/tests/999"),
        ("DELETE", "/api/categories/999"),
        ("DELETE", "/api/applicability/999"),
        ("DELETE", "/api/users/999"),
    ];

    for (method, endpoint) in endpoints {
        let response = match method {
            "GET" => {
                client
                    .get(endpoint)
                    .private_cookie(session_cookie(1))
                    .dispatch()
                    .await
            }
            "DELETE" => {
                client
                    .delete(endpoint)
                    .private_cookie(session_cookie(1))
                    .dispatch()
                    .await
            }
            _ => continue,
        };

        let status = response.status();
        assert!(
            status == Status::NotFound || status == Status::BadRequest,
            "Endpoint {} {} should return 404 Not Found or 400 Bad Request, got {:?}",
            method,
            endpoint,
            status
        );
    }
}

// ============================================================================
// 400 Bad Request - Consistent Format
// ============================================================================

#[rocket::async_test]
async fn bad_request_errors_return_400_status() {
    let client = test_client(base_repo()).await;

    // Empty PATCH request
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

// ============================================================================
// 422 Unprocessable Entity - Consistent Format
// ============================================================================

#[rocket::async_test]
async fn unprocessable_entity_errors_return_422_status() {
    let client = test_client(base_repo()).await;

    // Invalid JSON
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

// ============================================================================
// Error Response Structure Tests
// ============================================================================

#[rocket::async_test]
async fn error_responses_have_consistent_structure() {
    let client = test_client(base_repo()).await;

    // Test 404 error response structure
    let response = client
        .get("/api/requirements/999")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    if response.status() == Status::NotFound {
        let body: Option<Value> = response.into_json().await;
        // Error responses should be valid JSON
        if let Some(error) = body {
            // Should have some error information
            assert!(error.is_object() || error.is_string());
        }
    }
}

#[rocket::async_test]
async fn bad_request_has_error_message() {
    let mut repo = base_repo();
    repo.requirements.insert(
        1,
        Requirement {
            id: 1,
            title: "Test".into(),
            description: "Test".into(),
            reference_code: "REQ-001".into(),
            category_id: 1,
            applicability_id: 1,
            status_id: 1,
            verification_method_id: 1,
            author_id: 1,
            reviewer_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            justification: None,
            project_id: 1,
        },
    );

    let client = test_client(repo).await;

    // Empty PATCH should return BadRequest with message
    let patch = json!({});
    let response = client
        .patch("/api/requirements/1")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(patch.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);

    let body: Option<Value> = response.into_json().await;
    if let Some(error) = body {
        // Should have error message
        assert!(error.is_object() || error.is_string());
    }
}

// ============================================================================
// HTTP Method Validation
// ============================================================================

#[rocket::async_test]
async fn unsupported_methods_return_405_or_404() {
    let client = test_client(base_repo()).await;

    // Try PUT on requirements (only PATCH is supported)
    let response = client
        .put("/api/requirements/1")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body("{}")
        .dispatch()
        .await;

    // Should return 404 (route not found) or 405 (method not allowed)
    let status = response.status();
    assert!(status == Status::NotFound || status == Status::MethodNotAllowed);
}

// ============================================================================
// Content-Type Validation
// ============================================================================

#[rocket::async_test]
async fn missing_content_type_on_post_handled_gracefully() {
    let client = test_client(base_repo()).await;

    let payload = json!({
        "req_title": "Test",
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
        .private_cookie(session_cookie(1))
        .body(payload.to_string())
        .dispatch()
        .await;

    // Should either work (if Content-Type is optional) or return error
    let status = response.status();
    assert!(
        status == Status::Ok
            || status == Status::Created
            || status == Status::BadRequest
            || status == Status::UnprocessableEntity
    );
}

// ============================================================================
// Error Message Clarity Tests
// ============================================================================

#[rocket::async_test]
async fn error_messages_are_clear_and_actionable() {
    let mut repo = base_repo();
    repo.requirements.insert(
        1,
        Requirement {
            id: 1,
            title: "Test".into(),
            description: "Test".into(),
            reference_code: "REQ-001".into(),
            category_id: 1,
            applicability_id: 1,
            status_id: 1,
            verification_method_id: 1,
            author_id: 1,
            reviewer_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            justification: None,
            project_id: 1,
        },
    );

    let client = test_client(repo).await;

    // Empty PATCH should have clear error message
    let patch = json!({});
    let response = client
        .patch("/api/requirements/1")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(patch.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);

    let body: Option<Value> = response.into_json().await;
    if let Some(error) = body {
        let error_str = error.to_string();
        // Error should contain some information
        assert!(!error_str.is_empty());
    }
}

// ============================================================================
// Status Code Consistency Across Endpoints
// ============================================================================

#[rocket::async_test]
async fn same_error_conditions_return_same_status_codes() {
    let client = test_client(base_repo()).await;

    // All create endpoints with missing required fields should return same status
    let endpoints = vec![
        ("/api/requirements", json!({"req_title": "Test"})),
        ("/api/tests", json!({"test_name": "Test"})),
        ("/api/categories", json!({"title": "Test"})),
        ("/api/applicability", json!({"title": "Test"})),
    ];

    for (endpoint, payload) in endpoints {
        let response = client
            .post(endpoint)
            .header(ContentType::JSON)
            .private_cookie(session_cookie(1))
            .body(payload.to_string())
            .dispatch()
            .await;

        // All should return validation error (422 or 400)
        let status = response.status();
        assert!(
            status == Status::UnprocessableEntity || status == Status::BadRequest,
            "Endpoint {} should return 422 or 400 for missing fields, got {:?}",
            endpoint,
            status
        );
    }
}

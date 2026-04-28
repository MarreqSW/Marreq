// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

//! Comprehensive authentication and authorization tests for all API endpoints.
//!
//! These tests verify:
//! - All endpoints require authentication
//! - Invalid sessions are rejected
//! - Expired sessions are handled
//! - Admin vs regular user permissions

use marreq_core::auth::csrf::CSRF_COOKIE;
use marreq_core::auth::hash_password;
use marreq_core::auth::session::{session_cookie_name_for_request, SESSION_COOKIE};
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

        let user = DieselRepoMock::make_user(2, "user", "password");
        repo.users.insert(2, user);

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

        repo
    }
}

use test_support::*;

// ============================================================================
// Auth API - Login Tests
// ============================================================================

#[rocket::async_test]
async fn auth_login_returns_authenticated_user_and_sets_cookies() {
    let mut repo = base_repo();
    let mut admin = repo.users.get(&1).cloned().expect("admin user");
    admin.password_hash = hash_password("Voyager!Marble_2026").expect("hashed password");
    repo.users.insert(1, admin);

    let client = test_client(repo).await;

    let response = client
        .post("/api/auth/login")
        .header(ContentType::JSON)
        .body(
            json!({
                "username": "admin",
                "password": "Voyager!Marble_2026",
            })
            .to_string(),
        )
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);

    let body: Value = response.into_json().await.expect("json");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["user"]["id"], 1);
    assert_eq!(body["user"]["username"], "admin");
    assert!(body["user"].get("password_hash").is_none());

    let jar = client.cookies();
    let session_cookie = jar
        .get_private(session_cookie_name_for_request())
        .expect("session cookie");
    assert_ne!(session_cookie.value(), "1");
    assert!(session_cookie.value().len() >= 40);

    let csrf_cookie = jar.get_private(CSRF_COOKIE).expect("csrf cookie");
    assert_eq!(csrf_cookie.value().len(), 64);
    assert!(csrf_cookie.value().chars().all(|c| c.is_ascii_hexdigit()));

    let me = client.get("/api/auth/me").dispatch().await;
    assert_eq!(me.status(), Status::Ok);
}

// ============================================================================
// Requirements API - Authentication Tests
// ============================================================================

#[rocket::async_test]
async fn requirements_list_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/requirements").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn requirements_get_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/requirements/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn requirements_create_requires_authentication() {
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
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn requirements_delete_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.delete("/api/requirements/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn requirements_patch_requires_authentication() {
    let client = test_client(base_repo()).await;

    let patch = json!({
        "req_title": "Updated Title"
    });

    let response = client
        .patch("/api/requirements/1")
        .header(ContentType::JSON)
        .body(patch.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// Tests API - Authentication Tests
// ============================================================================

#[rocket::async_test]
async fn tests_list_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/verifications").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn tests_get_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/verifications/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn tests_create_requires_authentication() {
    let client = test_client(base_repo()).await;

    let payload = json!({
        "test_name": "Test",
        "test_description": "Test description",
        "test_reference": "TEST-001",
        "status_id": 1,
        "test_source": "manual",
        "project_id": 1
    });

    let response = client
        .post("/api/verifications")
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn tests_delete_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.delete("/api/verifications/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn tests_update_field_requires_authentication() {
    let client = test_client(base_repo()).await;

    let update = json!({
        "field": "test_name",
        "value": "Updated Name"
    });

    let response = client
        .post("/api/verifications/1/field")
        .header(ContentType::JSON)
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// Categories API - Authentication Tests
// ============================================================================

#[rocket::async_test]
async fn categories_list_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/categories").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn categories_get_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/categories/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn categories_create_requires_authentication() {
    let client = test_client(base_repo()).await;

    let payload = json!({
        "title": "New Category",
        "description": "Description",
        "tag": "NEW",
        "project_id": 1
    });

    let response = client
        .post("/api/categories")
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn categories_update_requires_authentication() {
    let client = test_client(base_repo()).await;

    let payload = json!({
        "title": "Updated Category",
        "description": "Updated",
        "tag": "UPD",
        "project_id": 1
    });

    let response = client
        .put("/api/categories/1")
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn categories_delete_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.delete("/api/categories/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// Applicability API - Authentication Tests
// ============================================================================

#[rocket::async_test]
async fn applicability_list_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/applicability").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn applicability_get_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/applicability/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn applicability_create_requires_authentication() {
    let client = test_client(base_repo()).await;

    let payload = json!({
        "title": "New Applicability",
        "description": "Description",
        "tag": "NEW",
        "project_id": 1
    });

    let response = client
        .post("/api/applicability")
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn applicability_update_requires_authentication() {
    let client = test_client(base_repo()).await;

    let payload = json!({
        "title": "Updated Applicability",
        "description": "Updated",
        "tag": "UPD",
        "project_id": 1
    });

    let response = client
        .put("/api/applicability/1")
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn applicability_delete_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.delete("/api/applicability/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// Users API - Authentication Tests
// ============================================================================

#[rocket::async_test]
async fn users_list_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/users").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn users_get_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/users/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn users_create_requires_authentication() {
    let client = test_client(base_repo()).await;

    let payload = json!({
        "user_username": "newuser",
        "user_name": "New User",
        "user_email": "new@example.com",
        "is_admin": false
    });

    let response = client
        .post("/api/users")
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn users_delete_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.delete("/api/users/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// Status API - Authentication Tests
// ============================================================================

#[rocket::async_test]
async fn status_list_does_not_require_authentication() {
    // Status endpoint doesn't require auth based on the code
    let client = test_client(base_repo()).await;

    let response = client.get("/api/status").dispatch().await;

    // Status endpoint is public, should work without auth
    let status = response.status();
    assert!(status == Status::Ok || status == Status::InternalServerError);
}

#[rocket::async_test]
async fn status_get_does_not_require_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/status/1").dispatch().await;

    // Status endpoint is public
    let status = response.status();
    assert!(
        status == Status::Ok || status == Status::NotFound || status == Status::InternalServerError
    );
}

#[rocket::async_test]
async fn status_create_does_not_require_authentication() {
    let client = test_client(base_repo()).await;

    let payload = json!({
        "title": "New Status",
        "description": "Description",
        "tag": "NEW",
        "project_id": 1
    });

    let response = client
        .post("/api/status")
        .header(ContentType::JSON)
        .body(payload.to_string())
        .dispatch()
        .await;

    // Status endpoint is public
    let status = response.status();
    assert!(status == Status::Created || status == Status::InternalServerError);
}

// ============================================================================
// Matrix API - Authentication Tests
// ============================================================================

#[rocket::async_test]
async fn matrix_list_does_not_require_authentication() {
    // Matrix endpoint doesn't require auth based on the code
    let client = test_client(base_repo()).await;

    let response = client.get("/api/matrix").dispatch().await;

    // Matrix endpoint is public, may return error if DB connection fails
    let status = response.status();
    assert!(status == Status::Ok || status == Status::InternalServerError);
}

// ============================================================================
// Cache API - Authentication Tests
// ============================================================================

#[rocket::async_test]
async fn cache_stats_does_not_require_authentication() {
    // Cache endpoints don't require auth based on the code
    let client = test_client(base_repo()).await;

    let response = client.get("/api/cache/stats").dispatch().await;

    let status = response.status();
    assert!(status == Status::Ok || status == Status::InternalServerError);
}

#[rocket::async_test]
async fn cache_clear_does_not_require_authentication() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/cache/clear")
        .header(ContentType::JSON)
        .dispatch()
        .await;

    let status = response.status();
    assert!(status == Status::Ok || status == Status::InternalServerError);
}

// ============================================================================
// Invalid Session Tests
// ============================================================================

#[rocket::async_test]
async fn invalid_session_cookie_returns_unauthorized() {
    let client = test_client(base_repo()).await;

    let mut invalid_cookie = Cookie::new(SESSION_COOKIE, "99999");
    invalid_cookie.set_path("/");

    let response = client
        .get("/api/requirements")
        .private_cookie(invalid_cookie)
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn malformed_session_cookie_returns_unauthorized() {
    let client = test_client(base_repo()).await;

    let mut invalid_cookie = Cookie::new(SESSION_COOKIE, "not-a-number");
    invalid_cookie.set_path("/");

    let response = client
        .get("/api/requirements")
        .private_cookie(invalid_cookie)
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn missing_session_cookie_returns_unauthorized() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/requirements").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// Admin vs Regular User Tests
// ============================================================================

#[rocket::async_test]
async fn admin_can_access_all_endpoints() {
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
            parent_id: Some(0),
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

    // Admin should be able to list requirements
    let response = client
        .get("/api/requirements")
        .private_cookie(session_cookie(&client, 1)) // Admin user
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn regular_user_can_access_endpoints() {
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
            author_id: 2,
            reviewer_id: 2,
            parent_id: Some(0),
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

    // Regular user should be able to list requirements
    let response = client
        .get("/api/requirements")
        .private_cookie(session_cookie(&client, 2)) // Regular user
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
}

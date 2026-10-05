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
use marreq_core::auth::session::{SESSION_COOKIE, session_cookie_name_for_request};
use marreq_core::models::*;
use marreq_core::repository::UserRepository;
use marreq_core::status_enums::ProjectStatus;
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;
use serde_json::{Value, json};

mod test_support {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use marreq_core::app::AppState;
    use marreq_core::auth::session::test_session_cookie_for;
    use marreq_core::repository::{CacheRepository, diesel_repo_mock::DieselRepoMock};
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
        test_client_with_auth(repo, marreq_core::auth::AuthConfig::default()).await
    }

    pub async fn test_client_with_auth(
        repo: DieselRepoMock,
        auth_config: marreq_core::auth::AuthConfig,
    ) -> Client {
        marreq_core::deployment::install_test_server_mode();
        let rocket = rocket::build()
            .manage(managed_state(repo))
            .manage(auth_config)
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
                archived_at: None,
                archived_by: None,
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

    pub fn hashed_user_repo() -> DieselRepoMock {
        let mut repo = base_repo();
        let mut user = repo.users.get(&2).cloned().expect("user");
        user.password_hash = Some(hash_password("Voyager!Marble_2026").expect("hashed password"));
        repo.users.insert(2, user);
        repo
    }
}

use test_support::*;

// ============================================================================
// Auth API - Login Tests
// ============================================================================

#[rocket::async_test]
async fn auth_provider_discovery_defaults_to_password_only() {
    let client = test_client(base_repo()).await;
    let response = client.get("/api/auth/providers").dispatch().await;

    assert_eq!(response.status(), Status::Ok);
    let body: Value = response.into_json().await.expect("json");
    assert_eq!(body["password_enabled"], true);
    assert_eq!(body["external"], json!([]));
}

#[rocket::async_test]
async fn separate_apps_keep_independent_auth_configurations() {
    let password_client = test_client_with_auth(
        base_repo(),
        marreq_core::auth::AuthConfig::new(true, Vec::new()),
    )
    .await;
    let external_only_client = test_client_with_auth(
        base_repo(),
        marreq_core::auth::AuthConfig::new(false, Vec::new()),
    )
    .await;

    let password_discovery: Value = password_client
        .get("/api/auth/providers")
        .dispatch()
        .await
        .into_json()
        .await
        .expect("password discovery");
    let external_discovery: Value = external_only_client
        .get("/api/auth/providers")
        .dispatch()
        .await
        .into_json()
        .await
        .expect("external-only discovery");

    assert_eq!(password_discovery["password_enabled"], true);
    assert_eq!(external_discovery["password_enabled"], false);
}

#[rocket::async_test]
async fn deleting_unknown_external_identity_returns_not_found() {
    let client = test_client(base_repo()).await;
    let response = client
        .delete("/api/auth/identities/999")
        .private_cookie(session_cookie(&client, 2))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn deleting_another_users_identity_returns_not_found() {
    use marreq_core::repository::ExternalIdentityRepository;

    let mut repo = base_repo();
    let identity_id = repo
        .insert_identity(&NewUserIdentity {
            user_id: 1,
            provider_key: "github".into(),
            issuer: "https://github.com".into(),
            subject: "admin-subject".into(),
        })
        .expect("identity");
    let client = test_client(repo).await;
    let response = client
        .delete(format!("/api/auth/identities/{identity_id}"))
        .private_cookie(session_cookie(&client, 2))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn external_callback_rejects_missing_single_use_transaction() {
    let client = test_client(base_repo()).await;
    let response = client
        .get("/api/auth/external/github/callback?code=secret&state=wrong")
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
    let body: Value = response.into_json().await.expect("json");
    assert!(
        body["message"]
            .as_str()
            .unwrap_or_default()
            .contains("missing or already used")
    );
}

#[rocket::async_test]
async fn auth_login_returns_authenticated_user_and_sets_cookies() {
    let mut repo = base_repo();
    let mut admin = repo.users.get(&1).cloned().expect("admin user");
    admin.password_hash = Some(hash_password("Voyager!Marble_2026").expect("hashed password"));
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
async fn status_list_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/status").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn status_get_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/status/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn status_create_requires_authentication() {
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

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// Matrix API - Authentication Tests
// ============================================================================

#[rocket::async_test]
async fn matrix_list_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/matrix").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// Cache API - Authentication Tests
// ============================================================================

#[rocket::async_test]
async fn cache_stats_requires_admin_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/cache/stats").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn cache_clear_requires_admin_authentication() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/cache/clear")
        .header(ContentType::JSON)
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
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

// ============================================================================
// Auth API - Change password
// ============================================================================

#[rocket::async_test]
async fn change_password_requires_authentication() {
    let client = test_client(hashed_user_repo()).await;
    let response = client
        .post("/api/auth/change-password")
        .header(ContentType::JSON)
        .body(
            json!({
                "current_password": "Voyager!Marble_2026",
                "new_password": "Another!Strong_2026",
                "confirm_password": "Another!Strong_2026"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn change_password_rejects_mismatched_confirm() {
    let client = test_client(hashed_user_repo()).await;
    let cookie = session_cookie(&client, 2);
    let before = {
        let state = client.rocket().state::<TestAppState>().expect("state");
        state
            .repo_read()
            .inner_repo()
            .get_user_by_id(2)
            .expect("user")
            .password_hash
    };

    let response = client
        .post("/api/auth/change-password")
        .header(ContentType::JSON)
        .private_cookie(cookie)
        .body(
            json!({
                "current_password": "Voyager!Marble_2026",
                "new_password": "Another!Strong_2026",
                "confirm_password": "Mismatch!Strong_2026"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);

    let after = {
        let state = client.rocket().state::<TestAppState>().expect("state");
        state
            .repo_read()
            .inner_repo()
            .get_user_by_id(2)
            .expect("user")
            .password_hash
    };
    assert_eq!(before, after);
}

#[rocket::async_test]
async fn change_password_rejects_wrong_current_password() {
    let client = test_client(hashed_user_repo()).await;
    let cookie = session_cookie(&client, 2);
    let before = {
        let state = client.rocket().state::<TestAppState>().expect("state");
        state
            .repo_read()
            .inner_repo()
            .get_user_by_id(2)
            .expect("user")
            .password_hash
    };

    let response = client
        .post("/api/auth/change-password")
        .header(ContentType::JSON)
        .private_cookie(cookie)
        .body(
            json!({
                "current_password": "Wrong!Password_2026",
                "new_password": "Another!Strong_2026",
                "confirm_password": "Another!Strong_2026"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
    let body: Value = response.into_json().await.expect("json");
    assert!(
        body["message"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase()
            .contains("current password")
    );

    let after = {
        let state = client.rocket().state::<TestAppState>().expect("state");
        state
            .repo_read()
            .inner_repo()
            .get_user_by_id(2)
            .expect("user")
            .password_hash
    };
    assert_eq!(before, after);
}

#[rocket::async_test]
async fn change_password_rejects_policy_violation() {
    let client = test_client(hashed_user_repo()).await;
    let cookie = session_cookie(&client, 2);

    let response = client
        .post("/api/auth/change-password")
        .header(ContentType::JSON)
        .private_cookie(cookie)
        .body(
            json!({
                "current_password": "Voyager!Marble_2026",
                "new_password": "short",
                "confirm_password": "short"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
    let body: Value = response.into_json().await.expect("json");
    let msg = body["message"].as_str().unwrap_or_default();
    assert!(
        msg.to_lowercase().contains("at least"),
        "expected policy message, got {msg}"
    );
}

#[rocket::async_test]
async fn change_password_updates_hash_and_revokes_session() {
    let client = test_client(hashed_user_repo()).await;
    let cookie = session_cookie(&client, 2);
    let before = {
        let state = client.rocket().state::<TestAppState>().expect("state");
        state
            .repo_read()
            .inner_repo()
            .get_user_by_id(2)
            .expect("user")
            .password_hash
    };

    let response = client
        .post("/api/auth/change-password")
        .header(ContentType::JSON)
        .private_cookie(cookie.clone())
        .body(
            json!({
                "current_password": "Voyager!Marble_2026",
                "new_password": "Another!Strong_2026",
                "confirm_password": "Another!Strong_2026"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    let after = {
        let state = client.rocket().state::<TestAppState>().expect("state");
        state
            .repo_read()
            .inner_repo()
            .get_user_by_id(2)
            .expect("user")
            .password_hash
    };
    assert_ne!(before, after);
    assert!(
        after
            .as_deref()
            .is_some_and(|hash| hash.starts_with("$argon2"))
    );

    let me = client
        .get("/api/auth/me")
        .private_cookie(cookie)
        .dispatch()
        .await;
    assert_eq!(me.status(), Status::Unauthorized);
}

// ============================================================================
// Self-service profile (PUT /api/auth/me, issue #251)
// ============================================================================

fn profile_repo() -> marreq_core::repository::diesel_repo_mock::DieselRepoMock {
    let mut repo = base_repo();
    let admin = repo.users.get_mut(&1).unwrap();
    admin.email = "admin@example.com".into();
    let user = repo.users.get_mut(&2).unwrap();
    // "user" is a reserved namespace segment, which profile validation rejects.
    user.username = "jdoe".into();
    user.email = "user@example.com".into();
    user.password_hash = Some(hash_password("Orbit!Delta_2026").unwrap());
    repo
}

#[rocket::async_test]
async fn update_me_requires_session() {
    let client = test_client(profile_repo()).await;
    let response = client
        .put("/api/auth/me")
        .header(ContentType::JSON)
        .body(json!({ "name": "Someone", "email": "user@example.com" }).to_string())
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn update_me_changes_name_and_ignores_privileged_fields() {
    let client = test_client(profile_repo()).await;
    let response = client
        .put("/api/auth/me")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 2))
        .body(
            json!({
                "name": "Renamed User",
                "email": "user@example.com",
                "is_admin": true,
                "username": "hijack"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let body: Value = response.into_json().await.unwrap();
    assert_eq!(body["name"], "Renamed User");
    assert_eq!(body["username"], "jdoe");
    assert_eq!(body["is_admin"], false);
}

#[rocket::async_test]
async fn update_me_email_change_checks_password_and_conflicts() {
    let client = test_client(profile_repo()).await;
    let put = |body: Value| {
        let client = &client;
        async move {
            client
                .put("/api/auth/me")
                .header(ContentType::JSON)
                .private_cookie(session_cookie(client, 2))
                .body(body.to_string())
                .dispatch()
                .await
        }
    };

    let wrong =
        put(json!({ "name": "User", "email": "new@example.com", "current_password": "bad" })).await;
    assert_eq!(wrong.status(), Status::BadRequest);
    let body: Value = wrong.into_json().await.unwrap();
    assert_eq!(body["message"], "Current password is incorrect");

    let taken = put(json!({ "name": "User", "email": "admin@example.com", "current_password": "Orbit!Delta_2026" })).await;
    assert_eq!(taken.status(), Status::Conflict);

    let ok = put(json!({ "name": "User", "email": "New@Example.com", "current_password": "Orbit!Delta_2026" })).await;
    assert_eq!(ok.status(), Status::Ok);
    let body: Value = ok.into_json().await.unwrap();
    assert_eq!(body["email"], "new@example.com");
}

#[rocket::async_test]
async fn identities_report_password_configured_even_when_user_is_cached() {
    let client = test_client(profile_repo()).await;
    // Several requests: later ones resolve the session user from the cache,
    // which does not keep `password_hash`.
    for _ in 0..3 {
        let response = client
            .get("/api/auth/identities")
            .private_cookie(session_cookie(&client, 2))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: Value = response.into_json().await.unwrap();
        assert_eq!(body["password_configured"], true);
    }
}

// ============================================================================
// Session lifetimes (issue #285): absolute and idle limits, throttled touch
// ============================================================================

mod session_expiry {
    use super::*;
    use chrono::{Duration, NaiveDateTime, Utc};
    use marreq_core::auth::session::{BACKGROUND_REQUEST_HEADER, hash_token};
    use marreq_core::repository::diesel_repo_mock::DieselRepoMock;
    use rocket::http::Header;

    const RAW: &str = "session-expiry-test-token";

    /// A session of the admin, created `created_ago` and last seen `seen_ago`,
    /// with an `expires_at` far in the future (so only the new limits apply).
    fn repo_with_session(created_ago: Duration, seen_ago: Duration) -> DieselRepoMock {
        let mut repo = base_repo();
        let now = Utc::now().naive_utc();
        repo.sessions.push(Session {
            token_hash: hash_token(RAW),
            user_id: 1,
            created_at: now - created_ago,
            expires_at: now + Duration::days(365),
            last_seen_at: now - seen_ago,
            user_agent: None,
            ip_addr: None,
        });
        repo
    }

    fn cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(session_cookie_name_for_request(), RAW);
        cookie.set_path("/");
        cookie
    }

    async fn get(client: &Client, background: bool) -> Status {
        let mut request = client.get("/api/requirements").private_cookie(cookie());
        if background {
            request = request.header(Header::new(BACKGROUND_REQUEST_HEADER, "1"));
        }
        request.dispatch().await.status()
    }

    fn last_seen(client: &Client) -> Option<NaiveDateTime> {
        let state = client.rocket().state::<TestAppState>().unwrap();
        let repo = state.repo.read().unwrap();
        repo.inner_repo()
            .sessions
            .iter()
            .find(|s| s.token_hash == hash_token(RAW))
            .map(|s| s.last_seen_at)
    }

    #[rocket::async_test]
    async fn an_active_session_is_accepted() {
        let client = test_client(repo_with_session(Duration::days(2), Duration::minutes(5))).await;
        assert_eq!(get(&client, false).await, Status::Ok);
    }

    #[rocket::async_test]
    async fn an_idle_session_is_rejected_and_removed() {
        // Default idle limit: 8 hours.
        let client = test_client(repo_with_session(Duration::days(1), Duration::hours(9))).await;
        assert_eq!(get(&client, false).await, Status::Unauthorized);
        assert_eq!(last_seen(&client), None, "the rejected session is deleted");
    }

    #[rocket::async_test]
    async fn a_session_past_the_absolute_limit_is_rejected() {
        // Default absolute limit: 30 days, counted from created_at even though
        // expires_at is still in the future.
        let client = test_client(repo_with_session(Duration::days(31), Duration::minutes(1))).await;
        assert_eq!(get(&client, false).await, Status::Unauthorized);
    }

    #[rocket::async_test]
    async fn activity_is_recorded_at_most_once_a_minute() {
        let client = test_client(repo_with_session(Duration::days(1), Duration::seconds(10))).await;
        let before = last_seen(&client).unwrap();
        assert_eq!(get(&client, false).await, Status::Ok);
        assert_eq!(
            last_seen(&client),
            Some(before),
            "inside the throttle: no write"
        );

        let client = test_client(repo_with_session(Duration::days(1), Duration::minutes(5))).await;
        let before = last_seen(&client).unwrap();
        assert_eq!(get(&client, false).await, Status::Ok);
        let after = last_seen(&client).unwrap();
        assert!(after > before + Duration::minutes(4), "refreshed to now");
    }

    #[rocket::async_test]
    async fn background_requests_do_not_count_as_activity() {
        let client = test_client(repo_with_session(Duration::days(1), Duration::minutes(5))).await;
        let before = last_seen(&client).unwrap();
        assert_eq!(get(&client, true).await, Status::Ok, "still authenticated");
        assert_eq!(last_seen(&client), Some(before));
    }
}

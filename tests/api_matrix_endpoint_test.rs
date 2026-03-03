// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

//! Comprehensive integration tests for Matrix API endpoint `/api/matrix`.
//!
//! These tests verify the HTTP endpoint behavior including:
//! - Authentication requirements
//! - Response format
//! - Error handling
//! - Project filtering (if applicable)

use marreq::auth::session::SESSION_COOKIE;
use marreq::models::*;
use marreq::status_enums::ProjectStatus;
use rocket::http::{Cookie, Status};
use rocket::local::asynchronous::Client;
use serde_json::Value;

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
            },
        );

        repo
    }

    pub fn sample_matrix_link(req_id: i32, test_id: i32, project_id: i32) -> MatrixLink {
        MatrixLink {
            req_id,
            test_id,
            creation_date: timestamp(),
            project_id,
            suspect: false,
            suspect_at: None,
            suspect_reason: None,
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: None,
            triggering_user_id: None,
        }
    }
}

use test_support::*;

// ============================================================================
// GET /api/matrix - Authentication Tests (ASVS V8.2.1)
// ============================================================================

#[rocket::async_test]
async fn get_matrix_unauthenticated_returns_401() {
    // ASVS V8.2.1 regression: endpoint must reject callers with no credentials.
    let client = test_client(base_repo()).await;

    let response = client.get("/api/matrix").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn get_matrix_non_admin_returns_403() {
    // ASVS V8.2.1: non-admin authenticated users must not enumerate cross-project links.
    let client = test_client(base_repo()).await;

    // user id 2 exists in base_repo() as a regular (non-admin) user
    let response = client
        .get("/api/matrix")
        .private_cookie(session_cookie(2))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn get_matrix_with_admin_session_returns_ok() {
    let mut repo = base_repo();
    repo.matrices.push(sample_matrix_link(1, 1, 1));

    let client = test_client(repo).await;

    let response = client
        .get("/api/matrix")
        .private_cookie(session_cookie(1)) // admin user
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
}

// ============================================================================
// GET /api/matrix - Response Format Tests
// ============================================================================

#[rocket::async_test]
async fn get_matrix_returns_json_array() {
    let mut repo = base_repo();
    repo.matrices.push(sample_matrix_link(1, 1, 1));
    repo.matrices.push(sample_matrix_link(2, 2, 1));

    let client = test_client(repo).await;

    let response = client
        .get("/api/matrix")
        .private_cookie(session_cookie(1)) // admin
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let content_type = response.content_type().unwrap();
    assert_eq!(content_type.to_string(), "application/json");

    let body: Vec<Value> = response.into_json().await.expect("json array");
    assert_eq!(body.len(), 2);
}

// ============================================================================
// GET /api/matrix - Error Handling Tests
// ============================================================================

#[rocket::async_test]
async fn get_matrix_unknown_session_returns_401() {
    // A valid-looking cookie for a user that doesn't exist must yield 401.
    let client = test_client(base_repo()).await;

    let mut unknown_cookie = Cookie::new(SESSION_COOKIE, "999");
    unknown_cookie.set_path("/");

    let response = client
        .get("/api/matrix")
        .private_cookie(unknown_cookie)
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// GET /api/matrix - Empty Results Tests
// ============================================================================

#[rocket::async_test]
async fn get_matrix_returns_empty_array_when_no_links() {
    let repo = base_repo(); // admin present, no matrix links
    let client = test_client(repo).await;

    let response = client
        .get("/api/matrix")
        .private_cookie(session_cookie(1)) // admin
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let matrix: Vec<Value> = response.into_json().await.expect("json");
    assert_eq!(matrix.len(), 0);
}

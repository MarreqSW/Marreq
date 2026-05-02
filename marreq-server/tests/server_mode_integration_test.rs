// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Integration tests for server-mode deployment behavior:
//!
//! * Deployment metadata endpoint returns server-mode capability flags.
//! * Login succeeds even when `email_verified = false` (server mode ignores it).
//! * Cloud-only public auth routes are absent (404) in server mode.

use marreq_core::app::AppState;
use marreq_core::auth::hash_password;
use marreq_core::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};

type TestState = AppState<CacheRepository<DieselRepoMock>>;

fn state_from(repo: DieselRepoMock) -> TestState {
    AppState {
        repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
    }
}

async fn server_client_with(repo: DieselRepoMock) -> Client {
    // OnceLock ignores subsequent registrations — safe to call in parallel tests.
    marreq_core::deployment::install_test_server_mode();
    let rocket = rocket::build()
        .manage(state_from(repo))
        .manage(marreq_core::auth::rate_limiter::LoginRateLimiter::new())
        .mount("/api", marreq_core::api::routes());
    Client::tracked(rocket).await.unwrap()
}

async fn server_client() -> Client {
    server_client_with(DieselRepoMock::default()).await
}

// ============================================================================
// Deployment metadata
// ============================================================================

#[rocket::async_test]
async fn deployment_info_returns_server_mode_payload() {
    let client = server_client().await;
    let response = client.get("/api/meta/deployment").dispatch().await;

    assert_eq!(response.status(), Status::Ok);
    let body: Value = response.into_json().await.expect("json body");
    assert_eq!(body["mode"], "server");
    assert_eq!(body["allows_self_registration"], false);
    assert_eq!(body["requires_email_verification"], false);
    assert_eq!(body["allows_admin_promotion"], true);
    assert_eq!(body["allows_self_administered_user_creation"], true);
    assert_eq!(body["assigns_personal_workspace"], false);
}

// ============================================================================
// Login behavior
// ============================================================================

#[rocket::async_test]
async fn login_succeeds_despite_unverified_email_in_server_mode() {
    let mut repo = DieselRepoMock::default();
    let mut user = DieselRepoMock::make_user(1, "alice", "");
    user.password_hash = hash_password("Voyage!Silver_2026").expect("hash");
    user.email_verified = false; // not verified — server mode must not reject this
    repo.users.insert(1, user);

    let client = server_client_with(repo).await;

    let response = client
        .post("/api/auth/login")
        .header(ContentType::JSON)
        .body(json!({"username": "alice", "password": "Voyage!Silver_2026"}).to_string())
        .dispatch()
        .await;

    assert_eq!(
        response.status(),
        Status::Ok,
        "server mode must not gate login on email verification"
    );
    let body: Value = response.into_json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["user"]["username"], "alice");
}

// ============================================================================
// Cloud-only routes absent in server mode
// ============================================================================

#[rocket::async_test]
async fn cloud_register_route_is_absent_in_server_mode() {
    let client = server_client().await;
    let response = client
        .post("/api/auth/register")
        .header(ContentType::JSON)
        .body(
            json!({
                "username": "bob", "name": "Bob",
                "email": "bob@example.com", "password": "Secret!River_2026"
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn cloud_forgot_password_route_is_absent_in_server_mode() {
    let client = server_client().await;
    let response = client
        .post("/api/auth/forgot-password")
        .header(ContentType::JSON)
        .body(json!({"email": "ghost@example.com"}).to_string())
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn cloud_verify_email_route_is_absent_in_server_mode() {
    let client = server_client().await;
    let response = client
        .get("/api/auth/verify-email?token=irrelevant")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn cloud_reset_password_route_is_absent_in_server_mode() {
    let client = server_client().await;
    let response = client
        .post("/api/auth/reset-password")
        .header(ContentType::JSON)
        .body(json!({"token": "irrelevant", "new_password": "Another!Strong_2026"}).to_string())
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::NotFound);
}

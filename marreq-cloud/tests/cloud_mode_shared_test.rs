// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Integration tests for shared behavior under cloud deployment mode:
//!
//! * Deployment metadata endpoint returns cloud-mode capability flags.
//! * Login is blocked until `email_verified = true` in cloud mode.
//! * Login succeeds once the email verification step completes.
//! * `POST /api/users` (admin-managed creation) returns 410 Gone in cloud mode.

mod common;

use common::cloud_client;
use marreq_core::auth::session::test_session_cookie_for;
use marreq_core::repository::UserRepository;
use rocket::http::{ContentType, Status};
use serde_json::{json, Value};

// ============================================================================
// Deployment metadata
// ============================================================================

#[rocket::async_test]
async fn deployment_info_returns_cloud_mode_payload() {
    let client = cloud_client().await;
    let response = client.get("/api/meta/deployment").dispatch().await;

    assert_eq!(response.status(), Status::Ok);
    let body: Value = response.into_json().await.expect("json body");
    assert_eq!(body["mode"], "cloud");
    assert_eq!(body["allows_self_registration"], true);
    assert_eq!(body["requires_email_verification"], true);
    assert_eq!(body["allows_admin_promotion"], false);
    assert_eq!(body["allows_self_administered_user_creation"], false);
}

// ============================================================================
// Login gating: email verification required in cloud mode
// ============================================================================

#[rocket::async_test]
async fn login_blocked_until_email_verified_in_cloud_mode() {
    let client = cloud_client().await;

    // Registration creates an unverified account.
    let reg = client
        .post("/api/auth/register")
        .header(ContentType::JSON)
        .body(
            json!({
                "username": "alice",
                "name": "Alice Example",
                "email": "alice@example.com",
                "password": "CobaltRiver!Vacuum88",
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(reg.status(), Status::Created);

    let response = client
        .post("/api/auth/login")
        .header(ContentType::JSON)
        .body(json!({"username": "alice", "password": "CobaltRiver!Vacuum88"}).to_string())
        .dispatch()
        .await;

    assert_eq!(
        response.status(),
        Status::BadRequest,
        "cloud mode must block login for unverified users"
    );
    let body = response.into_string().await.unwrap_or_default();
    assert!(
        body.to_lowercase().contains("email address has not been verified"),
        "body should mention email verification, got: {body}"
    );
}

#[rocket::async_test]
async fn login_succeeds_after_email_verified_in_cloud_mode() {
    let client = cloud_client().await;

    // Register → unverified account.
    let reg = client
        .post("/api/auth/register")
        .header(ContentType::JSON)
        .body(
            json!({
                "username": "alice",
                "name": "Alice Example",
                "email": "alice@example.com",
                "password": "CobaltRiver!Vacuum88",
            })
            .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(reg.status(), Status::Created);

    // Confirm login is blocked before verification.
    let blocked = client
        .post("/api/auth/login")
        .header(ContentType::JSON)
        .body(json!({"username": "alice", "password": "CobaltRiver!Vacuum88"}).to_string())
        .dispatch()
        .await;
    assert_eq!(blocked.status(), Status::BadRequest);

    // Directly mark the account as verified through the in-memory mock repo.
    // The raw token is never stored — only its SHA-256 hash — so we bypass
    // the verify-email endpoint and set the flag directly.
    let state = common::app_state(&client);
    let user_id = {
        let repo = state.repo_read();
        repo.get_user_by_email("alice@example.com")
            .unwrap()
            .unwrap()
            .id
    };
    state
        .repo_write()
        .set_user_email_verified(user_id, true)
        .expect("repo should allow setting email_verified");

    // Login must now succeed.
    let login = client
        .post("/api/auth/login")
        .header(ContentType::JSON)
        .body(json!({"username": "alice", "password": "CobaltRiver!Vacuum88"}).to_string())
        .dispatch()
        .await;
    assert_eq!(
        login.status(),
        Status::Ok,
        "login should succeed after email verification"
    );
    let body: Value = login.into_json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["user"]["username"], "alice");
}

// ============================================================================
// Admin user creation disabled in cloud mode
// ============================================================================

#[rocket::async_test]
async fn admin_user_creation_via_api_returns_gone_in_cloud_mode() {
    std::env::set_var("MARREQ_SITE_ADMIN_EMAIL", "admin@cloud-test.example.com");
    std::env::set_var(
        "MARREQ_SITE_ADMIN_BOOTSTRAP_PASSWORD",
        "Admin!Bootstrap_2026",
    );

    let client = cloud_client().await;

    // The CloudAdminBootstrapFairing creates the admin at ignite.
    let state = common::app_state(&client);
    let admin_id = {
        let repo = state.repo_read();
        repo.get_user_by_email("admin@cloud-test.example.com")
            .expect("repo lookup")
            .expect("bootstrap fairing should have created the admin")
            .id
    };

    // Build a session cookie so the request is authenticated as admin.
    let auth_cookie = test_session_cookie_for(state, admin_id);

    let response = client
        .post("/api/users")
        .header(ContentType::JSON)
        .private_cookie(auth_cookie)
        .body(
            json!({
                "username": "bob",
                "name": "Bob",
                "email": "bob@example.com",
                "password": "Orbit!Delta_2026",
                "is_admin": false
            })
            .to_string(),
        )
        .dispatch()
        .await;

    assert_eq!(
        response.status(),
        Status::Gone,
        "POST /api/users must return 410 in cloud mode (admin-managed creation disabled)"
    );
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! End-to-end integration tests for the cloud-only public auth endpoints
//! mounted by `marreq_cloud::routes::routes()`:
//!
//! * `POST /api/auth/register`
//! * `GET  /api/auth/verify-email`
//! * `POST /api/auth/forgot-password`
//! * `POST /api/auth/reset-password`
//!
//! The tests drive Rocket through the production
//! [`marreq_core::app::build_with`] pipeline against the in-memory
//! `DieselRepoMock` repository (enabled via marreq-core's `test-helpers`
//! feature in `[dev-dependencies]`).

mod common;

use common::cloud_client;
use marreq_core::models::EmailToken;
use marreq_core::repository::UserRepository;
use rocket::http::{ContentType, Status};
use serde_json::{json, Value};

#[rocket::async_test]
async fn register_happy_path_returns_created_and_seeds_verification_token() {
    let client = cloud_client().await;

    let response = client
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

    assert_eq!(response.status(), Status::Created);
    let body: Value = response.into_json().await.expect("json body");
    assert_eq!(body, json!({ "status": "ok" }));

    // Cross-check repo state: a verification email-token should now exist.
    let state = common::app_state(&client);
    let repo = state.repo_read();
    let user = repo
        .get_user_by_email("alice@example.com")
        .expect("repo lookup")
        .expect("user inserted");
    assert!(!user.email_verified, "new account must start unverified");

    let tokens = &repo.inner_repo().email_tokens;
    assert!(
        tokens.iter().any(|t| t.user_id == user.id
            && t.purpose == EmailToken::PURPOSE_VERIFY_EMAIL
            && t.used_at.is_none()),
        "expected an unused verification token, got {tokens:?}"
    );
}

#[rocket::async_test]
async fn register_with_duplicate_email_returns_ok_for_anti_enumeration() {
    let client = cloud_client().await;

    // First registration: succeeds with 201.
    let first = client
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
    assert_eq!(first.status(), Status::Created);

    // Second registration reusing the same email but a different username
    // must not leak the conflict — surface a generic 200 ok.
    let second = client
        .post("/api/auth/register")
        .header(ContentType::JSON)
        .body(
            json!({
                "username": "alice2",
                "name": "Alice Two",
                "email": "alice@example.com",
                "password": "CobaltRiver!Vacuum88",
            })
            .to_string(),
        )
        .dispatch()
        .await;

    assert_eq!(second.status(), Status::Ok);
    let body: Value = second.into_json().await.expect("json body");
    assert_eq!(body, json!({ "status": "ok" }));
}

#[rocket::async_test]
async fn register_with_duplicate_username_but_new_email_returns_400() {
    let client = cloud_client().await;

    let first = client
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
    assert_eq!(first.status(), Status::Created);

    let second = client
        .post("/api/auth/register")
        .header(ContentType::JSON)
        .body(
            json!({
                "username": "alice",
                "name": "Alice Other",
                "email": "alice-other@example.com",
                "password": "CobaltRiver!Vacuum88",
            })
            .to_string(),
        )
        .dispatch()
        .await;

    assert_eq!(second.status(), Status::BadRequest);
    let body = second.into_string().await.unwrap_or_default();
    assert!(
        body.to_lowercase().contains("username already taken"),
        "expected body to mention the username conflict, got: {body}"
    );
}

#[rocket::async_test]
async fn verify_email_with_unknown_token_returns_400() {
    let client = cloud_client().await;

    let response = client
        .get("/api/auth/verify-email?token=not-a-real-token")
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
    let body = response.into_string().await.unwrap_or_default();
    assert!(
        body.to_lowercase()
            .contains("invalid or already-used token"),
        "expected body to mention the invalid token, got: {body}"
    );
}

#[rocket::async_test]
async fn forgot_password_returns_ok_for_unknown_email() {
    let client = cloud_client().await;

    let response = client
        .post("/api/auth/forgot-password")
        .header(ContentType::JSON)
        .body(json!({ "email": "ghost@example.com" }).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let body: Value = response.into_json().await.expect("json body");
    assert_eq!(body, json!({ "status": "ok" }));
}

#[rocket::async_test]
async fn forgot_password_returns_ok_for_existing_user_and_mints_reset_token() {
    let client = cloud_client().await;

    // Seed a user via the public registration endpoint.
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
        .post("/api/auth/forgot-password")
        .header(ContentType::JSON)
        .body(json!({ "email": "alice@example.com" }).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let body: Value = response.into_json().await.expect("json body");
    assert_eq!(body, json!({ "status": "ok" }));

    // A reset-password token should now exist alongside the verification token.
    let state = common::app_state(&client);
    let repo = state.repo_read();
    let user = repo
        .get_user_by_email("alice@example.com")
        .unwrap()
        .unwrap();
    let tokens = &repo.inner_repo().email_tokens;
    assert!(
        tokens.iter().any(|t| t.user_id == user.id
            && t.purpose == EmailToken::PURPOSE_RESET_PASSWORD
            && t.used_at.is_none()),
        "expected a password-reset token, got {tokens:?}"
    );
}

#[rocket::async_test]
async fn reset_password_with_invalid_token_returns_400() {
    let client = cloud_client().await;

    let response = client
        .post("/api/auth/reset-password")
        .header(ContentType::JSON)
        .body(
            json!({
                "token": "not-a-real-token",
                "new_password": "Another!Strong_2026",
            })
            .to_string(),
        )
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
    let body = response.into_string().await.unwrap_or_default();
    assert!(
        body.to_lowercase()
            .contains("invalid or already-used token"),
        "expected body to mention the invalid token, got: {body}"
    );
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

//! Integration tests for CSRF protection (ASVS V3.5.1).
//!
//! Verifies that the [`CsrfFairing`] correctly:
//! * Rejects cross-origin requests from authenticated sessions.
//! * Accepts same-origin requests (`Origin` header matches allowlist).
//! * Accepts requests carrying a valid `X-CSRF-Token` header (AJAX pattern).
//! * Never blocks Bearer-authenticated requests (API-token path).
//! * Never blocks safe HTTP methods (`GET`, `HEAD`, `OPTIONS`).
//! * Protects unauthenticated state-changing endpoints (`POST /login`).

#[macro_use]
extern crate rocket;

use marreq_core::auth::csrf::{CSRF_COOKIE, CSRF_HEADER};
use marreq_core::auth::session::SESSION_COOKIE;
use marreq_core::fairings::CsrfFairing;
use rocket::http::{ContentType, Cookie, Header, Status};
use rocket::local::asynchronous::Client;

// ---------------------------------------------------------------------------
// Test Rocket setup
// ---------------------------------------------------------------------------

const TEST_ALLOWED_ORIGIN: &str = "http://test.local";
const EVIL_ORIGIN: &str = "http://evil.example.com";
const TOKEN: &str = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";

/// Minimal POST handler used as a proxy for any real state-changing endpoint.
#[post("/protected")]
fn protected_post() -> &'static str {
    "ok"
}

/// Simulate an unauthenticated POST endpoint (like /login).
#[post("/unauthenticated_post")]
fn unauthenticated_post() -> &'static str {
    "ok"
}

/// Build an async test client with the CsrfFairing attached.
async fn test_client() -> Client {
    let rocket = rocket::build()
        .mount(
            "/",
            routes![
                protected_post,
                unauthenticated_post,
                marreq_core::fairings::csrf_denied
            ],
        )
        .attach(CsrfFairing::new().with_origin(TEST_ALLOWED_ORIGIN));
    Client::tracked(rocket).await.expect("valid rocket")
}

// ---------------------------------------------------------------------------
// Helper – private session cookie (simulates an authenticated session)
// ---------------------------------------------------------------------------

fn session_cookie() -> Cookie<'static> {
    let mut c = Cookie::new(SESSION_COOKIE, "1");
    c.set_path("/");
    c
}

fn csrf_cookie() -> Cookie<'static> {
    let mut c = Cookie::new(CSRF_COOKIE, TOKEN);
    c.set_path("/");
    c
}

// ---------------------------------------------------------------------------
// Safe HTTP methods – must never be blocked
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn get_request_is_never_blocked() {
    let client = test_client().await;
    // Even with a foreign Origin on a GET request, CSRF must not interfere.
    let response = client
        .get("/protected")
        .header(Header::new("Origin", EVIL_ORIGIN))
        .dispatch()
        .await;
    // Route doesn't exist (it's POST-only), but we should NOT get 403.
    assert_ne!(response.status(), Status::Forbidden);
}

// ---------------------------------------------------------------------------
// Authenticated POST – Origin header validation
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn authenticated_post_with_matching_origin_is_allowed() {
    let client = test_client().await;
    let response = client
        .post("/protected")
        .header(ContentType::Form)
        .header(Header::new("Origin", TEST_ALLOWED_ORIGIN))
        .private_cookie(session_cookie())
        .body("") // empty form body
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn authenticated_post_with_mismatched_origin_is_forbidden() {
    let client = test_client().await;
    let response = client
        .post("/protected")
        .header(ContentType::Form)
        .header(Header::new("Origin", EVIL_ORIGIN))
        .private_cookie(session_cookie())
        .body("")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn authenticated_post_without_origin_is_forbidden() {
    let client = test_client().await;
    // Session cookie present, no Origin, no CSRF token → must reject.
    let response = client
        .post("/protected")
        .header(ContentType::Form)
        .private_cookie(session_cookie())
        .body("")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
}

// ---------------------------------------------------------------------------
// AJAX path – X-CSRF-Token header validation
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn post_with_valid_csrf_header_is_allowed() {
    let client = test_client().await;
    // No Origin header, but X-CSRF-Token matches the csrf cookie.
    let response = client
        .post("/protected")
        .header(ContentType::JSON)
        .header(Header::new(CSRF_HEADER, TOKEN))
        .private_cookie(session_cookie())
        .private_cookie(csrf_cookie())
        .body("{}")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn post_with_wrong_csrf_header_is_forbidden() {
    let client = test_client().await;
    let response = client
        .post("/protected")
        .header(ContentType::JSON)
        .header(Header::new(CSRF_HEADER, "wrong-token-value"))
        .private_cookie(session_cookie())
        .private_cookie(csrf_cookie())
        .body("{}")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn post_with_mismatched_csrf_header_overrides_valid_origin() {
    // Token present but mismatched → reject, even with a valid Origin.
    let client = test_client().await;
    let response = client
        .post("/protected")
        .header(ContentType::JSON)
        .header(Header::new("Origin", TEST_ALLOWED_ORIGIN))
        .header(Header::new(CSRF_HEADER, "tampered-token"))
        .private_cookie(session_cookie())
        .private_cookie(csrf_cookie())
        .body("{}")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
}

// ---------------------------------------------------------------------------
// Bearer auth – must bypass CSRF (API-token path)
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn bearer_auth_post_without_origin_is_allowed() {
    let client = test_client().await;
    let response = client
        .post("/protected")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", "Bearer some-api-token"))
        .body("{}")
        .dispatch()
        .await;
    // CSRF guard must not block; route returns 200.
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn bearer_auth_post_with_evil_origin_is_allowed() {
    // Origin header is irrelevant for Bearer auth.
    let client = test_client().await;
    let response = client
        .post("/protected")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", "Bearer some-api-token"))
        .header(Header::new("Origin", EVIL_ORIGIN))
        .body("{}")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}

// ---------------------------------------------------------------------------
// Unauthenticated POST (login CSRF)
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn login_post_with_same_origin_is_allowed() {
    let client = test_client().await;
    // No session cookie – simulates a fresh login attempt.
    let response = client
        .post("/unauthenticated_post")
        .header(ContentType::Form)
        .header(Header::new("Origin", TEST_ALLOWED_ORIGIN))
        .body("username=alice&password=s3cr3t")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn login_post_with_cross_site_origin_is_forbidden() {
    let client = test_client().await;
    let response = client
        .post("/unauthenticated_post")
        .header(ContentType::Form)
        .header(Header::new("Origin", EVIL_ORIGIN))
        .body("username=alice&password=s3cr3t")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn login_post_without_origin_and_no_session_is_allowed() {
    // Headless clients (e.g. integration test runners, curl) legitimately
    // lack an Origin header and have no session – they must not be blocked.
    let client = test_client().await;
    let response = client
        .post("/unauthenticated_post")
        .header(ContentType::Form)
        .body("username=alice&password=s3cr3t")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}

// ---------------------------------------------------------------------------
// Referer fallback
// ---------------------------------------------------------------------------

#[rocket::async_test]
async fn authenticated_post_with_matching_referer_is_allowed() {
    let client = test_client().await;
    let referer_value = format!("{}/some/page", TEST_ALLOWED_ORIGIN);
    let response = client
        .post("/protected")
        .header(ContentType::Form)
        .header(Header::new("Referer", referer_value))
        .private_cookie(session_cookie())
        .body("")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn authenticated_post_with_mismatched_referer_is_forbidden() {
    let client = test_client().await;
    let referer_value = format!("{}/steal", EVIL_ORIGIN);
    let response = client
        .post("/protected")
        .header(ContentType::Form)
        .header(Header::new("Referer", referer_value))
        .private_cookie(session_cookie())
        .body("")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
}

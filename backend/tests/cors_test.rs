// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

//! Integration tests for the CORS fairing (ASVS V3.4.2).
//!
//! Verifies:
//! - Allowed origin receives `Access-Control-Allow-Origin` echoing the origin exactly.
//! - Allowed origin never triggers a wildcard response.
//! - `Vary: Origin` is present when an origin is echoed.
//! - Disallowed origin receives no CORS headers.
//! - Request without `Origin` header receives no CORS headers.
//! - `OPTIONS` preflight for an allowed origin returns `204` with
//!   `Access-Control-Allow-Methods` and `Access-Control-Allow-Headers`.
//! - `OPTIONS` preflight for a disallowed origin receives no CORS headers.
//! - `Access-Control-Allow-Credentials` is present iff the policy enables it.

use marreq::cors::{CorsFairing, CorsPolicy};
use rocket::http::{Header, Status};
use rocket::local::asynchronous::Client;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Minimal route used as a CORS target.
#[rocket::get("/ping")]
fn ping() -> &'static str {
    "pong"
}

/// Build a test Rocket instance with the given CORS policy attached.
async fn make_client(allowed_origins: &[&str], allow_credentials: bool) -> Client {
    let policy = CorsPolicy::new(
        allowed_origins.iter().map(|s| s.to_string()).collect(),
        allow_credentials,
    );
    let rocket = rocket::build()
        .attach(CorsFairing(policy))
        .mount("/", rocket::routes![ping]);
    Client::tracked(rocket).await.expect("rocket instance")
}

const ALLOWED: &str = "https://app.example.com";
const DISALLOWED: &str = "https://evil.example.com";

// ===========================================================================
// Allowed origin
// ===========================================================================

#[rocket::async_test]
async fn allowed_origin_receives_acao_header() {
    let client = make_client(&[ALLOWED], false).await;
    let response = client
        .get("/ping")
        .header(Header::new("Origin", ALLOWED))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Access-Control-Allow-Origin"),
        Some(ALLOWED),
        "Allowed origin must be echoed in Access-Control-Allow-Origin"
    );
}

#[rocket::async_test]
async fn allowed_origin_never_uses_wildcard() {
    let client = make_client(&[ALLOWED], false).await;
    let response = client
        .get("/ping")
        .header(Header::new("Origin", ALLOWED))
        .dispatch()
        .await;

    let acao = response.headers().get_one("Access-Control-Allow-Origin");
    assert_ne!(
        acao,
        Some("*"),
        "ACAO header must never be the wildcard `*`"
    );
    assert_eq!(acao, Some(ALLOWED));
}

#[rocket::async_test]
async fn allowed_origin_has_vary_origin() {
    let client = make_client(&[ALLOWED], false).await;
    let response = client
        .get("/ping")
        .header(Header::new("Origin", ALLOWED))
        .dispatch()
        .await;

    let vary = response.headers().get_one("Vary");
    assert!(
        vary.is_some_and(|v| v.contains("Origin")),
        "Vary header must contain `Origin` when the origin is echoed, got: {:?}",
        vary
    );
}

// ===========================================================================
// Disallowed / absent origin
// ===========================================================================

#[rocket::async_test]
async fn disallowed_origin_receives_no_cors_headers() {
    let client = make_client(&[ALLOWED], false).await;
    let response = client
        .get("/ping")
        .header(Header::new("Origin", DISALLOWED))
        .dispatch()
        .await;

    assert_eq!(
        response.headers().get_one("Access-Control-Allow-Origin"),
        None,
        "Disallowed origin must not receive an ACAO header"
    );
}

#[rocket::async_test]
async fn no_origin_header_produces_no_cors_headers() {
    let client = make_client(&[ALLOWED], false).await;
    let response = client.get("/ping").dispatch().await;

    assert_eq!(
        response.headers().get_one("Access-Control-Allow-Origin"),
        None,
        "Request without Origin must not receive an ACAO header"
    );
}

// ===========================================================================
// Preflight (OPTIONS)
// ===========================================================================

#[rocket::async_test]
async fn preflight_allowed_origin_returns_204() {
    let client = make_client(&[ALLOWED], false).await;
    let response = client
        .options("/nonexistent-route")
        .header(Header::new("Origin", ALLOWED))
        .header(Header::new("Access-Control-Request-Method", "POST"))
        .dispatch()
        .await;

    assert_eq!(
        response.status(),
        Status::NoContent,
        "Preflight for allowed origin must return 204 No Content"
    );
}

#[rocket::async_test]
async fn preflight_allowed_origin_has_allow_methods() {
    let client = make_client(&[ALLOWED], false).await;
    let response = client
        .options("/nonexistent-route")
        .header(Header::new("Origin", ALLOWED))
        .header(Header::new("Access-Control-Request-Method", "DELETE"))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NoContent);
    assert!(
        response
            .headers()
            .get_one("Access-Control-Allow-Methods")
            .is_some(),
        "Preflight response must include Access-Control-Allow-Methods"
    );
}

#[rocket::async_test]
async fn preflight_allowed_origin_has_allow_headers() {
    let client = make_client(&[ALLOWED], false).await;
    let response = client
        .options("/nonexistent-route")
        .header(Header::new("Origin", ALLOWED))
        .header(Header::new("Access-Control-Request-Method", "POST"))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NoContent);
    assert!(
        response
            .headers()
            .get_one("Access-Control-Allow-Headers")
            .is_some(),
        "Preflight response must include Access-Control-Allow-Headers"
    );
}

#[rocket::async_test]
async fn preflight_allowed_origin_has_max_age() {
    let client = make_client(&[ALLOWED], false).await;
    let response = client
        .options("/nonexistent-route")
        .header(Header::new("Origin", ALLOWED))
        .header(Header::new("Access-Control-Request-Method", "GET"))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NoContent);
    assert!(
        response
            .headers()
            .get_one("Access-Control-Max-Age")
            .is_some(),
        "Preflight response must include Access-Control-Max-Age"
    );
}

#[rocket::async_test]
async fn preflight_allowed_origin_has_vary_origin() {
    let client = make_client(&[ALLOWED], false).await;
    let response = client
        .options("/nonexistent-route")
        .header(Header::new("Origin", ALLOWED))
        .header(Header::new("Access-Control-Request-Method", "GET"))
        .dispatch()
        .await;

    let vary = response.headers().get_one("Vary");
    assert!(
        vary.is_some_and(|v| v.contains("Origin")),
        "Preflight response must contain Vary: Origin, got: {:?}",
        vary
    );
}

#[rocket::async_test]
async fn preflight_disallowed_origin_produces_no_cors_headers() {
    let client = make_client(&[ALLOWED], false).await;
    let response = client
        .options("/nonexistent-route")
        .header(Header::new("Origin", DISALLOWED))
        .header(Header::new("Access-Control-Request-Method", "POST"))
        .dispatch()
        .await;

    assert_eq!(
        response.headers().get_one("Access-Control-Allow-Origin"),
        None,
        "Preflight for disallowed origin must not receive ACAO header"
    );
}

// ===========================================================================
// Credentials policy
// ===========================================================================

#[rocket::async_test]
async fn credentials_header_present_when_configured() {
    let client = make_client(&[ALLOWED], true).await;
    let response = client
        .get("/ping")
        .header(Header::new("Origin", ALLOWED))
        .dispatch()
        .await;

    assert_eq!(
        response
            .headers()
            .get_one("Access-Control-Allow-Credentials"),
        Some("true"),
        "Access-Control-Allow-Credentials must be `true` when policy enables it"
    );
}

#[rocket::async_test]
async fn credentials_header_absent_when_not_configured() {
    let client = make_client(&[ALLOWED], false).await;
    let response = client
        .get("/ping")
        .header(Header::new("Origin", ALLOWED))
        .dispatch()
        .await;

    assert_eq!(
        response
            .headers()
            .get_one("Access-Control-Allow-Credentials"),
        None,
        "Access-Control-Allow-Credentials must be absent when policy disables it"
    );
}

// ===========================================================================
// Empty allowlist (production safe-default)
// ===========================================================================

#[rocket::async_test]
async fn empty_allowlist_produces_no_cors_headers_for_any_origin() {
    let client = make_client(&[], false).await;
    let response = client
        .get("/ping")
        .header(Header::new("Origin", ALLOWED))
        .dispatch()
        .await;

    assert_eq!(
        response.headers().get_one("Access-Control-Allow-Origin"),
        None,
        "Empty allowlist must produce no CORS headers regardless of origin"
    );
}

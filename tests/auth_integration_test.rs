// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

//! Comprehensive integration tests for Authentication workflows.
//!
//! These tests verify the complete behavior of authentication endpoints including:
//! - Login (success/failure)
//! - Logout
//! - Password changes
//! - Session management
//! - Security validations

use rocket::http::{ContentType, Cookie, SameSite, Status};
use rocket::local::asynchronous::Client;

mod test_support {
    use super::*;
    use marreq::app::AppState;
    use marreq::auth::hash_password;
    use marreq::auth::session::SESSION_COOKIE;
    use marreq::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use std::sync::{Arc, RwLock};

    pub type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

    pub fn managed_state(repo: DieselRepoMock) -> TestAppState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    pub async fn test_client(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(managed_state(repo))
            .manage(marreq::auth::rate_limiter::LoginRateLimiter::new())
            .attach(rocket_dyn_templates::Template::fairing())
            .mount("/", marreq::routes::html::auth::routes())
            .mount("/api", marreq::api::routes()); // Mount API for verification if needed

        Client::tracked(rocket).await.expect("rocket instance")
    }

    pub fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        // Create a user with known password hash
        // "secret" -> hashed
        let pwd_hash = hash_password("secret").expect("hash");
        let mut user = DieselRepoMock::make_user(1, "alice", &pwd_hash);
        user.is_admin = true;
        repo.users.insert(1, user);

        repo
    }

    pub fn session_cookie(user_id: i32) -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, user_id.to_string());
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(SameSite::Strict);
        cookie
    }
}

use test_support::*;

// ============================================================================
// POST /login
// ============================================================================

#[rocket::async_test]
async fn login_success_redirects_to_dashboard() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/login")
        .header(ContentType::Form)
        .body("username=alice&password=secret")
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    // Dashboard is at root "/"
    assert_eq!(response.headers().get_one("Location"), Some("/"));

    // Verify session cookie is set (name is "session" on HTTP, "__Host-session" on HTTPS)
    let cookie_name = marreq::auth::session::session_cookie_name_for_request();
    let cookie = response.cookies().get(cookie_name);
    assert!(cookie.is_some());

    // Verify session works by making an authenticated request
    let user_response = client.get("/api/users/1").dispatch().await;

    assert_eq!(user_response.status(), Status::Ok);
}

#[rocket::async_test]
async fn login_failure_redirects_with_error() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/login")
        .header(ContentType::Form)
        .body("username=alice&password=wrong")
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("/login"));
    assert!(location.contains("error=Invalid%20username%20or%20password"));

    // Verify no session cookie (check both possible names)
    assert!(response
        .cookies()
        .get(marreq::auth::session::SESSION_COOKIE)
        .is_none());
    assert!(response.cookies().get("session").is_none());
}

#[rocket::async_test]
async fn login_unknown_user_fails() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/login")
        .header(ContentType::Form)
        .body("username=bob&password=secret")
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error=Invalid%20username%20or%20password"));
}

#[rocket::async_test]
async fn login_page_uses_masked_password_with_password_manager_autocomplete() {
    let client = test_client(base_repo()).await;

    let response = client.get("/login").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let html = response.into_string().await.expect("body");
    assert!(html.contains("type=\"password\""));
    assert!(html.contains("autocomplete=\"current-password\""));
}

// ============================================================================
// POST /logout
// ============================================================================

#[rocket::async_test]
async fn logout_clears_session() {
    let client = test_client(base_repo()).await;

    // Manually set session cookie to simulate logged in state
    let response = client
        .post("/logout")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    assert!(response
        .headers()
        .get_one("Location")
        .unwrap()
        .contains("/login"));

    // Verify session cookie is cleared (expired)
    let cookie = response
        .cookies()
        .get(marreq::auth::session::SESSION_COOKIE);
    assert!(cookie.is_some());
    // Rocket sets expiration to past to clear it
    assert!(cookie.unwrap().expires().is_some());
}

// ============================================================================
// POST /change_password
// ============================================================================

#[rocket::async_test]
async fn change_password_success() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/change_password")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body("current_password=secret&new_password=CobaltRiver%21Vacuum88&confirm_password=CobaltRiver%21Vacuum88")
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("success=Password%20changed%20successfully"));

    // Verify login with new password works (requires checking repo or logging in again)
    // Since we use mock repo, the change is in memory in the app state
    // Let's try to login with new password

    let login_response = client
        .post("/login")
        .header(ContentType::Form)
        .body("username=alice&password=CobaltRiver%21Vacuum88")
        .dispatch()
        .await;

    assert_eq!(login_response.status(), Status::SeeOther);
    assert_eq!(login_response.headers().get_one("Location"), Some("/"));
}

#[rocket::async_test]
async fn change_password_mismatch_fails() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/change_password")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body(
            "current_password=secret&new_password=CobaltRiver%21Vacuum88&confirm_password=mismatch",
        )
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error=New%20passwords%20do%20not%20match"));
}

#[rocket::async_test]
async fn change_password_too_short_fails() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/change_password")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body("current_password=secret&new_password=short&confirm_password=short")
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error=Password%20must%20be%20at%20least%208%20characters%20long"));
}

#[rocket::async_test]
async fn change_password_wrong_current_fails() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/change_password")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body("current_password=wrong&new_password=CobaltRiver%21Vacuum88&confirm_password=CobaltRiver%21Vacuum88")
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error=Invalid%20current%20password"));
}

#[rocket::async_test]
async fn change_password_requires_login() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/change_password")
        .header(ContentType::Form)
        .body("current_password=secret&new_password=CobaltRiver%21Vacuum88&confirm_password=CobaltRiver%21Vacuum88")
        .dispatch()
        .await;

    // Should redirect to login or return error depending on guard
    // The route handler takes cookies and state, but calls change_password_hash which checks auth
    // If the guard fails (if used), it might be 401.
    // But here the route signature is:
    // pub fn change_password(password_form: Form<ChangePasswordForm>, cookies: &CookieJar<'_>, state: &State<AppState>)
    // It doesn't use ApiUser guard, so it executes and change_password_hash returns NotLoggedIn

    assert_eq!(response.status(), Status::SeeOther);
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error=Not%20logged%20in"));
}

#[rocket::async_test]
async fn change_password_page_uses_masked_inputs_and_autocomplete_hints() {
    let client = test_client(base_repo()).await;

    let response = client.get("/change_password").dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    let html = response.into_string().await.expect("body");
    assert!(html.contains("id=\"current_password\""));
    assert!(html.contains("autocomplete=\"current-password\""));
    assert!(html.contains("id=\"new_password\""));
    assert!(html.contains("autocomplete=\"new-password\""));
}

#[rocket::async_test]
async fn change_password_rejects_common_password() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/change_password")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body("current_password=secret&new_password=password1&confirm_password=password1")
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response.headers().get_one("Location").unwrap();
    assert!(location
        .contains("error=Password%20is%20too%20common.%20Choose%20a%20more%20unique%20password"));
}

#[rocket::async_test]
async fn change_password_rejects_context_specific_password() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/change_password")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body("current_password=secret&new_password=alice-strong-passphrase&confirm_password=alice-strong-passphrase")
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response.headers().get_one("Location").unwrap();
    assert!(location.contains("error=Password%20must%20not%20contain%20context-specific%20terms"));
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

#![cfg(feature = "test-helpers")]

//! Comprehensive integration tests for Authentication workflows.
//!
//! These tests verify the complete behavior of authentication endpoints including:
//! - Login (success/failure)
//! - Logout
//! - Password changes
//! - Session management
//! - Security validations

use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;

mod test_support {
    use super::*;
    use req_man::app::AppState;
    use req_man::auth::hash_password;
    use req_man::auth::session::SESSION_COOKIE;
    use req_man::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
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
            .attach(rocket_dyn_templates::Template::fairing())
            .mount("/", req_man::routes::html::auth::routes())
            .mount("/api", req_man::api::routes()); // Mount API for verification if needed

        Client::tracked(rocket).await.expect("rocket instance")
    }

    pub fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        // Create a user with known password hash
        // "secret" -> hashed
        let pwd_hash = hash_password("secret").expect("hash");
        let user = DieselRepoMock::make_user(1, "alice", &pwd_hash);
        repo.users.insert(1, user);

        repo
    }

    pub fn session_cookie(user_id: i32) -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, user_id.to_string());
        cookie.set_path("/");
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

    // Verify session cookie is set
    let cookie = response
        .cookies()
        .get(req_man::auth::session::SESSION_COOKIE);
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

    // Verify no session cookie
    let cookie = response
        .cookies()
        .get(req_man::auth::session::SESSION_COOKIE);
    assert!(cookie.is_none());
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

// ============================================================================
// GET /logout
// ============================================================================

#[rocket::async_test]
async fn logout_clears_session() {
    let client = test_client(base_repo()).await;

    // Manually set session cookie to simulate logged in state
    let response = client
        .get("/logout")
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
        .get(req_man::auth::session::SESSION_COOKIE);
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
        .body("current_password=secret&new_password=newsecret123&confirm_password=newsecret123")
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
        .body("username=alice&password=newsecret123")
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
        .body("current_password=secret&new_password=newsecret123&confirm_password=mismatch")
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
    assert!(location.contains("error=New%20password%20must%20be%20at%20least%208%20characters"));
}

#[rocket::async_test]
async fn change_password_wrong_current_fails() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/change_password")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body("current_password=wrong&new_password=newsecret123&confirm_password=newsecret123")
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
        .body("current_password=secret&new_password=newsecret123&confirm_password=newsecret123")
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

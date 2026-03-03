// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

//! Integration tests for security-related HTTP headers (ASVS V14.3).
//!
//! Verifies:
//! - Anti-caching headers on authenticated HTML responses (V14.3.2)
//! - `Clear-Site-Data` header on logout responses (V14.3.1)
//! - Static assets are NOT affected by anti-caching headers

use rocket::http::{ContentType, Status};

mod support {
    use marreq::app::AppState;
    use marreq::auth::hash_password;
    use marreq::auth::session::SESSION_COOKIE;
    use marreq::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::Cookie;
    use rocket::local::asynchronous::Client;
    use std::sync::{Arc, RwLock};

    pub fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();
        let pwd_hash = hash_password("secret").expect("hash");
        let user = DieselRepoMock::make_user(1, "alice", &pwd_hash);
        repo.users.insert(1, user);
        repo
    }

    /// Build a test Rocket instance that mirrors production: auth routes + fairing.
    pub async fn test_client(repo: DieselRepoMock) -> Client {
        let state = AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        };

        let rocket = rocket::build()
            .manage(state)
            .attach(marreq::fairings::AntiCacheFairing)
            .attach(rocket_dyn_templates::Template::fairing())
            .mount("/", marreq::routes::html::auth::routes())
            .mount(
                "/static",
                rocket::fs::FileServer::from(rocket::fs::relative!("src/html/static")),
            );

        Client::tracked(rocket).await.expect("rocket instance")
    }

    pub fn session_cookie(user_id: i32) -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, user_id.to_string());
        cookie.set_path("/");
        cookie
    }
}

use support::*;

// ============================================================================
// Anti-caching headers (ASVS V14.3.2)
// ============================================================================

#[rocket::async_test]
async fn login_page_has_anti_cache_headers() {
    let client = test_client(base_repo()).await;

    let response = client.get("/login").dispatch().await;

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Cache-Control"),
        Some("no-store"),
        "Login page must have Cache-Control: no-store"
    );
    assert_eq!(
        response.headers().get_one("Pragma"),
        Some("no-cache"),
        "Login page must have Pragma: no-cache"
    );
    assert_eq!(
        response.headers().get_one("Expires"),
        Some("0"),
        "Login page must have Expires: 0"
    );
}

#[rocket::async_test]
async fn redirect_after_login_has_anti_cache_headers() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/login")
        .header(ContentType::Form)
        .body("username=alice&password=secret")
        .dispatch()
        .await;

    // Successful login redirects (3xx)
    assert!(response.status().code >= 300 && response.status().code < 400);
    assert_eq!(
        response.headers().get_one("Cache-Control"),
        Some("no-store"),
        "Login redirect must have Cache-Control: no-store"
    );
}

#[rocket::async_test]
async fn change_password_page_has_anti_cache_headers() {
    let client = test_client(base_repo()).await;

    let response = client.get("/change_password").dispatch().await;

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Cache-Control"),
        Some("no-store"),
    );
    assert_eq!(response.headers().get_one("Pragma"), Some("no-cache"));
    assert_eq!(response.headers().get_one("Expires"), Some("0"));
}

// ============================================================================
// Clear-Site-Data on logout (ASVS V14.3.1)
// ============================================================================

#[rocket::async_test]
async fn logout_has_clear_site_data_header() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/logout")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    assert!(
        response
            .headers()
            .get_one("Location")
            .unwrap()
            .contains("/login"),
        "Logout must redirect to login"
    );

    let csd = response
        .headers()
        .get_one("Clear-Site-Data")
        .expect("Logout must include Clear-Site-Data header");
    assert!(
        csd.contains("\"cache\""),
        "Clear-Site-Data must include \"cache\""
    );
    assert!(
        csd.contains("\"cookies\""),
        "Clear-Site-Data must include \"cookies\""
    );
    assert!(
        csd.contains("\"storage\""),
        "Clear-Site-Data must include \"storage\""
    );
}

#[rocket::async_test]
async fn logout_also_has_anti_cache_headers() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/logout")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(
        response.headers().get_one("Cache-Control"),
        Some("no-store"),
        "Logout redirect must also have Cache-Control: no-store"
    );
}

// ============================================================================
// Static assets must NOT get anti-cache headers
// ============================================================================

#[rocket::async_test]
async fn static_assets_no_anti_cache_headers() {
    let client = test_client(base_repo()).await;

    // Try to fetch a static asset (even if 404, the fairing should not add headers)
    let response = client.get("/static/marreq.css").dispatch().await;

    // If the file exists we get 200, if not 404 — either way, no anti-cache
    let cache_control = response.headers().get_one("Cache-Control");
    assert!(
        cache_control.is_none() || cache_control != Some("no-store"),
        "Static assets must NOT have Cache-Control: no-store"
    );
}

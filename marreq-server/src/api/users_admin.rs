// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Admin user-management — tests for the create / delete happy paths.
//!
//! The actual route handlers (`POST /api/users`, `DELETE /api/users/<id>`)
//! live in [`marreq_core::api::users`] and are mounted unconditionally as
//! part of core's base route set. The runtime guard
//! [`marreq_core::deployment::current()`]`.allows_self_administered_user_creation()`
//! returns 410 Gone in deployment modes where admin user creation is disabled
//! (e.g. cloud), so no compile-time `#[cfg]` gate is needed.
//!
//! This module is the home for any additional server-only admin
//! user-management handlers we add to the `marreq-server` crate.

#[cfg(test)]
mod tests {
    use marreq_core::api::users::{create, delete, get, list};
    use marreq_core::app::AppState;
    use marreq_core::auth::session::SESSION_COOKIE;
    use marreq_core::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::{ContentType, Cookie, SameSite, Status};
    use rocket::local::asynchronous::Client;
    use rocket::routes;
    use serde_json::{json, Value};
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        // Register a Server-equivalent deployment mode so that
        // `deployment::current()` works inside the `create` handler.  OnceLock
        // silently ignores subsequent calls, so this is safe across parallel
        // tests.
        marreq_core::deployment::install_test_server_mode();
        let rocket = rocket::build()
            .manage(state_from_repo(repo.with_admin_user()))
            .mount("/api", routes![list, get, create, delete]);
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(SameSite::Strict);
        cookie
    }

    #[rocket::async_test]
    async fn create_returns_new_identifier() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .post("/api/users")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(
                json!({
                    "username": "bob",
                    "name": "Bob",
                    "email": "bob@example.com",
                    "password": "Skyline!Current_2026",
                    "is_admin": false
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("status"), Some(&Value::from("ok")));
        assert_eq!(payload.get("id"), Some(&Value::from(2)));
    }

    #[rocket::async_test]
    async fn delete_removes_existing_user() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_response = client
            .post("/api/users")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(
                json!({
                    "username": "carol",
                    "name": "Carol",
                    "email": "carol@example.com",
                    "password": "Orbit!Delta_2026",
                    "is_admin": false
                })
                .to_string(),
            )
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let delete_response = client
            .delete(format!("/api/users/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(delete_response.status(), Status::NoContent);

        let not_found = client
            .get(format!("/api/users/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(not_found.status(), Status::NotFound);
    }
}

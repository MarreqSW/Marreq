// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::api::prelude::*;
use crate::models::{User, UserCreateRequest};
use crate::services::UserService;

/// `GET /api/users` — list all users. Restricted to administrators (ASVS V8.2.1).
#[get("/users")]
pub async fn list(_admin: AdminOnly, state: &State<AppState>) -> ApiResult<Json<Vec<User>>> {
    let service = UserService::new(state.inner());
    let users = service.list_all()?;
    Ok(Json(users))
}

/// `GET /api/users/<id>` — fetch a single user. Restricted to administrators (ASVS V8.2.1).
#[get("/users/<id>")]
pub async fn get(_admin: AdminOnly, id: i32, state: &State<AppState>) -> ApiResult<Json<User>> {
    let service = UserService::new(state.inner());
    let user = service.get_by_id(id)?;
    Ok(Json(user))
}

/// `POST /api/users` — create a new user. Restricted to administrators (ASVS V8.2.1).
#[post("/users", data = "<payload>")]
pub async fn create(
    caller: AdminOnly,
    state: &State<AppState>,
    payload: Json<UserCreateRequest>,
) -> ApiResult<Value> {
    let service = UserService::new(state.inner());
    let id = service.create(&caller, payload.into_inner())?;

    Ok(json!({ "status": "ok", "id": id }))
}

/// `DELETE /api/users/<id>` — delete a user. Restricted to administrators (ASVS V8.2.1).
#[delete("/users/<id>")]
pub async fn delete(admin: AdminOnly, id: i32, state: &State<AppState>) -> ApiResult<Status> {
    let service = UserService::new(state.inner());
    service.delete(&admin, id)?;
    Ok(Status::NoContent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::{ContentType, Cookie, SameSite};
    use rocket::local::asynchronous::Client;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;
    const NON_ADMIN_ID: i32 = 2;

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
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

    fn non_admin_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, NON_ADMIN_ID.to_string());
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(SameSite::Strict);
        cookie
    }

    /// Build a repo that contains the seeded admin user plus a plain (non-admin) user with id 2.
    fn repo_with_non_admin() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default().with_admin_user();
        let non_admin = DieselRepoMock::make_user(NON_ADMIN_ID, "bob", "hash");
        repo.users.insert(NON_ADMIN_ID, non_admin);
        repo
    }

    // ----- non-admin rejection tests (ASVS V8.2.1) -----

    #[rocket::async_test]
    async fn list_forbidden_for_non_admin() {
        let client = client_with_repo(repo_with_non_admin()).await;
        let response = client
            .get("/api/users")
            .private_cookie(non_admin_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn get_forbidden_for_non_admin() {
        let client = client_with_repo(repo_with_non_admin()).await;
        let response = client
            .get("/api/users/1")
            .private_cookie(non_admin_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn create_forbidden_for_non_admin() {
        let client = client_with_repo(repo_with_non_admin()).await;
        let response = client
            .post("/api/users")
            .header(ContentType::JSON)
            .private_cookie(non_admin_cookie())
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
        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn delete_forbidden_for_non_admin() {
        let client = client_with_repo(repo_with_non_admin()).await;
        let response = client
            .delete("/api/users/1")
            .private_cookie(non_admin_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
    }

    // ----- admin success tests -----

    #[rocket::async_test]
    async fn list_returns_seeded_users() {
        let mut repo = DieselRepoMock::default();
        let mut users = HashMap::new();
        let mut user = DieselRepoMock::make_user(1, "alice", "hash");
        user.is_admin = true;
        users.insert(1, user);
        repo.users = users;

        let client = client_with_repo(repo).await;
        let response = client
            .get("/api/users")
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let items: Vec<User> = response.into_json().await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].username, "alice");
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

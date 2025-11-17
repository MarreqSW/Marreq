use crate::api::prelude::*;
use crate::models::{NewUser, User};
use crate::services::UserService;

#[get("/users")]
pub async fn list(_user: ApiUser, state: &State<AppState>) -> ApiResult<Json<Vec<User>>> {
    let service = UserService::new(state.inner());
    let users = service.list_all()?;
    Ok(Json(users))
}

#[get("/users/<id>")]
pub async fn get(_user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Json<User>> {
    let service = UserService::new(state.inner());
    let user = service.get_by_id(id)?;
    Ok(Json(user))
}

#[post("/users", data = "<payload>")]
pub async fn create(
    caller: ApiUser,
    state: &State<AppState>,
    payload: Json<NewUser>,
) -> ApiResult<Value> {
    let service = UserService::new(state.inner());
    let id = service.create(caller.user(), payload.into_inner())?;

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/users/<id>")]
pub async fn delete(_user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Status> {
    let service = UserService::new(state.inner());
    service.delete(_user.user(), id)?;
    Ok(Status::NoContent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::{ContentType, Cookie};
    use rocket::local::asynchronous::Client;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;

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
        cookie
    }

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
                    "id": null,
                    "username": "bob",
                    "name": "Bob",
                    "email": "bob@example.com",
                    "password_hash": "secret",
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
                    "id": null,
                    "username": "carol",
                    "name": "Carol",
                    "email": "carol@example.com",
                    "password_hash": "secret",
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

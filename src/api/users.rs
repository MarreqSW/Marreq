use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{NewUser, User};
use crate::repository::errors::RepoError;
use crate::repository::UserRepository;

#[get("/users")]
pub async fn list(_user: ApiUser, state: &State<AppState>) -> ApiResult<Json<Vec<User>>> {
    let users = state.repo.async_read(|repo| repo.get_users_all()).await?;
    Ok(Json(users))
}

#[get("/users/<id>")]
pub async fn get(_user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Json<User>> {
    let user = state
        .repo
        .async_read(move |repo| repo.get_user_by_id(id))
        .await?;
    Ok(Json(user))
}

#[post("/users", data = "<payload>")]
pub async fn create(
    caller: ApiUser,
    state: &State<AppState>,
    payload: Json<NewUser>,
) -> ApiResult<Value> {
    let user = payload.into_inner();
    let log_ctx = caller.log_ctx().clone();

    let id = state
        .repo
        .async_write(move |repo| {
            let id = repo.insert_user(&user)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let _ = Logger::created(conn.as_mut(), &log_ctx, id, &user);
            }
            Ok::<_, RepoError>(id)
        })
        .await?;

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/users/<id>")]
pub async fn delete(_user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Status> {
    state
        .repo
        .async_write(move |repo| repo.delete_user(id))
        .await?;
    Ok(Status::NoContent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::{ContentType, Header};
    use rocket::local::asynchronous::Client;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(state_from_repo(repo))
            .mount("/api", routes![list, get, create, delete]);
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_header() -> Header<'static> {
        Header::new("x-test-user", "admin")
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
            .header(auth_header())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let items: Vec<User> = response.into_json().await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].user_username, "alice");
    }

    #[rocket::async_test]
    async fn create_returns_new_identifier() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .post("/api/users")
            .header(ContentType::JSON)
            .header(auth_header())
            .body(
                json!({
                    "user_id": null,
                    "user_username": "bob",
                    "user_name": "Bob",
                    "user_email": "bob@example.com",
                    "user_password": "secret",
                    "is_admin": false
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("status"), Some(&Value::from("ok")));
        assert_eq!(payload.get("id"), Some(&Value::from(1)));
    }

    #[rocket::async_test]
    async fn delete_removes_existing_user() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_response = client
            .post("/api/users")
            .header(ContentType::JSON)
            .header(auth_header())
            .body(
                json!({
                    "user_id": null,
                    "user_username": "carol",
                    "user_name": "Carol",
                    "user_email": "carol@example.com",
                    "user_password": "secret",
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
            .header(auth_header())
            .dispatch()
            .await;
        assert_eq!(delete_response.status(), Status::NoContent);

        let not_found = client
            .get(format!("/api/users/{id}"))
            .header(auth_header())
            .dispatch()
            .await;
        assert_eq!(not_found.status(), Status::NotFound);
    }
}

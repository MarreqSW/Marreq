use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{EntityType, NewUser, User};
use crate::repository::errors::RepoError;
use crate::repository::UserRepository;

#[get("/users")]
pub fn list(state: &State<AppState>) -> ApiResult<Json<Vec<User>>> {
    state
        .repo_read()
        .get_users_all()
        .map(Json)
        .map_err(ApiError::from)
}

#[get("/users/<id>")]
pub fn get(state: &State<AppState>, id: i32) -> ApiResult<Json<User>> {
    state
        .repo_read()
        .get_user_by_id(id)
        .map(Json)
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("user {id} not found")),
            other => other.into(),
        })
}

#[post("/users", data = "<payload>")]
pub fn create(state: &State<AppState>, payload: Json<NewUser>) -> ApiResult<Value> {
    let user = payload.into_inner();
    let username = user.user_username.clone();

    let id = state
        .repo_write()
        .insert_user(&user)
        .map_err(|err| match err {
            RepoError::Db(e) => ApiError::BadRequest(format!("failed to create user: {e}")),
            other => other.into(),
        })?;

    if let Ok(mut conn) = state.repo_read().inner_repo().get_conn() {
        if let Ok(new_values) = Logger::to_json_string(&user) {
            if let Err(err) = Logger::log_create(
                conn.as_mut(),
                0,
                EntityType::User,
                id,
                None,
                Some(new_values),
                Some(format!("Created user via API: {username}")),
                None,
            ) {
                eprintln!("failed to record user creation log: {err}");
            }
        }
    }

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/users/<id>")]
pub fn delete(state: &State<AppState>, id: i32) -> ApiResult<Status> {
    state
        .repo_write()
        .delete_user(id)
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("user {id} not found")),
            other => other.into(),
        })?;

    Ok(Status::NoContent)
}

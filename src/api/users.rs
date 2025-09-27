use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{EntityType, NewUser, User};
use crate::repository::errors::RepoError;
use crate::repository::UserRepository;

#[get("/users")]
pub async fn list(state: &State<AppState>) -> ApiResult<Json<Vec<User>>> {
    let users = state.repo.async_read(|repo| repo.get_users_all()).await?;
    Ok(Json(users))
}

#[get("/users/<id>")]
pub async fn get(id: i32, state: &State<AppState>) -> ApiResult<Json<User>> {
    let user = state
        .repo
        .async_read(move |repo| repo.get_user_by_id(id))
        .await?;
    Ok(Json(user))
}

#[post("/users", data = "<payload>")]
pub async fn create(state: &State<AppState>, payload: Json<NewUser>) -> ApiResult<Value> {
    let user = payload.into_inner();
    let username = user.user_username.clone();
    let new_values = Logger::to_json_string(&user).ok();

    let id = state
        .repo
        .async_write(move |repo| {
            let id = repo.insert_user(&user)?;
            if let (Some(payload), Ok(mut conn)) = (new_values, repo.inner_repo().get_conn()) {
                let _ = Logger::log_create(
                    conn.as_mut(),
                    0,
                    EntityType::User,
                    id,
                    None,
                    Some(payload),
                    Some(format!("Created user via API: {username}")),
                    None,
                );
            }
            Ok::<_, RepoError>(id)
        })
        .await?;

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/users/<id>")]
pub async fn delete(id: i32, state: &State<AppState>) -> ApiResult<Status> {
    state
        .repo
        .async_write(move |repo| repo.delete_user(id))
        .await?;
    Ok(Status::NoContent)
}

use crate::api::prelude::*;
use crate::logger::{LogCtx, Logger};
use crate::models::{NewUser, User};
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

    let id = state
        .repo
        .async_write(move |repo| {
            let id = repo.insert_user(&user)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let ctx = LogCtx::new(0);
                let _ = Logger::created(conn.as_mut(), &ctx, id, &user);
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

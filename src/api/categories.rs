use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{Category, NewCategory};
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;

#[get("/categories")]
pub async fn list(_user: ApiUser, state: &State<AppState>) -> ApiResult<Json<Vec<Category>>> {
    let categories = state
        .repo
        .async_read(|repo| repo.get_categories_all())
        .await?;
    Ok(Json(categories))
}

#[get("/categories/<id>")]
pub async fn get(_user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Json<Category>> {
    let category = state
        .repo
        .async_read(move |repo| repo.get_category_by_id(id))
        .await?;
    Ok(Json(category))
}

#[post("/categories", data = "<payload>")]
pub async fn create(
    user: ApiUser,
    state: &State<AppState>,
    payload: Json<NewCategory>,
) -> ApiResult<Value> {
    let category = payload.into_inner();
    let log_ctx = user.log_ctx().clone();

    let id = state
        .repo
        .async_write(move |repo| {
            let id = repo.insert_new_category(&category)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let _ = Logger::created(conn.as_mut(), &log_ctx, id, &category);
            }

            Ok::<_, RepoError>(id)
        })
        .await?;

    Ok(json!({ "status": "ok", "id": id }))
}

#[put("/categories/<id>", data = "<payload>")]
pub async fn update(
    user: ApiUser,
    state: &State<AppState>,
    id: i32,
    payload: Json<NewCategory>,
) -> ApiResult<Value> {
    let mut category = payload.into_inner();
    category.cat_id = Some(id);
    let project_id = category.project_id;
    let log_ctx = user.log_ctx().clone();

    state
        .repo
        .async_write(move |repo| {
            let before = match repo.get_category_by_id(id) {
                Ok(c) => Some(c),
                Err(RepoError::NotFound) => None,
                Err(e) => return Err(e),
            };

            let updated = repo.edit_category(&category)?;
            if !updated {
                return Err(RepoError::NotFound);
            }

            if let Some(previous) = before {
                let after = Category {
                    cat_id: id,
                    cat_title: category.cat_title.clone(),
                    cat_description: category.cat_description.clone(),
                    cat_tag: category.cat_tag.clone(),
                    project_id,
                };
                if let Ok(mut conn) = repo.inner_repo().get_conn() {
                    let _ = Logger::updated(conn.as_mut(), &log_ctx, &previous, &after);
                }
            }

            Ok::<_, RepoError>(())
        })
        .await?;

    Ok(json!({
        "status": "ok",
        "message": "Category updated successfully"
    }))
}

#[delete("/categories/<id>")]
pub async fn delete(user: ApiUser, state: &State<AppState>, id: i32) -> ApiResult<Status> {
    let log_ctx = user.log_ctx().clone();
    state
        .repo
        .async_write(move |repo| {
            let removed = repo.delete_category(id)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let _ = Logger::deleted(conn.as_mut(), &log_ctx, &removed);
            }
            Ok::<_, RepoError>(())
        })
        .await?;

    Ok(Status::NoContent)
}

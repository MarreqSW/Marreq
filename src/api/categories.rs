use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{Category, EntityType, NewCategory};
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;

#[get("/categories")]
pub fn list(state: &State<AppState>) -> ApiResult<Json<Vec<Category>>> {
    state
        .repo_read()
        .get_categories_all()
        .map(Json)
        .map_err(ApiError::from)
}

#[get("/categories/<id>")]
pub fn get(state: &State<AppState>, id: i32) -> ApiResult<Json<Category>> {
    state
        .repo_read()
        .get_category_by_id(id)
        .map(Json)
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("category {id} not found")),
            other => other.into(),
        })
}

#[post("/categories", data = "<payload>")]
pub fn create(state: &State<AppState>, payload: Json<NewCategory>) -> ApiResult<Value> {
    let category = payload.into_inner();
    let title = category.cat_title.clone();
    let project_id = category.project_id;

    let id = state
        .repo_write()
        .insert_new_category(&category)
        .map_err(|err| match err {
            RepoError::Db(e) => ApiError::BadRequest(format!("failed to create category: {e}")),
            other => other.into(),
        })?;

    if let Ok(mut conn) = state.repo_read().inner_repo().get_conn() {
        if let Ok(new_values) = Logger::to_json_string(&category) {
            if let Err(err) = Logger::log_create(
                conn.as_mut(),
                0,
                EntityType::Category,
                id,
                Some(project_id),
                Some(new_values),
                Some(format!("Created category via API: {title}")),
                None,
            ) {
                eprintln!("failed to record category creation log: {err}");
            }
        }
    }

    Ok(json!({ "status": "ok", "id": id }))
}

#[put("/categories/<id>", data = "<payload>")]
pub fn update(state: &State<AppState>, id: i32, payload: Json<NewCategory>) -> ApiResult<Value> {
    let mut category = payload.into_inner();
    category.cat_id = Some(id);
    let before = state.repo_read().get_category_by_id(id).ok();

    let updated = state
        .repo_write()
        .edit_category(&category)
        .map_err(|err| match err {
            RepoError::Db(e) => ApiError::BadRequest(format!("failed to update category: {e}")),
            other => other.into(),
        })?;

    if !updated {
        return Err(ApiError::NotFound(format!("category {id} not found")));
    }

    if let (Some(previous), Ok(mut conn)) = (before, state.repo_read().inner_repo().get_conn()) {
        if let (Ok(old_values), Ok(new_values)) = (
            Logger::to_json_string(&previous),
            Logger::to_json_string(&category),
        ) {
            if let Err(err) = Logger::log_update(
                conn.as_mut(),
                0,
                EntityType::Category,
                id,
                Some(category.project_id),
                Some(old_values),
                Some(new_values),
                Some("Updated category via API".into()),
                None,
            ) {
                eprintln!("failed to record category update log: {err}");
            }
        }
    }

    Ok(json!({
        "status": "ok",
        "message": "Category updated successfully"
    }))
}

#[delete("/categories/<id>")]
pub fn delete(state: &State<AppState>, id: i32) -> ApiResult<Status> {
    let category = state
        .repo_write()
        .delete_category(id)
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("category {id} not found")),
            other => other.into(),
        })?;

    if let (Ok(old_values), Ok(mut conn)) = (
        Logger::to_json_string(&category),
        state.repo_read().inner_repo().get_conn(),
    ) {
        if let Err(err) = Logger::log_delete(
            conn.as_mut(),
            0,
            EntityType::Category,
            id,
            Some(category.project_id),
            Some(old_values),
            Some(format!("Deleted category via API: {}", category.cat_title)),
            None,
        ) {
            eprintln!("failed to record category deletion log: {err}");
        }
    }

    Ok(Status::NoContent)
}

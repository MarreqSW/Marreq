use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{Category, EntityType, NewCategory};
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;

#[get("/categories")]
pub async fn list(state: &State<AppState>) -> ApiResult<Json<Vec<Category>>> {
    let categories = state
        .repo
        .db_read(|repo| repo.get_categories_all())
        .await
        .map_err(ApiError::from)?;
    Ok(Json(categories))
}

#[get("/categories/<id>")]
pub async fn get(id: i32, state: &State<AppState>) -> ApiResult<Json<Category>> {
    let category = state
        .repo
        .db_read(move |repo| repo.get_category_by_id(id))
        .await
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("category {id} not found")),
            other => other.into(),
        })?;
    Ok(Json(category))
}

#[post("/categories", data = "<payload>")]
pub async fn create(state: &State<AppState>, payload: Json<NewCategory>) -> ApiResult<Value> {
    let category = payload.into_inner();
    let title = category.cat_title.clone();
    let project_id = category.project_id;
    let new_values = Logger::to_json_string(&category).ok();

    let id = state
        .repo
        .db_write(move |repo| repo.insert_new_category(&category))
        .await
        .map_err(|err| match err {
            RepoError::Db(e) => ApiError::BadRequest(format!("failed to create category: {e}")),
            other => other.into(),
        })?;

    let _ = state
        .repo
        .db_read(move |repo| {
            let mut conn = repo.inner_repo().get_conn()?;
            if let Some(payload) = new_values {
                let _ = Logger::log_create(
                    conn.as_mut(),
                    0,
                    EntityType::Category,
                    id,
                    Some(project_id),
                    Some(payload),
                    Some(format!("Created category via API: {title}")),
                    None,
                );
            }
            Ok::<(), RepoError>(())
        })
        .await;

    Ok(json!({ "status": "ok", "id": id }))
}

#[put("/categories/<id>", data = "<payload>")]
pub async fn update(
    state: &State<AppState>,
    id: i32,
    payload: Json<NewCategory>,
) -> ApiResult<Value> {
    let mut category = payload.into_inner();
    category.cat_id = Some(id);

    let before = state
        .repo
        .db_read(move |repo| match repo.get_category_by_id(id) {
            Ok(c) => Ok::<Option<_>, RepoError>(Some(c)),
            Err(RepoError::NotFound) => Ok(None),
            Err(e) => Err(e),
        })
        .await?;

    let project_id = category.project_id;
    let new_values = Logger::to_json_string(&category).ok();

    let updated = state
        .repo
        .db_write({
            let category = category.clone();
            move |repo| repo.edit_category(&category)
        })
        .await
        .map_err(|err| match err {
            RepoError::Db(e) => ApiError::BadRequest(format!("failed to update category: {e}")),
            other => other.into(),
        })?;

    if !updated {
        return Err(ApiError::NotFound(format!("category {id} not found")));
    }

    let _ = state
        .repo
        .db_read(move |repo| {
            if let Some(previous) = before {
                let mut conn = repo.inner_repo().get_conn()?;
                if let (Ok(old_values), Some(new_values)) =
                    (Logger::to_json_string(&previous), new_values)
                {
                    let _ = Logger::log_update(
                        conn.as_mut(),
                        0,
                        EntityType::Category,
                        id,
                        Some(project_id),
                        Some(old_values),
                        Some(new_values),
                        Some("Updated category via API".into()),
                        None,
                    );
                }
            }
            Ok::<(), RepoError>(())
        })
        .await;

    Ok(json!({
        "status": "ok",
        "message": "Category updated successfully"
    }))
}

#[delete("/categories/<id>")]
pub async fn delete(state: &State<AppState>, id: i32) -> ApiResult<Status> {
    let removed = state
        .repo
        .db_write(move |repo| repo.delete_category(id))
        .await
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("category {id} not found")),
            other => other.into(),
        })?;

    let _ = state
        .repo
        .db_read(move |repo| {
            let mut conn = repo.inner_repo().get_conn()?;
            if let Ok(old_values) = Logger::to_json_string(&removed) {
                let _ = Logger::log_delete(
                    conn.as_mut(),
                    0,
                    EntityType::Category,
                    id,
                    Some(removed.project_id),
                    Some(old_values),
                    Some(format!("Deleted category via API: {}", removed.cat_title)),
                    None,
                );
            }
            Ok::<(), RepoError>(())
        })
        .await;

    Ok(Status::NoContent)
}

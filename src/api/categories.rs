use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{Category, EntityType, NewCategory};
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;

#[get("/categories")]
pub async fn list(state: &State<AppState>) -> ApiResult<Json<Vec<Category>>> {
    let categories = state
        .repo
        .async_read(|repo| repo.get_categories_all())
        .await?;
    Ok(Json(categories))
}

#[get("/categories/<id>")]
pub async fn get(id: i32, state: &State<AppState>) -> ApiResult<Json<Category>> {
    let category = state
        .repo
        .async_read(move |repo| repo.get_category_by_id(id))
        .await?;
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
        .async_write(move |repo| {
            let id = repo.insert_new_category(&category)?;

            if let (Some(payload), Ok(mut conn)) = (new_values, repo.inner_repo().get_conn()) {
                let _ = Logger::log_create(
                    conn.as_mut(),
                    0,
                    EntityType::Category,
                    id,
                    Some(project_id),
                    Some(payload),
                    Some(format!("Created category via API: {}", title)),
                    None,
                );
            }

            Ok::<_, RepoError>(id)
        })
        .await?;

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
    let project_id = category.project_id;

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
                if let Ok(mut conn) = repo.inner_repo().get_conn() {
                    if let (Ok(old_values), Ok(new_values)) = (
                        Logger::to_json_string(&previous),
                        Logger::to_json_string(&category),
                    ) {
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
pub async fn delete(state: &State<AppState>, id: i32) -> ApiResult<Status> {
    state
        .repo
        .async_write(move |repo| {
            let removed = repo.delete_category(id)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
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
            }
            Ok::<_, RepoError>(())
        })
        .await?;

    Ok(Status::NoContent)
}

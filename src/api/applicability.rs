use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{Applicability, EntityType, NewApplicability};
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;

#[get("/applicability")]
pub async fn list(state: &State<AppState>) -> ApiResult<Json<Vec<Applicability>>> {
    let items = state
        .repo
        .db_read(|repo| repo.get_applicability_all())
        .await?;
    Ok(Json(items))
}

#[get("/applicability/<id>")]
pub async fn get(state: &State<AppState>, id: i32) -> ApiResult<Json<Applicability>> {
    let applicability = state
        .repo
        .db_read(move |repo| repo.get_applicability_by_id(id))
        .await?;

    Ok(Json(applicability))
}

#[post("/applicability", data = "<payload>")]
pub async fn create(state: &State<AppState>, payload: Json<NewApplicability>) -> ApiResult<Value> {
    let applicability = payload.into_inner();

    let title = applicability.app_title.clone();
    let project_id = applicability.project_id;
    let new_values = Logger::to_json_string(&applicability).ok();

    let id = state
        .repo
        .db_write(move |repo| repo.insert_new_applicability(&applicability))
        .await?;

    let title_for_log = title.clone();
    let _ = state
        .repo
        .db_read(move |repo| {
            let mut conn = repo.inner_repo().get_conn()?;
            if let Some(payload) = new_values {
                let _ = Logger::log_create(
                    conn.as_mut(),
                    0,
                    EntityType::Applicability,
                    id,
                    Some(project_id),
                    Some(payload),
                    Some(format!("Created applicability via API: {}", title_for_log)),
                    None,
                );
            }
            Ok(())
        })
        .await;

    Ok(json!({ "status": "ok", "id": id }))
}

#[put("/applicability/<id>", data = "<payload>")]
pub async fn update(
    state: &State<AppState>,
    id: i32,
    payload: Json<NewApplicability>,
) -> ApiResult<Value> {
    let mut applicability = payload.into_inner();
    applicability.app_id = Some(id);

    let before = state
        .repo
        .db_read(move |repo| match repo.get_applicability_by_id(id) {
            Ok(a) => Ok::<Option<_>, RepoError>(Some(a)),
            Err(RepoError::NotFound) => Ok(None),
            Err(e) => Err(e),
        })
        .await?;

    let updated = state
        .repo
        .db_write({
            let applicability = applicability.clone();
            move |repo| repo.edit_applicability(&applicability)
        })
        .await?;

    if !updated {
        return Err(ApiError::NotFound(format!("applicability {id} not found")));
    }

    let app_for_log = applicability.clone();
    let _ = state
        .repo
        .db_read(move |repo| {
            let mut conn = repo.inner_repo().get_conn()?;
            if let Some(previous) = before {
                if let (Ok(old_values), Ok(new_values)) = (
                    Logger::to_json_string(&previous),
                    Logger::to_json_string(&app_for_log),
                ) {
                    let _ = Logger::log_update(
                        conn.as_mut(),
                        0,
                        EntityType::Applicability,
                        id,
                        Some(app_for_log.project_id),
                        Some(old_values),
                        Some(new_values),
                        Some("Updated applicability via API".into()),
                        None,
                    );
                }
            }
            Ok::<(), RepoError>(())
        })
        .await;

    Ok(json!({
        "status": "ok",
        "message": "Applicability updated successfully"
    }))
}

#[delete("/applicability/<id>")]
pub async fn delete(state: &State<AppState>, id: i32) -> ApiResult<Status> {
    let removed = state
        .repo
        .db_write(move |repo| repo.delete_applicability(id))
        .await?;

    let removed_for_log = removed.clone();
    let _ = state
        .repo
        .db_read(move |repo| {
            let mut conn = repo.inner_repo().get_conn()?;
            if let Ok(old_values) = Logger::to_json_string(&removed_for_log) {
                let _ = Logger::log_delete(
                    conn.as_mut(),
                    0,
                    EntityType::Applicability,
                    id,
                    Some(removed_for_log.project_id),
                    Some(old_values),
                    Some(format!(
                        "Deleted applicability via API: {}",
                        removed_for_log.app_title
                    )),
                    None,
                );
            }
            Ok::<(), RepoError>(())
        })
        .await;

    Ok(Status::NoContent)
}

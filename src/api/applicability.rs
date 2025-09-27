use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{Applicability, EntityType, NewApplicability};
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;

#[get("/applicability")]
pub fn list(state: &State<AppState>) -> ApiResult<Json<Vec<Applicability>>> {
    state
        .repo_read()
        .get_applicability_all()
        .map(Json)
        .map_err(ApiError::from)
}

#[get("/applicability/<id>")]
pub fn get(state: &State<AppState>, id: i32) -> ApiResult<Json<Applicability>> {
    state
        .repo_read()
        .get_applicability_by_id(id)
        .map(Json)
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("applicability {id} not found")),
            other => other.into(),
        })
}

#[post("/applicability", data = "<payload>")]
pub fn create(state: &State<AppState>, payload: Json<NewApplicability>) -> ApiResult<Value> {
    let applicability = payload.into_inner();
    let title = applicability.app_title.clone();
    let project_id = applicability.project_id;

    let id = state
        .repo_write()
        .insert_new_applicability(&applicability)
        .map_err(|err| match err {
            RepoError::Db(e) => {
                ApiError::BadRequest(format!("failed to create applicability: {e}"))
            }
            other => other.into(),
        })?;

    if let Ok(mut conn) = state.repo_read().inner_repo().get_conn() {
        if let Ok(new_values) = Logger::to_json_string(&applicability) {
            if let Err(err) = Logger::log_create(
                conn.as_mut(),
                0,
                EntityType::Applicability,
                id,
                Some(project_id),
                Some(new_values),
                Some(format!("Created applicability via API: {title}")),
                None,
            ) {
                eprintln!("failed to record applicability creation log: {err}");
            }
        }
    }

    Ok(json!({ "status": "ok", "id": id }))
}

#[put("/applicability/<id>", data = "<payload>")]
pub fn update(
    state: &State<AppState>,
    id: i32,
    payload: Json<NewApplicability>,
) -> ApiResult<Value> {
    let mut applicability = payload.into_inner();
    applicability.app_id = Some(id);
    let before = state.repo_read().get_applicability_by_id(id).ok();

    let updated = state
        .repo_write()
        .edit_applicability(&applicability)
        .map_err(|err| match err {
            RepoError::Db(e) => {
                ApiError::BadRequest(format!("failed to update applicability: {e}"))
            }
            other => other.into(),
        })?;

    if !updated {
        return Err(ApiError::NotFound(format!("applicability {id} not found")));
    }

    if let (Some(previous), Ok(mut conn)) = (before, state.repo_read().inner_repo().get_conn()) {
        if let (Ok(old_values), Ok(new_values)) = (
            Logger::to_json_string(&previous),
            Logger::to_json_string(&applicability),
        ) {
            if let Err(err) = Logger::log_update(
                conn.as_mut(),
                0,
                EntityType::Applicability,
                id,
                Some(applicability.project_id),
                Some(old_values),
                Some(new_values),
                Some("Updated applicability via API".into()),
                None,
            ) {
                eprintln!("failed to record applicability update log: {err}");
            }
        }
    }

    Ok(json!({
        "status": "ok",
        "message": "Applicability updated successfully"
    }))
}

#[delete("/applicability/<id>")]
pub fn delete(state: &State<AppState>, id: i32) -> ApiResult<Status> {
    let applicability = state
        .repo_write()
        .delete_applicability(id)
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("applicability {id} not found")),
            other => other.into(),
        })?;

    if let (Ok(old_values), Ok(mut conn)) = (
        Logger::to_json_string(&applicability),
        state.repo_read().inner_repo().get_conn(),
    ) {
        if let Err(err) = Logger::log_delete(
            conn.as_mut(),
            0,
            EntityType::Applicability,
            id,
            Some(applicability.project_id),
            Some(old_values),
            Some(format!(
                "Deleted applicability via API: {}",
                applicability.app_title
            )),
            None,
        ) {
            eprintln!("failed to record applicability deletion log: {err}");
        }
    }

    Ok(Status::NoContent)
}

use rocket::serde::{Deserialize, Serialize};

use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{EntityType, NewRequirement, Requirement};
use crate::repository::errors::RepoError;
use crate::repository::RequirementsRepository;

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FieldUpdateRequest {
    pub field: String,
    pub value: String,
}

#[get("/requirements")]
pub async fn list(state: &State<AppState>) -> ApiResult<Json<Vec<Requirement>>> {
    let requirements = state
        .repo
        .db_read(|repo| repo.get_requirements_all())
        .await?;
    Ok(Json(requirements))
}

#[get("/requirements/<id>")]
pub async fn get(id: i32, state: &State<AppState>) -> ApiResult<Json<Requirement>> {
    let requirement = state
        .repo
        .db_read(move |repo| repo.get_requirement_by_id(id))
        .await?;
    Ok(Json(requirement))
}

#[post("/requirements", data = "<payload>")]
pub async fn create(state: &State<AppState>, payload: Json<NewRequirement>) -> ApiResult<Value> {
    let requirement = payload.into_inner();
    let title = requirement.req_title.clone();
    let project_id = requirement.project_id;
    let new_values = Logger::to_json_string(&requirement).ok();

    let id = state
        .repo
        .db_write(move |repo| repo.insert_new_requirement(&requirement))
        .await?;

    let _ = state
        .repo
        .db_read(move |repo| {
            let mut conn = repo.inner_repo().get_conn()?;
            if let Some(payload) = new_values {
                let _ = Logger::log_create(
                    conn.as_mut(),
                    0,
                    EntityType::Requirement,
                    id,
                    Some(project_id),
                    Some(payload),
                    Some(format!("Created requirement via API: {title}")),
                    None,
                );
            }
            Ok::<(), RepoError>(())
        })
        .await;

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/requirements/<id>")]
pub async fn delete(id: i32, state: &State<AppState>) -> ApiResult<Status> {
    let removed = state
        .repo
        .db_write(move |repo| repo.delete_requirement(id))
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
                    EntityType::Requirement,
                    id,
                    Some(removed_for_log.project_id),
                    Some(old_values),
                    Some(format!(
                        "Deleted requirement via API: {}",
                        removed_for_log.req_title
                    )),
                    None,
                );
            }
            Ok::<(), RepoError>(())
        })
        .await;

    Ok(Status::NoContent)
}

#[post("/requirements/<id>/field", data = "<update>")]
pub async fn update_field(
    state: &State<AppState>,
    id: i32,
    update: Json<FieldUpdateRequest>,
) -> ApiResult<Value> {
    let update = update.into_inner();

    let mut requirement = state
        .repo
        .db_read(move |repo| repo.get_requirement_by_id(id))
        .await?;
    let original = requirement.clone();

    match update.field.as_str() {
        "req_title" => requirement.req_title = update.value,
        "req_description" => requirement.req_description = update.value,
        "req_current_status" => {
            requirement.req_current_status = update
                .value
                .parse()
                .map_err(|_| ApiError::BadRequest("invalid status id".into()))?;
        }
        "req_verification" => {
            requirement.req_verification = update
                .value
                .parse()
                .map_err(|_| ApiError::BadRequest("invalid verification id".into()))?;
        }
        "req_author" => {
            requirement.req_author = update
                .value
                .parse()
                .map_err(|_| ApiError::BadRequest("invalid author id".into()))?;
        }
        "req_reviewer" => {
            requirement.req_reviewer = update
                .value
                .parse()
                .map_err(|_| ApiError::BadRequest("invalid reviewer id".into()))?;
        }
        "req_category" => {
            requirement.req_category = update
                .value
                .parse()
                .map_err(|_| ApiError::BadRequest("invalid category id".into()))?;
        }
        "req_applicability" => {
            requirement.req_applicability = update
                .value
                .parse()
                .map_err(|_| ApiError::BadRequest("invalid applicability id".into()))?;
        }
        other => return Err(ApiError::BadRequest(format!("unsupported field '{other}'"))),
    }

    let payload = NewRequirement {
        req_id: Some(requirement.req_id),
        req_title: requirement.req_title.clone(),
        req_description: requirement.req_description.clone(),
        req_verification: requirement.req_verification,
        req_author: requirement.req_author,
        req_link: requirement.req_link.clone(),
        req_category: requirement.req_category,
        req_current_status: requirement.req_current_status,
        req_parent: requirement.req_parent,
        req_reference: requirement.req_reference.clone(),
        req_reviewer: requirement.req_reviewer,
        req_applicability: requirement.req_applicability,
        req_justification: requirement.req_justification.clone(),
        project_id: requirement.project_id,
    };

    state
        .repo
        .db_write(move |repo| repo.edit_requirement(&payload))
        .await?;

    let requirement_for_log = requirement.clone();
    let original_for_log = original.clone();
    let _ = state
        .repo
        .db_read(move |repo| {
            let mut conn = repo.inner_repo().get_conn()?;
            if let (Ok(old_values), Ok(new_values)) = (
                Logger::to_json_string(&original_for_log),
                Logger::to_json_string(&requirement_for_log),
            ) {
                let _ = Logger::log_update(
                    conn.as_mut(),
                    0,
                    EntityType::Requirement,
                    requirement_for_log.req_id,
                    Some(requirement_for_log.project_id),
                    Some(old_values),
                    Some(new_values),
                    Some("Updated requirement via API".into()),
                    None,
                );
            }
            Ok::<(), RepoError>(())
        })
        .await;

    Ok(json!({
        "success": true,
        "message": "Field updated successfully"
    }))
}

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
pub fn list(state: &State<AppState>) -> ApiResult<Json<Vec<Requirement>>> {
    state
        .repo_read()
        .get_requirements_all()
        .map(Json)
        .map_err(ApiError::from)
}

#[get("/requirements/<id>")]
pub fn get(state: &State<AppState>, id: i32) -> ApiResult<Json<Requirement>> {
    state
        .repo_read()
        .get_requirement_by_id(id)
        .map(Json)
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("requirement {id} not found")),
            other => other.into(),
        })
}

#[post("/requirements", data = "<payload>")]
pub fn create(state: &State<AppState>, payload: Json<NewRequirement>) -> ApiResult<Value> {
    let requirement = payload.into_inner();
    let title = requirement.req_title.clone();
    let project_id = requirement.project_id;

    let id = state
        .repo_write()
        .insert_new_requirement(&requirement)
        .map_err(|err| match err {
            RepoError::Db(e) => ApiError::BadRequest(format!("failed to create requirement: {e}")),
            other => other.into(),
        })?;

    if let Ok(mut conn) = state.repo_read().inner_repo().get_conn() {
        if let Ok(new_values) = Logger::to_json_string(&requirement) {
            if let Err(err) = Logger::log_create(
                conn.as_mut(),
                0,
                EntityType::Requirement,
                id,
                Some(project_id),
                Some(new_values),
                Some(format!("Created requirement via API: {title}")),
                None,
            ) {
                eprintln!("failed to record requirement creation log: {err}");
            }
        }
    }

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/requirements/<id>")]
pub fn delete(state: &State<AppState>, id: i32) -> ApiResult<Status> {
    let requirement = state
        .repo_write()
        .delete_requirement(id)
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("requirement {id} not found")),
            other => other.into(),
        })?;

    if let (Ok(old_values), Ok(mut conn)) = (
        Logger::to_json_string(&requirement),
        state.repo_read().inner_repo().get_conn(),
    ) {
        if let Err(err) = Logger::log_delete(
            conn.as_mut(),
            0,
            EntityType::Requirement,
            id,
            Some(requirement.project_id),
            Some(old_values),
            Some(format!(
                "Deleted requirement via API: {}",
                requirement.req_title
            )),
            None,
        ) {
            eprintln!("failed to record requirement deletion log: {err}");
        }
    }

    Ok(Status::NoContent)
}

#[post("/requirements/<id>/field", data = "<update>")]
pub fn update_field(
    state: &State<AppState>,
    id: i32,
    update: Json<FieldUpdateRequest>,
) -> ApiResult<Value> {
    let update = update.into_inner();
    let mut requirement = state
        .repo_read()
        .get_requirement_by_id(id)
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("requirement {id} not found")),
            other => other.into(),
        })?;
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
        other => {
            return Err(ApiError::BadRequest(format!("unsupported field '{other}'")));
        }
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
        .repo_write()
        .edit_requirement(&payload)
        .map_err(ApiError::from)?;

    if let (Ok(old_values), Ok(new_values), Ok(mut conn)) = (
        Logger::to_json_string(&original),
        Logger::to_json_string(&requirement),
        state.repo_read().inner_repo().get_conn(),
    ) {
        if let Err(err) = Logger::log_update(
            conn.as_mut(),
            0,
            EntityType::Requirement,
            requirement.req_id,
            Some(requirement.project_id),
            Some(old_values),
            Some(new_values),
            Some("Updated requirement via API".into()),
            None,
        ) {
            eprintln!("failed to record requirement update log: {err}");
        }
    }

    Ok(json!({
        "success": true,
        "message": "Field updated successfully"
    }))
}

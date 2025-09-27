use rocket::serde::Deserialize;

use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{EntityType, NewRequirement, Requirement};
use crate::repository::errors::RepoError;
use crate::repository::RequirementsRepository;

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct RequirementPatch {
    pub req_title: Option<String>,
    pub req_description: Option<String>,
    pub req_current_status: Option<i32>,
    pub req_verification: Option<i32>,
    pub req_author: Option<i32>,
    pub req_reviewer: Option<i32>,
    pub req_category: Option<i32>,
    pub req_applicability: Option<i32>,
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

#[patch("/requirements/<id>", data = "<patch>")]
pub async fn patch_requirement(
    state: &State<AppState>,
    id: i32,
    patch: Json<RequirementPatch>,
) -> ApiResult<Value> {
    let patch = patch.into_inner();

    let mut requirement = state
        .repo
        .db_read(move |repo| repo.get_requirement_by_id(id))
        .await?;
    let original = requirement.clone();

    let mut updated = false;

    if let Some(req_title) = patch.req_title {
        requirement.req_title = req_title;
        updated = true;
    }
    if let Some(req_description) = patch.req_description {
        requirement.req_description = req_description;
        updated = true;
    }
    if let Some(req_current_status) = patch.req_current_status {
        requirement.req_current_status = req_current_status;
        updated = true;
    }
    if let Some(req_verification) = patch.req_verification {
        requirement.req_verification = req_verification;
        updated = true;
    }
    if let Some(req_author) = patch.req_author {
        requirement.req_author = req_author;
        updated = true;
    }
    if let Some(req_reviewer) = patch.req_reviewer {
        requirement.req_reviewer = req_reviewer;
        updated = true;
    }
    if let Some(req_category) = patch.req_category {
        requirement.req_category = req_category;
        updated = true;
    }
    if let Some(req_applicability) = patch.req_applicability {
        requirement.req_applicability = req_applicability;
        updated = true;
    }

    if !updated {
        return Err(ApiError::BadRequest("no fields provided".into()));
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

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
        .async_read(|repo| repo.get_requirements_all())
        .await?;
    Ok(Json(requirements))
}

#[get("/requirements/<id>")]
pub async fn get(id: i32, state: &State<AppState>) -> ApiResult<Json<Requirement>> {
    let requirement = state
        .repo
        .async_read(move |repo| repo.get_requirement_by_id(id))
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
        .async_write(move |repo| {
            let id = repo.insert_new_requirement(&requirement)?;
            if let (Some(payload), Ok(mut conn)) = (new_values, repo.inner_repo().get_conn()) {
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
            Ok::<_, RepoError>(id)
        })
        .await?;

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/requirements/<id>")]
pub async fn delete(id: i32, state: &State<AppState>) -> ApiResult<Status> {
    state
        .repo
        .async_write(move |repo| {
            let removed = repo.delete_requirement(id)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                if let Ok(old_values) = Logger::to_json_string(&removed) {
                    let _ = Logger::log_delete(
                        conn.as_mut(),
                        0,
                        EntityType::Requirement,
                        id,
                        Some(removed.project_id),
                        Some(old_values),
                        Some(format!(
                            "Deleted requirement via API: {}",
                            removed.req_title
                        )),
                        None,
                    );
                }
            }
            Ok::<_, RepoError>(())
        })
        .await?;
    Ok(Status::NoContent)
}

#[patch("/requirements/<id>", data = "<patch>")]
pub async fn patch_requirement(
    state: &State<AppState>,
    id: i32,
    patch: Json<RequirementPatch>,
) -> ApiResult<Value> {
    let patch = patch.into_inner();

    let any_updates = patch.req_title.is_some()
        || patch.req_description.is_some()
        || patch.req_current_status.is_some()
        || patch.req_verification.is_some()
        || patch.req_author.is_some()
        || patch.req_reviewer.is_some()
        || patch.req_category.is_some()
        || patch.req_applicability.is_some();

    if !any_updates {
        return Err(ApiError::BadRequest("no fields provided".into()));
    }

    state
        .repo
        .async_write(move |repo| {
            let mut requirement = repo.get_requirement_by_id(id)?;
            let original = requirement.clone();

            if let Some(v) = patch.req_title {
                requirement.req_title = v;
            }
            if let Some(v) = patch.req_description {
                requirement.req_description = v;
            }
            if let Some(v) = patch.req_current_status {
                requirement.req_current_status = v;
            }
            if let Some(v) = patch.req_verification {
                requirement.req_verification = v;
            }
            if let Some(v) = patch.req_author {
                requirement.req_author = v;
            }
            if let Some(v) = patch.req_reviewer {
                requirement.req_reviewer = v;
            }
            if let Some(v) = patch.req_category {
                requirement.req_category = v;
            }
            if let Some(v) = patch.req_applicability {
                requirement.req_applicability = v;
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

            repo.edit_requirement(&payload)?;

            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                if let (Ok(old_values), Ok(new_values)) = (
                    Logger::to_json_string(&original),
                    Logger::to_json_string(&requirement),
                ) {
                    let _ = Logger::log_update(
                        conn.as_mut(),
                        0,
                        EntityType::Requirement,
                        requirement.req_id,
                        Some(requirement.project_id),
                        Some(old_values),
                        Some(new_values),
                        Some("Updated requirement via API".into()),
                        None,
                    );
                }
            }

            Ok::<_, RepoError>(())
        })
        .await?;

    Ok(json!({
        "success": true,
        "message": "Field updated successfully"
    }))
}

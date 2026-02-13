//! REST API for project-scoped custom field definitions.

use rocket::State;

use crate::api::prelude::*;
use crate::models::{CustomFieldDefinition, CustomFieldDefinitionPayload};
use crate::repository::ProjectsRepository;
use crate::services::CustomFieldService;

#[get("/projects/<project_id>/custom_fields")]
pub async fn list_by_project(
    _user: ApiUser,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<CustomFieldDefinition>>> {
    let _ = state
        .repo_read()
        .get_project_by_id(project_id)
        .map_err(ApiError::from)?;
    let service = CustomFieldService::new(state.inner());
    let list = service.list_by_project(project_id)?;
    Ok(Json(list))
}

#[get("/projects/<project_id>/custom_fields/<field_id>")]
pub async fn get(
    _user: ApiUser,
    project_id: i32,
    field_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<CustomFieldDefinition>> {
    let _ = state
        .repo_read()
        .get_project_by_id(project_id)
        .map_err(ApiError::from)?;
    let service = CustomFieldService::new(state.inner());
    let def = service.get_by_id(field_id)?;
    if def.project_id != project_id {
        return Err(ApiError::NotFound("custom field not in project".into()));
    }
    Ok(Json(def))
}

#[post("/projects/<project_id>/custom_fields", data = "<payload>")]
pub async fn create(
    _user: ApiUser,
    project_id: i32,
    state: &State<AppState>,
    payload: Json<CustomFieldDefinitionPayload>,
) -> ApiResult<Value> {
    let _ = state
        .repo_read()
        .get_project_by_id(project_id)
        .map_err(ApiError::from)?;
    let service = CustomFieldService::new(state.inner());
    let id = service.create(project_id, payload.into_inner())?;
    Ok(json!({ "status": "ok", "id": id }))
}

#[put("/projects/<project_id>/custom_fields/<field_id>", data = "<payload>")]
pub async fn update(
    _user: ApiUser,
    project_id: i32,
    field_id: i32,
    state: &State<AppState>,
    payload: Json<CustomFieldDefinitionPayload>,
) -> ApiResult<Value> {
    let _ = state
        .repo_read()
        .get_project_by_id(project_id)
        .map_err(ApiError::from)?;
    let service = CustomFieldService::new(state.inner());
    service.update(field_id, payload.into_inner())?;
    let def = service.get_by_id(field_id)?;
    if def.project_id != project_id {
        return Err(ApiError::NotFound("custom field not in project".into()));
    }
    Ok(json!({
        "status": "ok",
        "message": "Custom field updated successfully"
    }))
}

#[delete("/projects/<project_id>/custom_fields/<field_id>")]
pub async fn delete(
    _user: ApiUser,
    project_id: i32,
    field_id: i32,
    state: &State<AppState>,
) -> ApiResult<Value> {
    let _ = state
        .repo_read()
        .get_project_by_id(project_id)
        .map_err(ApiError::from)?;
    let service = CustomFieldService::new(state.inner());
    let def = service.get_by_id(field_id)?;
    if def.project_id != project_id {
        return Err(ApiError::NotFound("custom field not in project".into()));
    }
    let in_use = service.count_versions_using_field(field_id)?;
    if in_use > 0 {
        return Err(ApiError::BadRequest(format!(
            "Cannot delete: field is in use by {} requirement version(s). Remove or update those values first.",
            in_use
        )));
    }
    service.delete(field_id)?;
    Ok(json!({
        "status": "ok",
        "message": "Custom field deleted"
    }))
}

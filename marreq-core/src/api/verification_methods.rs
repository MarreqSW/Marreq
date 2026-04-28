// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Verification methods per project: list (read) and CRUD for SPA catalog.

use rocket::serde::json::Json;

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::models::{NewVerificationMethod, VerificationMethod};
use crate::repository::LookupRepository;

#[get("/projects/<project_id>/verification-methods")]
pub async fn list_by_project(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<VerificationMethod>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let methods = state
        .repo_read()
        .get_verification_methods_by_project(project_id)?;
    Ok(Json(methods))
}

#[post("/projects/<project_id>/verification-methods", data = "<payload>")]
pub async fn create_by_project(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
    payload: Json<NewVerificationMethod>,
) -> ApiResult<Value> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    let mut body = payload.into_inner();
    body.project_id = project_id;
    let id = {
        let mut repo = state.repo_write();
        repo.insert_new_verification_method(&body)?
    };
    Ok(json!({ "status": "ok", "id": id }))
}

#[put(
    "/projects/<project_id>/verification-methods/<method_id>",
    data = "<payload>"
)]
pub async fn update_by_project(
    access: ProjectAccessOrBearer,
    project_id: i32,
    method_id: i32,
    state: &State<AppState>,
    payload: Json<NewVerificationMethod>,
) -> ApiResult<Value> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    let vm = state
        .repo_read()
        .get_verification_method_by_id(method_id)
        .map_err(ApiError::from)?;
    if vm.project_id != project_id {
        return Err(ApiError::NotFound(
            "verification method not in project".into(),
        ));
    }
    let mut body = payload.into_inner();
    body.id = Some(method_id);
    body.project_id = project_id;
    let mut repo = state.repo_write();
    let ok = repo.edit_verification_method(&body)?;
    if !ok {
        return Err(ApiError::NotFound("verification method not found".into()));
    }
    Ok(json!({ "status": "ok" }))
}

#[delete("/projects/<project_id>/verification-methods/<method_id>")]
pub async fn delete_by_project(
    access: ProjectAccessOrBearer,
    project_id: i32,
    method_id: i32,
    state: &State<AppState>,
) -> ApiResult<Status> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    let vm = state
        .repo_read()
        .get_verification_method_by_id(method_id)
        .map_err(ApiError::from)?;
    if vm.project_id != project_id {
        return Err(ApiError::NotFound(
            "verification method not in project".into(),
        ));
    }
    let mut repo = state.repo_write();
    repo.delete_verification_method(method_id)?;
    Ok(Status::NoContent)
}

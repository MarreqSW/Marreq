// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::services::{
    CommitRequest, DocumentImportError, DocumentImportService, ImportSession, ReviewPatch,
};

fn map_document_import_error(error: DocumentImportError) -> ApiError {
    match error {
        DocumentImportError::BadRequest(message) => ApiError::BadRequest(message),
        DocumentImportError::NotFound(message) => ApiError::NotFound(message),
        DocumentImportError::Conflict(message) => ApiError::Conflict(message),
        DocumentImportError::Internal(message) => ApiError::Internal(message),
        DocumentImportError::Repo(error) => ApiError::from(error),
    }
}

#[get("/projects/<project_id>/document_imports/<session_id>")]
pub async fn get(
    access: ProjectAccessOrBearer,
    project_id: i32,
    session_id: &str,
    state: &State<AppState>,
) -> ApiResult<Json<ImportSession>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    let service = DocumentImportService::new(state.inner());
    let session = service
        .get_session(project_id, access.user().id, session_id)
        .map_err(map_document_import_error)?;
    Ok(Json(session))
}

#[patch(
    "/projects/<project_id>/document_imports/<session_id>",
    data = "<patch>"
)]
pub async fn patch(
    access: ProjectAccessOrBearer,
    project_id: i32,
    session_id: &str,
    patch: Json<ReviewPatch>,
    state: &State<AppState>,
) -> ApiResult<Json<ImportSession>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    let service = DocumentImportService::new(state.inner());
    let session = service
        .apply_review_patch(project_id, access.user().id, session_id, patch.into_inner())
        .await
        .map_err(map_document_import_error)?;
    Ok(Json(session))
}

#[post(
    "/projects/<project_id>/document_imports/<session_id>/commit",
    data = "<request>"
)]
pub async fn commit(
    access: ProjectAccessOrBearer,
    project_id: i32,
    session_id: &str,
    request: Json<CommitRequest>,
    state: &State<AppState>,
) -> ApiResult<Value> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    let service = DocumentImportService::new(state.inner());
    let result = service
        .commit_session(project_id, access.user(), session_id, request.into_inner())
        .await
        .map_err(map_document_import_error)?;
    Ok(json!({
        "status": "ok",
        "result": result,
    }))
}

#[delete("/projects/<project_id>/document_imports/<session_id>")]
pub async fn delete(
    access: ProjectAccessOrBearer,
    project_id: i32,
    session_id: &str,
    state: &State<AppState>,
) -> ApiResult<Status> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    let service = DocumentImportService::new(state.inner());
    service
        .delete_session(project_id, access.user().id, session_id)
        .map_err(map_document_import_error)?;
    Ok(Status::NoContent)
}

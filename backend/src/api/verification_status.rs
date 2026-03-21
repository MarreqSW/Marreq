// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! REST API for verification status (list, get, create, update, delete).
//! System statuses cannot be updated or deleted.

use crate::api::prelude::*;
use crate::models::{NewVerificationStatus, VerificationStatus};
use crate::services::StatusService;

#[get("/verification-status")]
pub async fn list_verification_statuses(
    _user: ApiUser,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<VerificationStatus>>> {
    let service = StatusService::new(state.inner());
    let statuses = service.list_verification_statuses()?;
    Ok(Json(statuses))
}

#[get("/verification-status/<id>")]
pub async fn get_verification_status(
    _user: ApiUser,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Value>> {
    let service = StatusService::new(state.inner());
    let status = service.get_verification_status(id)?;
    Ok(Json(json!({
        "id": status.id,
        "title": status.title,
        "description": status.description,
        "tag": status.tag,
        "project_id": status.project_id,
        "is_system": status.is_system,
        "tag_color": status.tag_color,
    })))
}

#[post("/verification-status", data = "<payload>")]
pub async fn create_verification_status(
    _user: ApiUser,
    state: &State<AppState>,
    payload: Json<NewVerificationStatus>,
) -> ApiResult<(Status, Value)> {
    let service = StatusService::new(state.inner());
    let id = service.create_verification_status(payload.into_inner())?;
    Ok((Status::Created, json!({ "status": "ok", "id": id })))
}

#[put("/verification-status/<id>", data = "<payload>")]
pub async fn update_verification_status(
    _user: ApiUser,
    id: i32,
    state: &State<AppState>,
    payload: Json<NewVerificationStatus>,
) -> ApiResult<Value> {
    let service = StatusService::new(state.inner());
    service.update_verification_status(id, &payload.into_inner())?;
    Ok(json!({ "status": "ok" }))
}

#[delete("/verification-status/<id>")]
pub async fn delete_verification_status(
    _user: ApiUser,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Status> {
    let service = StatusService::new(state.inner());
    service.delete_verification_status(id)?;
    Ok(Status::NoContent)
}

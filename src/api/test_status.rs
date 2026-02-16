//! REST API for test status (list, get, create, update, delete).
//! System statuses cannot be updated or deleted.

use crate::api::prelude::*;
use crate::models::{NewTestStatus, TestStatus};
use crate::services::StatusService;

#[get("/test-status")]
pub async fn list_test_statuses(
    _user: ApiUser,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<TestStatus>>> {
    let service = StatusService::new(state.inner());
    let statuses = service.list_test_statuses()?;
    Ok(Json(statuses))
}

#[get("/test-status/<id>")]
pub async fn get_test_status(
    _user: ApiUser,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Value>> {
    let service = StatusService::new(state.inner());
    let status = service.get_test_status(id)?;
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

#[post("/test-status", data = "<payload>")]
pub async fn create_test_status(
    _user: ApiUser,
    state: &State<AppState>,
    payload: Json<NewTestStatus>,
) -> ApiResult<(Status, Value)> {
    let service = StatusService::new(state.inner());
    let id = service.create_test_status(payload.into_inner())?;
    Ok((Status::Created, json!({ "status": "ok", "id": id })))
}

#[put("/test-status/<id>", data = "<payload>")]
pub async fn update_test_status(
    _user: ApiUser,
    id: i32,
    state: &State<AppState>,
    payload: Json<NewTestStatus>,
) -> ApiResult<Value> {
    let service = StatusService::new(state.inner());
    service.update_test_status(id, &payload.into_inner())?;
    Ok(json!({ "status": "ok" }))
}

#[delete("/test-status/<id>")]
pub async fn delete_test_status(
    _user: ApiUser,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Status> {
    let service = StatusService::new(state.inner());
    service.delete_test_status(id)?;
    Ok(Status::NoContent)
}

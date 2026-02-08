//! API routes for immutable project baselines.

use rocket::serde::Deserialize;

use crate::api::prelude::*;
use crate::models::{Baseline, BaselineTraceability, NewBaseline, Requirement};
use crate::services::BaselineService;

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct CreateBaselineRequest {
    pub name: String,
    pub description: Option<String>,
}

#[get("/projects/<project_id>/baselines")]
pub async fn list(
    _user: ApiUser,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<Baseline>>> {
    let service = BaselineService::new(state.inner());
    let baselines = service.list_by_project(project_id)?;
    Ok(Json(baselines))
}

#[get("/projects/<project_id>/baselines/<baseline_id>")]
pub async fn get(
    _user: ApiUser,
    project_id: i32,
    baseline_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Baseline>> {
    let service = BaselineService::new(state.inner());
    let baseline = service.get_by_id(baseline_id)?;
    if baseline.project_id != project_id {
        return Err(ApiError::NotFound(
            "baseline not found in this project".into(),
        ));
    }
    Ok(Json(baseline))
}

#[post("/projects/<project_id>/baselines", data = "<payload>")]
pub async fn create(
    user: ApiUser,
    project_id: i32,
    state: &State<AppState>,
    payload: Json<CreateBaselineRequest>,
) -> ApiResult<Json<Baseline>> {
    let payload = payload.into_inner();
    let new_baseline = NewBaseline {
        name: payload.name,
        description: payload.description,
    };
    let service = BaselineService::new(state.inner());
    let baseline = service.create_baseline(project_id, user.user().id, &new_baseline)?;
    Ok(Json(baseline))
}

/// Retrieve baseline contents: requirements as at baseline time (from snapshot).
#[get("/projects/<project_id>/baselines/<baseline_id>/requirements")]
pub async fn get_requirements(
    _user: ApiUser,
    project_id: i32,
    baseline_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<Requirement>>> {
    let service = BaselineService::new(state.inner());
    let baseline = service.get_by_id(baseline_id)?;
    if baseline.project_id != project_id {
        return Err(ApiError::NotFound(
            "baseline not found in this project".into(),
        ));
    }
    let requirements = service.get_requirements(baseline_id)?;
    Ok(Json(requirements))
}

/// Retrieve baseline traceability snapshot (requirement–test links).
#[get("/projects/<project_id>/baselines/<baseline_id>/traceability")]
pub async fn get_traceability(
    _user: ApiUser,
    project_id: i32,
    baseline_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<BaselineTraceability>>> {
    let service = BaselineService::new(state.inner());
    let baseline = service.get_by_id(baseline_id)?;
    if baseline.project_id != project_id {
        return Err(ApiError::NotFound(
            "baseline not found in this project".into(),
        ));
    }
    let traceability = service.get_traceability(baseline_id)?;
    Ok(Json(traceability))
}

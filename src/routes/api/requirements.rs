//! Requirements API routes.
//!
//! This module contains all API endpoints for requirement management.

use rocket::serde::json::Json;
use rocket::State;
use crate::errors::{ApiResponse, ApiResponseResult};
use crate::models::*;
use crate::services::RequirementService;

/// Get all requirements
#[get("/requirements")]
pub async fn get_requirements(
    service: &State<RequirementService>,
) -> ApiResponseResult<Vec<Requirement>> {
    let requirements = service.get_all_requirements().await?;
    Ok(Json(ApiResponse::success(requirements)))
}

/// Get requirements by project
#[get("/requirements?<project_id>")]
pub async fn get_requirements_by_project(
    project_id: i32,
    service: &State<RequirementService>,
) -> ApiResponseResult<Vec<Requirement>> {
    let requirements = service.get_requirements_by_project(project_id).await?;
    Ok(Json(ApiResponse::success(requirements)))
}

/// Get a specific requirement by ID
#[get("/requirements/<id>")]
pub async fn get_requirement_by_id(
    id: i32,
    service: &State<RequirementService>,
) -> ApiResponseResult<Requirement> {
    let requirement = service.get_requirement_by_id(id).await?;
    Ok(Json(ApiResponse::success(requirement)))
}

/// Create a new requirement
#[post("/requirements", data = "<new_req>")]
pub async fn create_requirement(
    new_req: Json<NewRequirement>,
    service: &State<RequirementService>,
) -> ApiResponseResult<i32> {
    let id = service.create_requirement(new_req.into_inner(), 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(id)))
}

/// Update an existing requirement
#[put("/requirements/<id>", data = "<updated_req>")]
pub async fn update_requirement(
    id: i32,
    updated_req: Json<NewRequirement>,
    service: &State<RequirementService>,
) -> ApiResponseResult<bool> {
    let success = service.update_requirement(id, updated_req.into_inner(), 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(success)))
}

/// Delete a requirement
#[delete("/requirements/<id>")]
pub async fn delete_requirement(
    id: i32,
    service: &State<RequirementService>,
) -> ApiResponseResult<bool> {
    let success = service.delete_requirement(id, 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(success)))
}

/// Get requirements by category
#[get("/requirements/category/<category_id>")]
pub async fn get_requirements_by_category(
    category_id: i32,
    service: &State<RequirementService>,
) -> ApiResponseResult<Vec<Requirement>> {
    let requirements = service.get_requirements_by_category(category_id).await?;
    Ok(Json(ApiResponse::success(requirements)))
}

/// Get requirements by status
#[get("/requirements/status/<status_id>")]
pub async fn get_requirements_by_status(
    status_id: i32,
    service: &State<RequirementService>,
) -> ApiResponseResult<Vec<Requirement>> {
    let requirements = service.get_requirements_by_status(status_id).await?;
    Ok(Json(ApiResponse::success(requirements)))
}

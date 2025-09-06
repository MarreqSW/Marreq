//! Status API routes.
//!
//! This module contains all API endpoints for status management.

use rocket::serde::json::Json;
use rocket::State;
use crate::errors::{ApiResponse, ApiResponseResult};
use crate::models::*;
use crate::services::StatusService;

/// Get all requirement status options
#[get("/requirement-status")]
pub async fn get_requirement_status(
    service: &State<StatusService>,
) -> ApiResponseResult<Vec<RequirementStatus>> {
    let status = service.get_all_requirement_status().await?;
    Ok(Json(ApiResponse::success(status)))
}

/// Get all test status options
#[get("/test-status")]
pub async fn get_test_status(
    service: &State<StatusService>,
) -> ApiResponseResult<Vec<TestStatus>> {
    let status = service.get_all_test_status().await?;
    Ok(Json(ApiResponse::success(status)))
}

/// Create a new requirement status
#[post("/requirement-status", data = "<new_status>")]
pub async fn create_requirement_status(
    new_status: Json<NewRequirementStatus>,
    service: &State<StatusService>,
) -> ApiResponseResult<i32> {
    let id = service.create_requirement_status(new_status.into_inner(), 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(id)))
}

/// Create a new test status
#[post("/test-status", data = "<new_status>")]
pub async fn create_test_status(
    new_status: Json<NewTestStatus>,
    service: &State<StatusService>,
) -> ApiResponseResult<i32> {
    let id = service.create_test_status(new_status.into_inner(), 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(id)))
}

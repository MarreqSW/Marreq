//! Applicability API routes.
//!
//! This module contains all API endpoints for applicability management.

use rocket::serde::json::Json;
use rocket::State;
use crate::errors::{ApiResponse, ApiResponseResult};
use crate::models::*;
use crate::services::ApplicabilityService;

/// Get all applicability options
#[get("/applicability")]
pub async fn get_applicability(
    service: &State<ApplicabilityService>,
) -> ApiResponseResult<Vec<Applicability>> {
    let applicability = service.get_all_applicability().await?;
    Ok(Json(ApiResponse::success(applicability)))
}

/// Get a specific applicability by ID
#[get("/applicability/<id>")]
pub async fn get_applicability_by_id(
    id: i32,
    service: &State<ApplicabilityService>,
) -> ApiResponseResult<Applicability> {
    let applicability = service.get_applicability_by_id(id).await?;
    Ok(Json(ApiResponse::success(applicability)))
}

/// Create a new applicability
#[post("/applicability", data = "<new_applicability>")]
pub async fn create_applicability(
    new_applicability: Json<NewApplicability>,
    service: &State<ApplicabilityService>,
) -> ApiResponseResult<i32> {
    let id = service.create_applicability(new_applicability.into_inner(), 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(id)))
}

/// Update an existing applicability
#[put("/applicability/<id>", data = "<updated_applicability>")]
pub async fn update_applicability(
    id: i32,
    updated_applicability: Json<NewApplicability>,
    service: &State<ApplicabilityService>,
) -> ApiResponseResult<bool> {
    let success = service.update_applicability(id, updated_applicability.into_inner(), 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(success)))
}

/// Delete an applicability
#[delete("/applicability/<id>")]
pub async fn delete_applicability(
    id: i32,
    service: &State<ApplicabilityService>,
) -> ApiResponseResult<bool> {
    let success = service.delete_applicability(id, 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(success)))
}

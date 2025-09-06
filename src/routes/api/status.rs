//! Status API routes.
//!
//! This module contains all API endpoints for status management.

use rocket::serde::json::Json;
use rocket::State;
use crate::errors::{ApiResponse, ApiResponseResult};
use crate::models::*;
use crate::services::StatusService;

/// Get all status options
#[get("/status")]
pub async fn get_status(
    service: &State<StatusService>,
) -> ApiResponseResult<Vec<Status>> {
    let status = service.get_all_status().await?;
    Ok(Json(ApiResponse::success(status)))
}

/// Create a new status
#[post("/status", data = "<new_status>")]
pub async fn create_status(
    new_status: Json<NewStatus>,
    service: &State<StatusService>,
) -> ApiResponseResult<i32> {
    let id = service.create_status(new_status.into_inner(), 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(id)))
}

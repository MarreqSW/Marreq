//! Matrix API routes.
//!
//! This module contains all API endpoints for traceability matrix management.

use rocket::serde::json::Json;
use rocket::State;
use crate::errors::{ApiResponse, ApiResponseResult};
use crate::models::*;
use crate::services::MatrixService;

/// Get all matrix links
#[get("/matrix")]
pub async fn get_matrix(
    service: &State<MatrixService>,
) -> ApiResponseResult<Vec<Matrix>> {
    let matrix = service.get_all_matrix().await?;
    Ok(Json(ApiResponse::success(matrix)))
}

/// Get matrix links by project
#[get("/matrix?<project_id>")]
pub async fn get_matrix_by_project(
    project_id: i32,
    service: &State<MatrixService>,
) -> ApiResponseResult<Vec<Matrix>> {
    let matrix = service.get_matrix_by_project(project_id).await?;
    Ok(Json(ApiResponse::success(matrix)))
}

/// Create a new matrix link
#[post("/matrix", data = "<matrix_link>")]
pub async fn create_matrix_link(
    matrix_link: Json<MatrixLinkRequest>,
    service: &State<MatrixService>,
) -> ApiResponseResult<bool> {
    let success = service.create_matrix_link(
        matrix_link.req_id,
        matrix_link.test_id,
        matrix_link.project_id,
        0, // TODO: Get user_id from auth
    ).await?;
    Ok(Json(ApiResponse::success(success)))
}

/// Delete a matrix link
#[delete("/matrix/<req_id>/<test_id>")]
pub async fn delete_matrix_link(
    req_id: i32,
    test_id: i32,
    service: &State<MatrixService>,
) -> ApiResponseResult<bool> {
    // We need to get the project_id from the matrix link first
    // For now, we'll use a default value - this should be improved
    let success = service.delete_matrix_link(req_id, test_id, 1, 0).await?; // TODO: Get project_id and user_id properly
    Ok(Json(ApiResponse::success(success)))
}

/// Request structure for creating matrix links
#[derive(serde::Deserialize)]
pub struct MatrixLinkRequest {
    pub req_id: i32,
    pub test_id: i32,
    pub project_id: i32,
}

//! Projects API routes.
//!
//! This module contains all API endpoints for project management.

use rocket::serde::json::Json;
use rocket::State;
use crate::errors::{ApiResponse, ApiResponseResult};
use crate::models::*;
use crate::services::ProjectService;

/// Get all projects
#[get("/projects")]
pub async fn get_projects(
    service: &State<ProjectService>,
) -> ApiResponseResult<Vec<Project>> {
    let projects = service.get_all_projects().await?;
    Ok(Json(ApiResponse::success(projects)))
}

/// Get a specific project by ID
#[get("/projects/<id>")]
pub async fn get_project_by_id(
    id: i32,
    service: &State<ProjectService>,
) -> ApiResponseResult<Project> {
    let project = service.get_project_by_id(id).await?;
    Ok(Json(ApiResponse::success(project)))
}

/// Create a new project
#[post("/projects", data = "<new_project>")]
pub async fn create_project(
    new_project: Json<NewProject>,
    service: &State<ProjectService>,
) -> ApiResponseResult<i32> {
    let id = service.create_project(new_project.into_inner(), 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(id)))
}

/// Update an existing project
#[put("/projects/<id>", data = "<updated_project>")]
pub async fn update_project(
    id: i32,
    updated_project: Json<NewProject>,
    service: &State<ProjectService>,
) -> ApiResponseResult<bool> {
    let success = service.update_project(id, updated_project.into_inner(), 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(success)))
}

/// Delete a project
#[delete("/projects/<id>")]
pub async fn delete_project(
    id: i32,
    service: &State<ProjectService>,
) -> ApiResponseResult<bool> {
    let success = service.delete_project(id, 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(success)))
}

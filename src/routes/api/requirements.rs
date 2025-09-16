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

/// Partial update for requirement fields (for inline editing)
#[put("/requirements/<id>/field", data = "<field_update>")]
pub async fn update_requirement_field(
    id: i32,
    field_update: Json<serde_json::Value>,
    service: &State<RequirementService>,
) -> ApiResponseResult<bool> {
    // Get the current requirement
    let current_req = service.get_requirement_by_id(id).await?;
    
    // Create a new requirement with the updated field
    let mut updated_req = NewRequirement {
        req_id: Some(current_req.req_id),
        req_title: current_req.req_title,
        req_description: current_req.req_description,
        req_verification: current_req.req_verification,
        req_author: current_req.req_author,
        req_link: current_req.req_link,
        req_category: current_req.req_category,
        req_current_status: current_req.req_current_status,
        req_parent: current_req.req_parent,
        req_reference: current_req.req_reference,
        req_reviewer: current_req.req_reviewer,
        req_applicability: current_req.req_applicability,
        req_justification: current_req.req_justification,
        project_id: current_req.project_id,
    };
    
    // Update the specific field
    if let Some(field_data) = field_update.as_object() {
        for (key, value) in field_data {
            match key.as_str() {
                "req_title" => {
                    if let Some(title) = value.as_str() {
                        updated_req.req_title = title.to_string();
                    }
                }
                "req_reference" => {
                    if let Some(reference) = value.as_str() {
                        updated_req.req_reference = reference.to_string();
                    }
                }
                "req_category" => {
                    if let Some(category) = value.as_i64() {
                        updated_req.req_category = category as i32;
                    }
                }
                "req_current_status" => {
                    if let Some(status) = value.as_i64() {
                        updated_req.req_current_status = status as i32;
                    }
                }
                "req_verification" => {
                    if let Some(verification) = value.as_i64() {
                        updated_req.req_verification = verification as i32;
                    }
                }
                "req_author" => {
                    if let Some(author) = value.as_i64() {
                        updated_req.req_author = author as i32;
                    }
                }
                "req_reviewer" => {
                    if let Some(reviewer) = value.as_i64() {
                        updated_req.req_reviewer = reviewer as i32;
                    }
                }
                "req_deadline_date" => {
                    if let Some(_deadline) = value.as_str() {
                        // Handle deadline date if needed
                        // For now, we'll skip this as it's not in the NewRequirement struct
                    }
                }
                _ => {
                    // Unknown field, skip
                }
            }
        }
    }
    
    let success = service.update_requirement(id, updated_req, 0).await?; // TODO: Get user_id from auth
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

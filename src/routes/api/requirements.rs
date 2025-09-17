//! Requirements API routes.
//!
//! This module contains all API endpoints for requirement management.

use rocket::serde::json::Json;
use rocket::State;
use crate::errors::{ApiResponse, ApiResponseResult};
use crate::models::*;
use crate::services::RequirementService;
use crate::repository::RequirementsRepository;

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
    
    // Track which fields are being updated
    let mut updated_fields = std::collections::HashSet::new();
    
    // Update the specific field
    if let Some(field_data) = field_update.as_object() {
        for (key, value) in field_data {
            match key.as_str() {
                "req_title" => {
                    if let Some(title) = value.as_str() {
                        updated_req.req_title = title.to_string();
                        updated_fields.insert("req_title");
                    }
                }
                "req_reference" => {
                    if let Some(reference) = value.as_str() {
                        updated_req.req_reference = reference.to_string();
                        updated_fields.insert("req_reference");
                    }
                }
                "req_category" => {
                    if let Some(category) = value.as_i64() {
                        updated_req.req_category = category as i32;
                        updated_fields.insert("req_category");
                    } else if let Some(category_str) = value.as_str() {
                        if let Ok(category) = category_str.parse::<i32>() {
                            updated_req.req_category = category;
                            updated_fields.insert("req_category");
                        }
                    }
                }
                "req_current_status" => {
                    if let Some(status) = value.as_i64() {
                        updated_req.req_current_status = status as i32;
                        updated_fields.insert("req_current_status");
                    } else if let Some(status_str) = value.as_str() {
                        if let Ok(status) = status_str.parse::<i32>() {
                            updated_req.req_current_status = status;
                            updated_fields.insert("req_current_status");
                        }
                    }
                }
                "req_verification" => {
                    if let Some(verification) = value.as_i64() {
                        updated_req.req_verification = verification as i32;
                        updated_fields.insert("req_verification");
                    } else if let Some(verification_str) = value.as_str() {
                        if let Ok(verification) = verification_str.parse::<i32>() {
                            updated_req.req_verification = verification;
                            updated_fields.insert("req_verification");
                        }
                    }
                }
                "req_author" => {
                    if let Some(author) = value.as_i64() {
                        updated_req.req_author = author as i32;
                        updated_fields.insert("req_author");
                    } else if let Some(author_str) = value.as_str() {
                        if let Ok(author) = author_str.parse::<i32>() {
                            updated_req.req_author = author;
                            updated_fields.insert("req_author");
                        }
                    }
                }
                "req_reviewer" => {
                    if let Some(reviewer) = value.as_i64() {
                        updated_req.req_reviewer = reviewer as i32;
                        updated_fields.insert("req_reviewer");
                    } else if let Some(reviewer_str) = value.as_str() {
                        if let Ok(reviewer) = reviewer_str.parse::<i32>() {
                            updated_req.req_reviewer = reviewer;
                            updated_fields.insert("req_reviewer");
                        }
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
    
    // Only validate fields that are being updated
    if updated_fields.contains("req_title") {
        if updated_req.req_title.trim().is_empty() {
            return Ok(Json(ApiResponse::error("Title cannot be empty".to_string())));
        }
        if updated_req.req_title.len() > 255 {
            return Ok(Json(ApiResponse::error("Title is too long (max 255 characters)".to_string())));
        }
        if updated_req.req_title.len() < 3 {
            return Ok(Json(ApiResponse::error("Title is too short (min 3 characters)".to_string())));
        }
    }
    
    if updated_fields.contains("req_reference") {
        if !updated_req.req_reference.trim().is_empty() {
            let ref_regex = regex::Regex::new(r"^[A-Z]{2,4}-[A-Z0-9]{3,6}$").unwrap();
            if !ref_regex.is_match(&updated_req.req_reference) {
                return Ok(Json(ApiResponse::error("Reference should be in format like REQ-001 or REQ-ABC-001".to_string())));
            }
        }
    }
    
    if updated_fields.contains("req_link") {
        if !updated_req.req_link.trim().is_empty() {
            let url_regex = regex::Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap();
            if !url_regex.is_match(&updated_req.req_link) {
                return Ok(Json(ApiResponse::error("Link must be a valid HTTP/HTTPS URL".to_string())));
            }
        }
    }
    
    // Validate IDs are positive for updated fields
    if updated_fields.contains("req_verification") && updated_req.req_verification <= 0 {
        return Ok(Json(ApiResponse::error("Verification method ID must be positive".to_string())));
    }
    if updated_fields.contains("req_current_status") && updated_req.req_current_status <= 0 {
        return Ok(Json(ApiResponse::error("Status ID must be positive".to_string())));
    }
    if updated_fields.contains("req_author") && updated_req.req_author <= 0 {
        return Ok(Json(ApiResponse::error("Author ID must be positive".to_string())));
    }
    if updated_fields.contains("req_reviewer") && updated_req.req_reviewer <= 0 {
        return Ok(Json(ApiResponse::error("Reviewer ID must be positive".to_string())));
    }
    if updated_fields.contains("req_category") && updated_req.req_category <= 0 {
        return Ok(Json(ApiResponse::error("Category ID must be positive".to_string())));
    }
    
    // Update directly in database to bypass service layer validation
    let mut repo = crate::repository::DieselRepo::new();
    let success = repo.edit_requirement(&updated_req)
        .map_err(|e| crate::errors::ApiError::Repository(e))?;
    
    // Invalidate relevant caches
    crate::cache::invalidate_requirement_cache(id);
    crate::cache::invalidate_project_cache(updated_req.project_id);
    
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

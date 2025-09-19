use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::http::Status;

use crate::models::*;
use crate::repository::{DieselCachedRepo, RequirementsRepository};

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FieldUpdateRequest {
    pub field: String,
    pub value: String,
}

/// Update a specific field of a requirement
#[post("/requirements/<id>/field", data = "<update_request>")]
pub fn update_requirement_field(
    id: i32,
    update_request: Json<FieldUpdateRequest>,
) -> Result<Json<serde_json::Value>, Status> {
    let field = &update_request.field;
    let value = &update_request.value;

    // Get the current requirement
    let mut requirement = match DieselCachedRepo::read().get_requirement_by_id(id) {
        Ok(req) => req,
        Err(_) => return Err(Status::NotFound),
    };

    // Update the specific field
    match field.as_str() {
        "req_title" => {
            requirement.req_title = value.clone();
        }
        "req_description" => {
            requirement.req_description = value.clone();
        }
        "req_current_status" => {
            if let Ok(status_id) = value.parse::<i32>() {
                requirement.req_current_status = status_id;
            } else {
                return Err(Status::BadRequest);
            }
        }
        "req_verification" => {
            if let Ok(verification_id) = value.parse::<i32>() {
                requirement.req_verification = verification_id;
            } else {
                return Err(Status::BadRequest);
            }
        }
        "req_author" => {
            if let Ok(author_id) = value.parse::<i32>() {
                requirement.req_author = author_id;
            } else {
                return Err(Status::BadRequest);
            }
        }
        "req_reviewer" => {
            if let Ok(reviewer_id) = value.parse::<i32>() {
                requirement.req_reviewer = reviewer_id;
            } else {
                return Err(Status::BadRequest);
            }
        }
        "req_category" => {
            if let Ok(category_id) = value.parse::<i32>() {
                requirement.req_category = category_id;
            } else {
                return Err(Status::BadRequest);
            }
        }
        "req_applicability" => {
            if let Ok(applicability_id) = value.parse::<i32>() {
                requirement.req_applicability = applicability_id;
            } else {
                return Err(Status::BadRequest);
            }
        }
        _ => {
            return Err(Status::BadRequest);
        }
    }

    // Create a NewRequirement for updating
    let new_requirement = NewRequirement {
        req_id: Some(requirement.req_id),
        req_title: requirement.req_title.clone(),
        req_description: requirement.req_description.clone(),
        req_verification: requirement.req_verification,
        req_current_status: requirement.req_current_status,
        req_author: requirement.req_author,
        req_reviewer: requirement.req_reviewer,
        req_link: requirement.req_link.clone(),
        req_reference: requirement.req_reference.clone(),
        req_category: requirement.req_category,
        req_applicability: requirement.req_applicability,
        req_parent: requirement.req_parent,
        req_justification: requirement.req_justification.clone(),
        project_id: requirement.project_id,
    };

    // Update the requirement
    match DieselCachedRepo::write().edit_requirement(&new_requirement) {
        Ok(_) => {
            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Field updated successfully"
            })))
        }
        Err(_) => Err(Status::InternalServerError),
    }
}
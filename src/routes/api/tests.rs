use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::http::Status;

use crate::models::*;
use crate::repository::{DieselCachedRepo, TestsRepository};

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FieldUpdateRequest {
    pub field: String,
    pub value: String,
}

/// Update a specific field of a test
#[post("/tests/<id>/field", data = "<update_request>")]
pub fn update_test_field(
    id: i32,
    update_request: Json<FieldUpdateRequest>,
) -> Result<Json<serde_json::Value>, Status> {
    let field = &update_request.field;
    let value = &update_request.value;

    // Get the current test
    let mut test = match DieselCachedRepo::read().get_test_by_id(id) {
        Ok(t) => t,
        Err(_) => return Err(Status::NotFound),
    };

    // Update the specific field
    match field.as_str() {
        "test_name" => {
            test.test_name = value.clone();
        }
        "test_description" => {
            test.test_description = value.clone();
        }
        "test_source" => {
            test.test_source = value.clone();
        }
        "test_status" => {
            if let Ok(status_id) = value.parse::<i32>() {
                test.test_status = status_id;
            } else {
                return Err(Status::BadRequest);
            }
        }
        "test_reference" => {
            test.test_reference = value.clone();
        }
        "test_parent" => {
            if let Ok(parent_id) = value.parse::<i32>() {
                test.test_parent = parent_id;
            } else {
                return Err(Status::BadRequest);
            }
        }
        _ => {
            return Err(Status::BadRequest);
        }
    }

    // Create a NewTest for updating
    let new_test = NewTest {
        test_id: Some(test.test_id),
        test_name: test.test_name.clone(),
        test_description: test.test_description.clone(),
        test_source: test.test_source.clone(),
        test_status: test.test_status,
        test_reference: test.test_reference.clone(),
        test_parent: test.test_parent,
        project_id: test.project_id,
    };

    // Update the test
    match DieselCachedRepo::write().edit_test(&new_test) {
        Ok(_) => {
            Ok(Json(serde_json::json!({
                "success": true,
                "message": "Field updated successfully"
            })))
        }
        Err(_) => Err(Status::InternalServerError),
    }
}
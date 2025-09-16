//! Tests API routes.
//!
//! This module contains all API endpoints for test management.

use rocket::serde::json::Json;
use rocket::State;
use crate::errors::{ApiResponse, ApiResponseResult};
use crate::models::*;
use crate::services::TestService;
use crate::repository::TestsRepository;

/// Get all tests
#[get("/tests")]
pub async fn get_tests(
    service: &State<TestService>,
) -> ApiResponseResult<Vec<Test>> {
    let tests = service.get_all_tests().await?;
    Ok(Json(ApiResponse::success(tests)))
}

/// Get tests by project
#[get("/tests?<project_id>")]
pub async fn get_tests_by_project(
    project_id: i32,
    service: &State<TestService>,
) -> ApiResponseResult<Vec<Test>> {
    let tests = service.get_tests_by_project(project_id).await?;
    Ok(Json(ApiResponse::success(tests)))
}

/// Get a specific test by ID
#[get("/tests/<id>")]
pub async fn get_test_by_id(
    id: i32,
    service: &State<TestService>,
) -> ApiResponseResult<Test> {
    let test = service.get_test_by_id(id).await?;
    Ok(Json(ApiResponse::success(test)))
}

/// Create a new test
#[post("/tests", data = "<new_test>")]
pub async fn create_test(
    new_test: Json<NewTest>,
    service: &State<TestService>,
) -> ApiResponseResult<i32> {
    let id = service.create_test(new_test.into_inner(), 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(id)))
}

/// Update an existing test
#[put("/tests/<id>", data = "<updated_test>")]
pub async fn update_test(
    id: i32,
    updated_test: Json<NewTest>,
    service: &State<TestService>,
) -> ApiResponseResult<bool> {
    let success = service.update_test(id, updated_test.into_inner(), 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(success)))
}

/// Partial update for test fields (for inline editing)
#[put("/tests/<id>/field", data = "<field_update>")]
pub async fn update_test_field(
    id: i32,
    field_update: Json<serde_json::Value>,
    service: &State<TestService>,
) -> ApiResponseResult<bool> {
    // Get the current test
    let current_test = service.get_test_by_id(id).await?;
    
    // Create a new test with the updated field
    let mut updated_test = NewTest {
        test_id: Some(current_test.test_id),
        test_name: current_test.test_name,
        test_description: current_test.test_description,
        test_source: current_test.test_source,
        test_reference: current_test.test_reference,
        test_status: current_test.test_status,
        test_parent: current_test.test_parent,
        project_id: current_test.project_id,
    };
    
    // Update the specific field
    if let Some(field_data) = field_update.as_object() {
        for (key, value) in field_data {
            match key.as_str() {
                "test_name" => {
                    if let Some(name) = value.as_str() {
                        updated_test.test_name = name.to_string();
                    }
                }
                "test_reference" => {
                    if let Some(reference) = value.as_str() {
                        updated_test.test_reference = reference.to_string();
                    }
                }
                "test_description" => {
                    if let Some(description) = value.as_str() {
                        updated_test.test_description = description.to_string();
                    }
                }
                "test_status" => {
                    if let Some(status) = value.as_i64() {
                        updated_test.test_status = status as i32;
                    }
                }
                "test_source" => {
                    if let Some(source) = value.as_str() {
                        updated_test.test_source = source.to_string();
                    }
                }
                "test_parent" => {
                    if let Some(parent) = value.as_i64() {
                        updated_test.test_parent = parent as i32;
                    }
                }
                _ => {
                    // Unknown field, skip
                }
            }
        }
    }
    
    // Update directly in database to bypass service layer validation
    let mut repo = crate::repository::DieselRepo::new();
    let success = repo.edit_test(&updated_test)
        .map_err(|e| crate::errors::ApiError::Repository(e))?;
    
    // Invalidate relevant caches
    crate::cache::invalidate_test_cache(id);
    crate::cache::invalidate_project_cache(updated_test.project_id);
    
    Ok(Json(ApiResponse::success(success)))
}

/// Delete a test
#[delete("/tests/<id>")]
pub async fn delete_test(
    id: i32,
    service: &State<TestService>,
) -> ApiResponseResult<bool> {
    let success = service.delete_test(id, 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(success)))
}

/// Get tests by status
#[get("/tests/status/<status_id>")]
pub async fn get_tests_by_status(
    status_id: i32,
    service: &State<TestService>,
) -> ApiResponseResult<Vec<Test>> {
    let tests = service.get_tests_by_status(status_id).await?;
    Ok(Json(ApiResponse::success(tests)))
}

/// Get tests by parent
#[get("/tests/parent/<parent_id>")]
pub async fn get_tests_by_parent(
    parent_id: i32,
    service: &State<TestService>,
) -> ApiResponseResult<Vec<Test>> {
    let tests = service.get_tests_by_parent(parent_id).await?;
    Ok(Json(ApiResponse::success(tests)))
}

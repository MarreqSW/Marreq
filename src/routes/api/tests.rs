//! Tests API routes.
//!
//! This module contains all API endpoints for test management.

use rocket::serde::json::Json;
use rocket::State;
use crate::errors::{ApiResponse, ApiResponseResult};
use crate::models::*;
use crate::services::TestService;

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

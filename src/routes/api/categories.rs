//! Categories API routes.
//!
//! This module contains all API endpoints for category management.

use rocket::serde::json::Json;
use rocket::State;
use crate::errors::{ApiResponse, ApiResponseResult};
use crate::models::*;
use crate::services::CategoryService;

/// Get all categories
#[get("/categories")]
pub async fn get_categories(
    service: &State<CategoryService>,
) -> ApiResponseResult<Vec<Category>> {
    let categories = service.get_all_categories().await?;
    Ok(Json(ApiResponse::success(categories)))
}

/// Get a specific category by ID
#[get("/categories/<id>")]
pub async fn get_category_by_id(
    id: i32,
    service: &State<CategoryService>,
) -> ApiResponseResult<Category> {
    let category = service.get_category_by_id(id).await?;
    Ok(Json(ApiResponse::success(category)))
}

/// Create a new category
#[post("/categories", data = "<new_category>")]
pub async fn create_category(
    new_category: Json<NewCategory>,
    service: &State<CategoryService>,
) -> ApiResponseResult<i32> {
    let id = service.create_category(new_category.into_inner(), 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(id)))
}

/// Update an existing category
#[put("/categories/<id>", data = "<updated_category>")]
pub async fn update_category(
    id: i32,
    updated_category: Json<NewCategory>,
    service: &State<CategoryService>,
) -> ApiResponseResult<bool> {
    let success = service.update_category(id, updated_category.into_inner(), 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(success)))
}

/// Delete a category
#[delete("/categories/<id>")]
pub async fn delete_category(
    id: i32,
    service: &State<CategoryService>,
) -> ApiResponseResult<bool> {
    let success = service.delete_category(id, 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(success)))
}

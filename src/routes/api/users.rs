//! Users API routes.
//!
//! This module contains all API endpoints for user management.

use rocket::serde::json::Json;
use rocket::State;
use crate::errors::{ApiResponse, ApiResponseResult};
use crate::models::*;
use crate::services::UserService;

/// Get all users
#[get("/users")]
pub async fn get_users(
    service: &State<UserService>,
) -> ApiResponseResult<Vec<User>> {
    let users = service.get_all_users().await?;
    Ok(Json(ApiResponse::success(users)))
}

/// Get a specific user by ID
#[get("/users/<id>")]
pub async fn get_user_by_id(
    id: i32,
    service: &State<UserService>,
) -> ApiResponseResult<User> {
    let user = service.get_user_by_id(id).await?;
    Ok(Json(ApiResponse::success(user)))
}

/// Create a new user
#[post("/users", data = "<new_user>")]
pub async fn create_user(
    new_user: Json<NewUser>,
    service: &State<UserService>,
) -> ApiResponseResult<i32> {
    let id = service.create_user(new_user.into_inner(), 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(id)))
}

/// Delete a user
#[delete("/users/<id>")]
pub async fn delete_user(
    id: i32,
    service: &State<UserService>,
) -> ApiResponseResult<bool> {
    let success = service.delete_user(id, 0).await?; // TODO: Get user_id from auth
    Ok(Json(ApiResponse::success(success)))
}

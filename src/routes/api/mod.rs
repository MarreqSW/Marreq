//! API routes module for the ReqMan application.
//!
//! This module contains all API route handlers organized by entity type.

pub mod requirements;
pub mod tests;
pub mod categories;
pub mod applicability;
pub mod users;
pub mod projects;
pub mod status;
pub mod matrix;
pub mod docs;

use rocket::serde::json::Json;
use crate::errors::{ApiResponse, ApiResponseResult};
use crate::services::*;

/// Health check endpoint
#[get("/health")]
pub fn health_check() -> Json<ApiResponse<String>> {
    Json(ApiResponse::success("OK".to_string()))
}

/// API version information
#[get("/version")]
pub fn api_version() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "version": "1.0.0",
        "name": "ReqMan API",
        "description": "Requirements Management System API"
    })))
}

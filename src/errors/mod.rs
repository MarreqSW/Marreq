//! Error types and handling for the ReqMan application.
//!
//! This module provides comprehensive error handling with proper error types,
//! conversion implementations, and user-friendly error messages.

use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::response::{Responder, Response};
use rocket::request::Request;
use serde::Serialize;
use std::fmt;
use thiserror::Error;

/// Main error type for the application
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),
    
    #[error("Repository error: {0}")]
    Repository(#[from] crate::repository::errors::RepoError),
    
    #[error("Not found: {entity} with id {id}")]
    NotFound { entity: String, id: i32 },
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    #[error("Authorization error: {0}")]
    Authorization(String),
    
    #[error("Internal server error: {0}")]
    Internal(String),
    
    #[error("Cache error: {0}")]
    Cache(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {  
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: String,
}

impl<T: Serialize> fmt::Display for ApiResponse<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string()))
    }
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a successful response with data
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    /// Create an error response with message
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Validation error for input validation
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Field '{field}' is required")]
    Required { field: String },
    
    #[error("Field '{field}' is too long (max {max} characters)")]
    TooLong { field: String, max: usize },
    
    #[error("Field '{field}' is too short (min {min} characters)")]
    TooShort { field: String, min: usize },
    
    #[error("Invalid format for field '{field}': {message}")]
    InvalidFormat { field: String, message: String },
    
    #[error("Custom validation error: {0}")]
    Custom(String),
}

impl From<ValidationError> for ApiError {
    fn from(err: ValidationError) -> Self {
        ApiError::Validation(err.to_string())
    }
}

/// Convert ApiError to HTTP Status
impl From<ApiError> for Status {
    fn from(err: ApiError) -> Self {
        match err {
            ApiError::NotFound { .. } => Status::NotFound,
            ApiError::Validation(_) => Status::BadRequest,
            ApiError::Authentication(_) => Status::Unauthorized,
            ApiError::Authorization(_) => Status::Forbidden,
            ApiError::Database(_) | ApiError::Repository(_) | ApiError::Internal(_) => Status::InternalServerError,
            ApiError::Cache(_) => Status::ServiceUnavailable,
            ApiError::Serialization(_) => Status::BadRequest,
        }
    }
}

/// Convert ApiError to JSON response
impl From<ApiError> for Json<ApiResponse<()>> {
    fn from(err: ApiError) -> Self {
        Json(ApiResponse::<()>::error(err.to_string()))
    }
}

/// Implement Responder for ApiError
impl<'r> Responder<'r, 'static> for ApiError {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        let error_msg = self.to_string();
        let status: Status = self.into();
        let response = ApiResponse::<()>::error(error_msg);
        let json_str = serde_json::to_string(&response).unwrap_or_else(|_| "{\"success\":false,\"error\":\"Serialization error\"}".to_string());
        Response::build()
            .status(status)
            .sized_body(json_str.len(), std::io::Cursor::new(json_str))
            .header(rocket::http::ContentType::JSON)
            .ok()
    }
}

/// Result type alias for API operations
pub type ApiResult<T> = Result<T, ApiError>;

/// Result type alias for API responses
pub type ApiResponseResult<T> = Result<Json<ApiResponse<T>>, ApiError>;

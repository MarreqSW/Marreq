//! Service layer for the ReqMan application.
//!
//! This module provides business logic services that abstract database operations
//! and provide a clean interface for route handlers.

pub mod applicability_service;
pub mod base_service;
pub mod cache_service;
pub mod category_service;
pub mod decorated_requirement_service;
pub mod log_service;
pub mod matrix_service;
pub mod project_service;
pub mod requirement_analytics_service;
pub mod requirement_service;
pub mod status_service;
pub mod test_service;
pub mod user_service;
pub mod verification_service;

#[cfg(test)]
mod tests;

pub use applicability_service::*;
pub use base_service::*;
pub use cache_service::*;
pub use category_service::*;
pub use decorated_requirement_service::*;
pub use log_service::*;
pub use matrix_service::*;
pub use project_service::*;
pub use requirement_analytics_service::*;
pub use requirement_service::*;
pub use status_service::*;
pub use test_service::*;
pub use user_service::*;
pub use verification_service::*;

//! Service layer for the ReqMan application.
//!
//! This module provides business logic services that abstract database operations
//! and provide a clean interface for route handlers.

//pub mod base_service;
pub mod applicability_service;
//pub mod requirement_service;
//pub mod test_service;
//pub mod category_service;
//pub mod user_service;
//pub mod project_service;
//pub mod status_service;
//pub mod matrix_service;

//#[cfg(test)]
//mod tests;

//pub use base_service::*;
pub use applicability_service::*;
//pub use requirement_service::*;
//pub use test_service::*;
//pub use category_service::*;
//pub use user_service::*;
//pub use project_service::*;
//pub use status_service::*;
//pub use matrix_service::*;

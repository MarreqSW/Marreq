//! Common helpers shared across service modules.
//!
//! The refactor towards lightweight service structs that operate on the
//! [`AppState`](crate::app::AppState) removed the need for a large base service
//! abstraction. A handful of utility functions are still reused by several
//! services, so they live in this module.

use crate::errors::{ApiError, ApiResult};
use crate::models::User;

/// Serialize a value into JSON so it can be stored in the audit log.
pub fn serialize_for_logging<T>(data: &T) -> ApiResult<String>
where
    T: serde::Serialize,
{
    serde_json::to_string(data).map_err(ApiError::Serialization)
}

/// Ensure the provided user may access the requested project.
pub fn check_project_permission(user: &User, _project_id: i32) -> ApiResult<()> {
    if user.is_admin {
        return Ok(());
    }

    // Project level permissions are not implemented yet.  Non admin users are
    // currently allowed to view all projects, which matches the behaviour prior
    // to the service refactor.
    Ok(())
}

/// Validate that a user may access an entity belonging to `entity_project_id`.
pub fn validate_entity_access(user: &User, entity_project_id: i32) -> ApiResult<()> {
    check_project_permission(user, entity_project_id)
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Common helpers shared across service modules.
//!
//! The refactor towards lightweight service structs that operate on the
//! [`AppState`](crate::app::AppState) removed the need for a large base service
//! abstraction. A handful of utility functions are still reused by several
//! services, so they live in this module.

use crate::api::error::{ApiError, ApiResult};
use crate::models::User;
use crate::permissions::{has_permission, Permission};
use crate::repository::ProjectMembersRepository;

/// Serialize a value into JSON so it can be stored in the audit log.
pub fn serialize_for_logging<T>(data: &T) -> ApiResult<String>
where
    T: serde::Serialize,
{
    serde_json::to_string(data).map_err(ApiError::from)
}

/// Ensure the provided user has the given permission in the project. Fail-closed.
pub fn check_project_permission<R>(
    repo: &R,
    user: &User,
    project_id: i32,
    permission: Permission,
) -> ApiResult<()>
where
    R: ProjectMembersRepository,
{
    if has_permission(repo, user, project_id, permission) {
        Ok(())
    } else {
        Err(ApiError::Forbidden("permission denied".into()))
    }
}

/// Validate that a user may access an entity belonging to `entity_project_id` (at least view).
pub fn validate_entity_access<R>(repo: &R, user: &User, entity_project_id: i32) -> ApiResult<()>
where
    R: ProjectMembersRepository,
{
    check_project_permission(repo, user, entity_project_id, Permission::ViewRequirements)
}

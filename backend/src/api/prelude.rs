// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

pub use rocket::http::Status;
pub use rocket::serde::json::{json, Json, Value};
pub use rocket::State;

pub use crate::api::error::{ApiError, ApiResult};
pub use crate::app::AppState;
pub use crate::auth::guards::AdminOnly;
pub use crate::auth::guards::ApiUser;
pub use crate::permissions::Permission;
pub use crate::repository::RepoLockExt;

/// Require the user to have the given project permission; returns `Err(ApiError::Forbidden)` otherwise.
pub fn require_project_permission(
    state: &State<AppState>,
    user: &crate::models::User,
    project_id: i32,
    permission: Permission,
) -> ApiResult<()> {
    let repo = state.repo_read();
    if crate::permissions::has_permission(&*repo, user, project_id, permission) {
        Ok(())
    } else {
        Err(ApiError::Forbidden("permission denied".into()))
    }
}

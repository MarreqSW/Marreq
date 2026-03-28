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
pub use crate::repository::ProjectReviewersRepository;
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

/// Require the user to be a designated project reviewer (or global admin). Fails if the project has
/// no reviewers configured (except for admins).
pub fn require_project_reviewer(
    state: &State<AppState>,
    user: &crate::models::User,
    project_id: i32,
) -> ApiResult<()> {
    if user.is_admin {
        return Ok(());
    }
    let repo = state.repo_read();
    let reviewer_ids = repo
        .list_project_reviewer_ids(project_id)
        .map_err(ApiError::from)?;
    if reviewer_ids.is_empty() {
        return Err(ApiError::Forbidden(
            "no project reviewers configured; add reviewers in project settings".into(),
        ));
    }
    let ok = repo
        .is_project_reviewer(project_id, user.id)
        .map_err(ApiError::from)?;
    if !ok {
        return Err(ApiError::Forbidden(
            "only designated project reviewers can perform this action".into(),
        ));
    }
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::repository::LookupRepository;

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

fn author_default_requirement_status_id(
    statuses: &[crate::models::RequirementStatus],
) -> Option<i32> {
    if statuses.is_empty() {
        return None;
    }
    statuses
        .iter()
        .find(|s| s.tag.eq_ignore_ascii_case("draft"))
        .map(|s| s.id)
        .or_else(|| statuses.iter().map(|s| s.id).min())
}

/// On create, authors may pick only the draft-like (or lexicographically first) requirement status without being a reviewer.
pub fn require_project_reviewer_unless_requirement_create_status_is_draft_like(
    state: &State<AppState>,
    user: &crate::models::User,
    project_id: i32,
    status_id: i32,
) -> ApiResult<()> {
    let repo = state.repo_read();
    let statuses = repo
        .get_requirement_status_by_project(project_id)
        .map_err(ApiError::from)?;
    let allowed = author_default_requirement_status_id(&statuses).is_some_and(|id| id == status_id);
    if allowed {
        Ok(())
    } else {
        require_project_reviewer(state, user, project_id)
    }
}

fn initial_verification_status_id(statuses: &[crate::models::VerificationStatus]) -> Option<i32> {
    if statuses.is_empty() {
        return None;
    }
    statuses
        .iter()
        .find(|s| s.tag.eq_ignore_ascii_case("nr"))
        .map(|s| s.id)
        .or_else(|| statuses.iter().map(|s| s.id).min())
}

/// On create, authors may pick only the initial (e.g. Not Run / minimum id) verification status without being a reviewer.
pub fn require_project_reviewer_unless_verification_create_status_is_initial(
    state: &State<AppState>,
    user: &crate::models::User,
    project_id: i32,
    status_id: i32,
) -> ApiResult<()> {
    let repo = state.repo_read();
    let statuses = repo
        .get_verification_status_by_project(project_id)
        .map_err(ApiError::from)?;
    let allowed = initial_verification_status_id(&statuses).is_some_and(|id| id == status_id);
    if allowed {
        Ok(())
    } else {
        require_project_reviewer(state, user, project_id)
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

pub use rocket::http::Status;
pub use rocket::serde::json::{json, Json, Value};
pub use rocket::State;

pub use crate::api::error::{ApiError, ApiResult};
pub use crate::app::AppState;
pub use crate::auth::guards::AdminOnly;
pub use crate::auth::guards::ApiUser;
pub use crate::permissions::{GroupPermission, Permission};
pub use crate::repository::ProjectReviewersRepository;
pub use crate::repository::RepoLockExt;

pub fn require_project_permission(
    state: &State<AppState>,
    user: &crate::models::User,
    project_id: i32,
    permission: Permission,
) -> ApiResult<()> {
    let repo = state.repo_read();
    crate::authorization::require_project_permission(&*repo, user, project_id, permission)
}

pub fn require_project_reviewer(
    state: &State<AppState>,
    user: &crate::models::User,
    project_id: i32,
) -> ApiResult<()> {
    let repo = state.repo_read();
    crate::authorization::require_project_reviewer(&*repo, user, project_id)
}

pub fn require_project_reviewer_unless_requirement_create_status_is_draft_like(
    state: &State<AppState>,
    user: &crate::models::User,
    project_id: i32,
    status_id: i32,
) -> ApiResult<()> {
    let repo = state.repo_read();
    crate::authorization::require_project_reviewer_unless_requirement_create_status_is_draft_like(
        &*repo, user, project_id, status_id,
    )
}

pub fn require_project_reviewer_unless_verification_create_status_is_initial(
    state: &State<AppState>,
    user: &crate::models::User,
    project_id: i32,
    status_id: i32,
) -> ApiResult<()> {
    let repo = state.repo_read();
    crate::authorization::require_project_reviewer_unless_verification_create_status_is_initial(
        &*repo, user, project_id, status_id,
    )
}

pub fn require_group_permission(
    state: &State<AppState>,
    user: &crate::models::User,
    group_id: i32,
    permission: GroupPermission,
) -> ApiResult<()> {
    let repo = state.repo_read();
    crate::authorization::require_group_permission(&*repo, user, group_id, permission)
}

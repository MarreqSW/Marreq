// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! REST API for project members and effective permissions.

use rocket::serde::Deserialize;
use rocket::State;

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::models::NewProjectMember;
use crate::permissions::{effective_permissions, role_label, EffectivePermissions};
use crate::repository::{
    ProjectMembersRepository, ProjectReviewersRepository, ProjectsRepository, UserRepository,
};

/// Response for one project member (id is user_id).
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct MemberResponse {
    pub user_id: i32,
    pub role: i32,
    pub role_label: &'static str,
    pub username: String,
    pub name: String,
}

/// GET /api/projects/<project_id>/me/permissions — effective permissions for the current user.
#[get("/projects/<project_id>/me/permissions")]
pub async fn get_my_permissions(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<EffectivePermissions>> {
    let repo = state.repo_read();
    let perms = effective_permissions(&*repo, access.user(), project_id);
    Ok(Json(perms))
}

/// GET /api/projects/<project_id>/members — list project members. Requires ViewRequirements.
#[get("/projects/<project_id>/members")]
pub async fn list_members(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<MemberResponse>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let repo = state.repo_read();
    let _ = repo.get_project_by_id(project_id).map_err(ApiError::from)?;
    let members = repo
        .get_members_by_project(project_id)
        .map_err(ApiError::from)?;
    let out: Vec<MemberResponse> = members
        .into_iter()
        .map(|m| {
            let user = repo.get_user_by_id(m.user_id).ok();
            MemberResponse {
                user_id: m.user_id,
                role: m.role,
                role_label: role_label(m.role),
                username: user
                    .as_ref()
                    .map(|u| u.username.clone())
                    .unwrap_or_default(),
                name: user.as_ref().map(|u| u.name.clone()).unwrap_or_default(),
            }
        })
        .collect();
    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct SetRoleRequest {
    pub role: i32,
}

/// PUT /api/projects/<project_id>/members/<user_id> — assign or update role. Requires ManageProjectMembers.
#[put("/projects/<project_id>/members/<user_id>", data = "<body>")]
pub async fn set_member_role(
    access: ProjectAccessOrBearer,
    project_id: i32,
    user_id: i32,
    state: &State<AppState>,
    body: Json<SetRoleRequest>,
) -> ApiResult<Json<MemberResponse>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ManageProjectMembers,
    )?;
    let role = body.into_inner().role;
    if !(1..=4).contains(&role) {
        return Err(ApiError::BadRequest(
            "role must be 1 (Admin), 2 (Reviewer), 3 (Author), or 4 (Viewer)".into(),
        ));
    }
    let mut repo = state.repo_write();
    let user = repo.get_user_by_id(user_id).map_err(ApiError::from)?;
    let _project = repo.get_project_by_id(project_id).map_err(ApiError::from)?;
    repo.add_project_member(&NewProjectMember {
        project_id,
        user_id,
        role,
    })?;
    Ok(Json(MemberResponse {
        user_id,
        role,
        role_label: role_label(role),
        username: user.username,
        name: user.name,
    }))
}

/// DELETE /api/projects/<project_id>/members/<user_id> — remove member. Requires ManageProjectMembers.
#[delete("/projects/<project_id>/members/<user_id>")]
pub async fn remove_member(
    access: ProjectAccessOrBearer,
    project_id: i32,
    user_id: i32,
    state: &State<AppState>,
) -> ApiResult<Status> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ManageProjectMembers,
    )?;
    let members = state
        .repo_read()
        .get_members_by_project(project_id)
        .map_err(ApiError::from)?;
    let admin_count = members.iter().filter(|m| m.role == 1).count();
    let target = members.iter().find(|m| m.user_id == user_id);
    if let Some(m) = target {
        if m.role == 1 && admin_count <= 1 {
            return Err(ApiError::BadRequest(
                "cannot remove the last Admin; assign another Admin first".into(),
            ));
        }
    } else {
        return Err(ApiError::NotFound("member not in project".into()));
    }
    state
        .repo_write()
        .remove_project_member(project_id, user_id)
        .map_err(ApiError::from)?;
    Ok(Status::NoContent)
}

/// GET /api/projects/<project_id>/reviewers — user ids designated as project reviewers.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ProjectReviewersResponse {
    pub user_ids: Vec<i32>,
}

#[get("/projects/<project_id>/reviewers")]
pub async fn list_project_reviewers(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<ProjectReviewersResponse>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let repo = state.repo_read();
    let _ = repo.get_project_by_id(project_id).map_err(ApiError::from)?;
    let user_ids = repo
        .list_project_reviewer_ids(project_id)
        .map_err(ApiError::from)?;
    Ok(Json(ProjectReviewersResponse { user_ids }))
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct PutProjectReviewersRequest {
    pub user_ids: Vec<i32>,
}

/// PUT /api/projects/<project_id>/reviewers — replace reviewer list (members only). Requires ManageProjectMembers.
#[put("/projects/<project_id>/reviewers", data = "<body>")]
pub async fn put_project_reviewers(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
    body: Json<PutProjectReviewersRequest>,
) -> ApiResult<Json<ProjectReviewersResponse>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ManageProjectMembers,
    )?;
    let user_ids = body.into_inner().user_ids;
    state
        .repo_write()
        .replace_project_reviewers(project_id, &user_ids)
        .map_err(ApiError::from)?;
    Ok(Json(ProjectReviewersResponse { user_ids }))
}

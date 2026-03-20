// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! REST API for groups and group members.

use rocket::serde::Deserialize;
use rocket::State;

use crate::api::prelude::*;
use crate::auth::guards::ApiUserOrBearer;
use crate::models::{NewGroup, NewGroupMember, UpdateGroup};
use crate::permissions::{
    group_role_label, has_group_permission, GroupPermission,
};
use crate::repository::{GroupMembersRepository, GroupsRepository, UserRepository};
use crate::services::GroupService;

/// Response for one group.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GroupResponse {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub owner_id: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<crate::models::Group> for GroupResponse {
    fn from(g: crate::models::Group) -> Self {
        Self {
            id: g.id,
            name: g.name,
            slug: g.slug,
            description: g.description,
            owner_id: g.owner_id,
            created_at: g.created_at.to_string(),
            updated_at: g.updated_at.to_string(),
        }
    }
}

/// Response for one group member.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GroupMemberResponse {
    pub user_id: i32,
    pub role: i32,
    pub role_label: &'static str,
}

/// Helper: require the user to have the given group permission.
fn require_group_permission(
    state: &State<AppState>,
    user: &crate::models::User,
    group_id: i32,
    permission: GroupPermission,
) -> ApiResult<()> {
    let repo = state.repo_read();
    if has_group_permission(&*repo, user, group_id, permission) {
        Ok(())
    } else {
        Err(ApiError::Forbidden("permission denied".into()))
    }
}

// ── Group CRUD ──────────────────────────────────────────────────

/// GET /api/groups — list all groups (admin) or groups the user is a member of.
#[get("/groups")]
pub async fn list(auth: ApiUserOrBearer, state: &State<AppState>) -> ApiResult<Json<Vec<GroupResponse>>> {
    let user = auth.user();
    let service = GroupService::new(state);
    let groups = if user.is_admin {
        service.list_all().map_err(ApiError::from)?
    } else {
        service.get_by_user_id(user.id).map_err(ApiError::from)?
    };
    Ok(Json(groups.into_iter().map(GroupResponse::from).collect()))
}

/// GET /api/groups/<group_id> — get group details.
#[get("/groups/<group_id>")]
pub async fn get(
    auth: ApiUserOrBearer,
    group_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<GroupResponse>> {
    require_group_permission(state, auth.user(), group_id, GroupPermission::ViewGroup)?;
    let service = GroupService::new(state);
    let group = service.get_by_id(group_id).map_err(ApiError::from)?;
    Ok(Json(GroupResponse::from(group)))
}

/// POST /api/groups — create a new group (any authenticated user).
#[post("/groups", data = "<body>")]
pub async fn create(
    auth: ApiUserOrBearer,
    state: &State<AppState>,
    body: Json<NewGroup>,
) -> ApiResult<Json<GroupResponse>> {
    let service = GroupService::new(state);
    let id = service
        .create(auth.user(), body.into_inner())
        .map_err(ApiError::from)?;
    let group = service.get_by_id(id).map_err(ApiError::from)?;
    Ok(Json(GroupResponse::from(group)))
}

/// Payload for updating a group.
#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct UpdateGroupRequest {
    pub name: String,
    pub description: Option<String>,
}

/// PATCH /api/groups/<group_id> — update a group. Requires ManageGroupMembers (owner-level).
#[patch("/groups/<group_id>", data = "<body>")]
pub async fn update(
    auth: ApiUserOrBearer,
    group_id: i32,
    state: &State<AppState>,
    body: Json<UpdateGroupRequest>,
) -> ApiResult<Json<GroupResponse>> {
    require_group_permission(
        state,
        auth.user(),
        group_id,
        GroupPermission::ManageGroupMembers,
    )?;
    let service = GroupService::new(state);
    let existing = service.get_by_id(group_id).map_err(ApiError::from)?;
    let payload = UpdateGroup {
        name: body.name.clone(),
        description: body.description.clone(),
        owner_id: existing.owner_id,
    };
    let group = service
        .update(auth.user(), group_id, payload)
        .map_err(ApiError::from)?;
    Ok(Json(GroupResponse::from(group)))
}

/// DELETE /api/groups/<group_id> — delete a group. Requires ManageGroupMembers (owner-level).
#[delete("/groups/<group_id>")]
pub async fn delete(
    auth: ApiUserOrBearer,
    group_id: i32,
    state: &State<AppState>,
) -> ApiResult<Status> {
    require_group_permission(
        state,
        auth.user(),
        group_id,
        GroupPermission::ManageGroupMembers,
    )?;
    let service = GroupService::new(state);
    service.delete(auth.user(), group_id).map_err(ApiError::from)?;
    Ok(Status::NoContent)
}

// ── Group projects ──────────────────────────────────────────────

/// GET /api/groups/<group_id>/projects — list projects in a group.
#[get("/groups/<group_id>/projects")]
pub async fn list_projects(
    auth: ApiUserOrBearer,
    group_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<crate::models::Project>>> {
    require_group_permission(state, auth.user(), group_id, GroupPermission::ViewGroup)?;
    let repo = state.repo_read();
    let projects = repo.get_projects_by_group(group_id).map_err(ApiError::from)?;
    Ok(Json(projects))
}

// ── Group members ───────────────────────────────────────────────

/// GET /api/groups/<group_id>/members — list group members.
#[get("/groups/<group_id>/members")]
pub async fn list_members(
    auth: ApiUserOrBearer,
    group_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<GroupMemberResponse>>> {
    require_group_permission(state, auth.user(), group_id, GroupPermission::ViewGroup)?;
    let repo = state.repo_read();
    let members = repo.get_members_by_group(group_id).map_err(ApiError::from)?;
    let out: Vec<GroupMemberResponse> = members
        .into_iter()
        .map(|m| GroupMemberResponse {
            user_id: m.user_id,
            role: m.role,
            role_label: group_role_label(m.role),
        })
        .collect();
    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct SetGroupRoleRequest {
    pub role: i32,
}

/// PUT /api/groups/<group_id>/members/<user_id> — assign or update group member role.
#[put("/groups/<group_id>/members/<user_id>", data = "<body>")]
pub async fn set_member_role(
    auth: ApiUserOrBearer,
    group_id: i32,
    user_id: i32,
    state: &State<AppState>,
    body: Json<SetGroupRoleRequest>,
) -> ApiResult<Json<GroupMemberResponse>> {
    require_group_permission(
        state,
        auth.user(),
        group_id,
        GroupPermission::ManageGroupMembers,
    )?;
    let role = body.into_inner().role;
    if !(1..=4).contains(&role) {
        return Err(ApiError::BadRequest(
            "role must be 1 (Owner), 2 (Maintainer), 3 (Contributor), or 4 (Viewer)".into(),
        ));
    }
    let mut repo = state.repo_write();
    let _user = repo.get_user_by_id(user_id).map_err(ApiError::from)?;
    let _group = repo.get_group_by_id(group_id).map_err(ApiError::from)?;
    repo.add_group_member(&NewGroupMember {
        group_id,
        user_id,
        role,
    })?;
    Ok(Json(GroupMemberResponse {
        user_id,
        role,
        role_label: group_role_label(role),
    }))
}

/// DELETE /api/groups/<group_id>/members/<user_id> — remove member from group.
#[delete("/groups/<group_id>/members/<user_id>")]
pub async fn remove_member(
    auth: ApiUserOrBearer,
    group_id: i32,
    user_id: i32,
    state: &State<AppState>,
) -> ApiResult<Status> {
    require_group_permission(
        state,
        auth.user(),
        group_id,
        GroupPermission::ManageGroupMembers,
    )?;
    let members = state
        .repo_read()
        .get_members_by_group(group_id)
        .map_err(ApiError::from)?;
    let owner_count = members.iter().filter(|m| m.role == 1).count();
    let target = members.iter().find(|m| m.user_id == user_id);
    if let Some(m) = target {
        if m.role == 1 && owner_count <= 1 {
            return Err(ApiError::BadRequest(
                "cannot remove the last Owner; assign another Owner first".into(),
            ));
        }
    } else {
        return Err(ApiError::NotFound("member not in group".into()));
    }
    state
        .repo_write()
        .remove_group_member(group_id, user_id)
        .map_err(ApiError::from)?;
    Ok(Status::NoContent)
}

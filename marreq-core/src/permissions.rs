// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Project-level permissions and role semantics.
//!
//! Roles (stored as `project_members.role`): 1 = Admin, 2 = Reviewer, 3 = Author, 4 = Viewer.
//! All checks are fail-closed: no membership, error, or unknown role → denied.

use crate::models::User;
use crate::repository::{
    GroupMembersRepository, ProjectMembersRepository, ProjectReviewersRepository,
};
use std::collections::BTreeSet;

/// Fine-grained permission for project-scoped actions.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Permission {
    ViewRequirements,
    EditRequirements,
    ApproveVersions,
    ManageCustomFields,
    ManageProjectMembers,
}

/// Role id as stored in `project_members.role`. 1 = Admin, 2 = Reviewer, 3 = Author, 4 = Viewer.
pub const ROLE_ADMIN: i32 = 1;
pub const ROLE_REVIEWER: i32 = 2;
pub const ROLE_AUTHOR: i32 = 3;
pub const ROLE_VIEWER: i32 = 4;

/// Human-readable label for a role id.
pub fn role_label(role: i32) -> &'static str {
    match role {
        ROLE_ADMIN => "Admin",
        ROLE_REVIEWER => "Reviewer",
        ROLE_AUTHOR => "Author",
        ROLE_VIEWER => "Viewer",
        _ => "Member",
    }
}

/// Permissions granted by a role. Unknown role returns empty set (fail-closed).
fn permissions_for_role(role: i32) -> BTreeSet<Permission> {
    use Permission::*;
    match role {
        ROLE_ADMIN => [
            ViewRequirements,
            EditRequirements,
            ApproveVersions,
            ManageCustomFields,
            ManageProjectMembers,
        ]
        .into_iter()
        .collect(),
        ROLE_REVIEWER => [ViewRequirements, EditRequirements, ApproveVersions]
            .into_iter()
            .collect(),
        ROLE_AUTHOR => [ViewRequirements, EditRequirements].into_iter().collect(),
        ROLE_VIEWER => [ViewRequirements].into_iter().collect(),
        _ => BTreeSet::new(),
    }
}

/// Effective permissions for a user in a project (for API response). Fail-closed.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub struct EffectivePermissions {
    pub view_requirements: bool,
    pub edit_requirements: bool,
    pub approve_versions: bool,
    /// True when the user may change requirement / verification status and version approval for this project.
    pub is_project_reviewer: bool,
    pub manage_custom_fields: bool,
    pub manage_project_members: bool,
}

fn user_is_project_reviewer<R>(repo: &R, user: &User, project_id: i32) -> bool
where
    R: ProjectReviewersRepository,
{
    let Ok(ids) = repo.list_project_reviewer_ids(project_id) else {
        return false;
    };
    if ids.is_empty() {
        // No explicit pool: only site admins (same rule as `require_project_reviewer`).
        return user.is_admin;
    }
    repo.is_project_reviewer(project_id, user.id)
        .unwrap_or(false)
}

/// Whether the user may change requirement/verification status and version approval
/// (member of the project's reviewer list, or site admin when that list is still empty).
pub fn may_change_review_gates<R>(repo: &R, user: &User, project_id: i32) -> bool
where
    R: ProjectReviewersRepository,
{
    user_is_project_reviewer(repo, user, project_id)
}

/// Compute effective permissions for a user in a project.
pub fn effective_permissions<R>(repo: &R, user: &User, project_id: i32) -> EffectivePermissions
where
    R: ProjectMembersRepository + ProjectReviewersRepository,
{
    use Permission::*;
    let is_project_reviewer = user_is_project_reviewer(repo, user, project_id);
    EffectivePermissions {
        view_requirements: has_permission(repo, user, project_id, ViewRequirements),
        edit_requirements: has_permission(repo, user, project_id, EditRequirements),
        approve_versions: has_permission(repo, user, project_id, ApproveVersions),
        is_project_reviewer,
        manage_custom_fields: has_permission(repo, user, project_id, ManageCustomFields),
        manage_project_members: has_permission(repo, user, project_id, ManageProjectMembers),
    }
}

/// Returns true only if the user has the given permission in the project. Fail-closed.
pub fn has_permission<R>(repo: &R, user: &User, project_id: i32, permission: Permission) -> bool
where
    R: ProjectMembersRepository,
{
    if user.is_admin {
        return true;
    }
    let memberships = match repo.get_projects_for_user(user.id) {
        Ok(m) => m,
        Err(_) => return false,
    };
    let membership = match memberships.iter().find(|m| m.project_id == project_id) {
        Some(m) => m,
        None => return false,
    };
    permissions_for_role(membership.role).contains(&permission)
}

// ── Group-level permissions ─────────────────────────────────────

/// Fine-grained permission for group-scoped actions.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GroupPermission {
    ViewGroup,
    ManageProjects,
    ManageGroupMembers,
}

/// Group role id as stored in `group_members.role`.
/// 1 = Owner, 2 = Maintainer, 3 = Contributor, 4 = Viewer
pub const GROUP_ROLE_OWNER: i32 = 1;
pub const GROUP_ROLE_MAINTAINER: i32 = 2;
pub const GROUP_ROLE_CONTRIBUTOR: i32 = 3;
pub const GROUP_ROLE_VIEWER: i32 = 4;

/// Human-readable label for a group role id.
pub fn group_role_label(role: i32) -> &'static str {
    match role {
        GROUP_ROLE_OWNER => "Owner",
        GROUP_ROLE_MAINTAINER => "Maintainer",
        GROUP_ROLE_CONTRIBUTOR => "Contributor",
        GROUP_ROLE_VIEWER => "Viewer",
        _ => "Member",
    }
}

/// Permissions granted by a group role. Unknown role returns empty set (fail-closed).
fn group_permissions_for_role(role: i32) -> BTreeSet<GroupPermission> {
    use GroupPermission::*;
    match role {
        GROUP_ROLE_OWNER => [ViewGroup, ManageProjects, ManageGroupMembers]
            .into_iter()
            .collect(),
        GROUP_ROLE_MAINTAINER => [ViewGroup, ManageProjects].into_iter().collect(),
        GROUP_ROLE_CONTRIBUTOR => [ViewGroup].into_iter().collect(),
        GROUP_ROLE_VIEWER => [ViewGroup].into_iter().collect(),
        _ => BTreeSet::new(),
    }
}

/// Returns true only if the user has the given permission in the group. Fail-closed.
pub fn has_group_permission<R>(
    repo: &R,
    user: &User,
    group_id: i32,
    permission: GroupPermission,
) -> bool
where
    R: GroupMembersRepository,
{
    if user.is_admin {
        return true;
    }
    let memberships = match repo.get_groups_for_user(user.id) {
        Ok(m) => m,
        Err(_) => return false,
    };
    let membership = match memberships.iter().find(|m| m.group_id == group_id) {
        Some(m) => m,
        None => return false,
    };
    group_permissions_for_role(membership.role).contains(&permission)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_label_mapping() {
        assert_eq!(role_label(ROLE_ADMIN), "Admin");
        assert_eq!(role_label(ROLE_REVIEWER), "Reviewer");
        assert_eq!(role_label(ROLE_AUTHOR), "Author");
        assert_eq!(role_label(ROLE_VIEWER), "Viewer");
        assert_eq!(role_label(99), "Member");
    }

    #[test]
    fn admin_has_all_permissions() {
        let perms = permissions_for_role(ROLE_ADMIN);
        assert!(perms.contains(&Permission::ViewRequirements));
        assert!(perms.contains(&Permission::EditRequirements));
        assert!(perms.contains(&Permission::ApproveVersions));
        assert!(perms.contains(&Permission::ManageCustomFields));
        assert!(perms.contains(&Permission::ManageProjectMembers));
        assert_eq!(perms.len(), 5);
    }

    #[test]
    fn reviewer_has_view_edit_approve() {
        let perms = permissions_for_role(ROLE_REVIEWER);
        assert!(perms.contains(&Permission::ViewRequirements));
        assert!(perms.contains(&Permission::EditRequirements));
        assert!(perms.contains(&Permission::ApproveVersions));
        assert!(!perms.contains(&Permission::ManageCustomFields));
        assert!(!perms.contains(&Permission::ManageProjectMembers));
    }

    #[test]
    fn author_has_view_edit_only() {
        let perms = permissions_for_role(ROLE_AUTHOR);
        assert!(perms.contains(&Permission::ViewRequirements));
        assert!(perms.contains(&Permission::EditRequirements));
        assert!(!perms.contains(&Permission::ApproveVersions));
        assert!(!perms.contains(&Permission::ManageCustomFields));
        assert!(!perms.contains(&Permission::ManageProjectMembers));
    }

    #[test]
    fn viewer_has_view_only() {
        let perms = permissions_for_role(ROLE_VIEWER);
        assert!(perms.contains(&Permission::ViewRequirements));
        assert!(!perms.contains(&Permission::EditRequirements));
        assert!(!perms.contains(&Permission::ApproveVersions));
        assert!(!perms.contains(&Permission::ManageCustomFields));
        assert!(!perms.contains(&Permission::ManageProjectMembers));
    }

    #[test]
    fn unknown_role_has_no_permissions() {
        let perms = permissions_for_role(0);
        assert!(perms.is_empty());
        let perms = permissions_for_role(99);
        assert!(perms.is_empty());
    }
}

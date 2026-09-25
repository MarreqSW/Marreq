// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Canonical authorization helpers shared by API and service layers.
//!
//! Keep project and group permission decisions here so route handlers and
//! services do not grow divergent, duplicated authorization semantics.

use crate::models::{RequirementStatus, User, VerificationStatus};
use crate::permissions::{has_group_permission, has_permission, GroupPermission, Permission};
use crate::repository::{
    GroupMembersRepository, LookupRepository, ProjectMembersRepository, ProjectReviewersRepository,
};

#[derive(Debug, thiserror::Error)]
pub enum AuthorizationError {
    #[error("permission denied")]
    Forbidden,
    #[error("authorization repository error: {0}")]
    Repository(#[from] crate::repository::errors::RepoError),
}

pub type AuthorizationResult<T> = Result<T, AuthorizationError>;

/// Require the user to have the given project permission. Fail-closed.
pub fn require_project_permission<R>(
    repo: &R,
    user: &User,
    project_id: i32,
    permission: Permission,
) -> AuthorizationResult<()>
where
    R: ProjectMembersRepository,
{
    if has_permission(repo, user, project_id, permission) {
        Ok(())
    } else {
        Err(AuthorizationError::Forbidden)
    }
}

/// Validate that a user may access an entity belonging to `entity_project_id`.
pub fn validate_entity_access<R>(
    repo: &R,
    user: &User,
    entity_project_id: i32,
) -> AuthorizationResult<()>
where
    R: ProjectMembersRepository,
{
    require_project_permission(repo, user, entity_project_id, Permission::ViewRequirements)
}

/// Require the user to be a designated project reviewer, or a site admin when
/// the project has no explicit reviewer pool yet.
///
/// `project_reviewers` is the sole authorization source for status and approval
/// gates. Role capabilities such as `Permission::ApproveVersions` do not grant
/// these transitions by themselves.
pub fn require_project_reviewer<R>(
    repo: &R,
    user: &User,
    project_id: i32,
) -> AuthorizationResult<()>
where
    R: ProjectMembersRepository + ProjectReviewersRepository,
{
    let reviewer_ids = repo.list_project_reviewer_ids(project_id)?;
    if reviewer_ids.is_empty() {
        if user.is_admin {
            return Ok(());
        }
        return Err(AuthorizationError::Forbidden);
    }
    let ok = repo.is_project_reviewer(project_id, user.id)?;
    if !ok {
        return Err(AuthorizationError::Forbidden);
    }
    Ok(())
}

fn author_default_requirement_status_id(statuses: &[RequirementStatus]) -> Option<i32> {
    if statuses.is_empty() {
        return None;
    }
    statuses
        .iter()
        .find(|s| s.tag.eq_ignore_ascii_case("draft"))
        .map(|s| s.id)
        .or_else(|| statuses.iter().map(|s| s.id).min())
}

/// On create, authors may pick only the draft-like requirement status without
/// being a reviewer. Non-draft status selection requires reviewer authority.
pub fn require_project_reviewer_unless_requirement_create_status_is_draft_like<R>(
    repo: &R,
    user: &User,
    project_id: i32,
    status_id: i32,
) -> AuthorizationResult<()>
where
    R: LookupRepository + ProjectMembersRepository + ProjectReviewersRepository,
{
    let statuses = repo.get_requirement_status_by_project(project_id)?;
    let allowed = author_default_requirement_status_id(&statuses).is_some_and(|id| id == status_id);
    if allowed {
        Ok(())
    } else {
        require_project_reviewer(repo, user, project_id)
    }
}

fn initial_verification_status_id(statuses: &[VerificationStatus]) -> Option<i32> {
    if statuses.is_empty() {
        return None;
    }
    statuses
        .iter()
        .find(|s| s.tag.eq_ignore_ascii_case("nr"))
        .map(|s| s.id)
        .or_else(|| statuses.iter().map(|s| s.id).min())
}

/// On create, authors may pick only the initial verification status without
/// being a reviewer. Non-initial status selection requires reviewer authority.
pub fn require_project_reviewer_unless_verification_create_status_is_initial<R>(
    repo: &R,
    user: &User,
    project_id: i32,
    status_id: i32,
) -> AuthorizationResult<()>
where
    R: LookupRepository + ProjectMembersRepository + ProjectReviewersRepository,
{
    let statuses = repo.get_verification_status_by_project(project_id)?;
    let allowed = initial_verification_status_id(&statuses).is_some_and(|id| id == status_id);
    if allowed {
        Ok(())
    } else {
        require_project_reviewer(repo, user, project_id)
    }
}

/// Require the user to have the given group permission. Fail-closed.
pub fn require_group_permission<R>(
    repo: &R,
    user: &User,
    group_id: i32,
    permission: GroupPermission,
) -> AuthorizationResult<()>
where
    R: GroupMembersRepository,
{
    if has_group_permission(repo, user, group_id, permission) {
        Ok(())
    } else {
        Err(AuthorizationError::Forbidden)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GroupMember, ProjectMember};
    use crate::permissions::{GROUP_ROLE_OWNER, ROLE_VIEWER};
    use crate::repository::diesel_repo_mock::DieselRepoMock;

    fn user(id: i32, is_admin: bool) -> User {
        let mut user = DieselRepoMock::make_user(id, "tester", "");
        user.is_admin = is_admin;
        user
    }

    fn project_member(project_id: i32, user_id: i32, role: i32) -> ProjectMember {
        ProjectMember {
            project_id,
            user_id,
            role,
            created_at: chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            updated_at: chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        }
    }

    fn group_member(group_id: i32, user_id: i32, role: i32) -> GroupMember {
        GroupMember {
            group_id,
            user_id,
            role,
            created_at: chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            updated_at: chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        }
    }

    #[test]
    fn project_permission_allows_matching_role() {
        let user = user(7, false);
        let mut repo = DieselRepoMock::default();
        repo.project_members
            .push(project_member(10, 7, ROLE_VIEWER));

        assert!(require_project_permission(&repo, &user, 10, Permission::ViewRequirements).is_ok());
    }

    #[test]
    fn project_permission_denies_missing_membership() {
        let user = user(7, false);
        let repo = DieselRepoMock::default();

        assert!(matches!(
            require_project_permission(&repo, &user, 10, Permission::ViewRequirements),
            Err(AuthorizationError::Forbidden)
        ));
    }

    #[test]
    fn validate_entity_access_uses_view_permission() {
        let user = user(7, false);
        let mut repo = DieselRepoMock::default();
        repo.project_members
            .push(project_member(10, 7, ROLE_VIEWER));

        assert!(validate_entity_access(&repo, &user, 10).is_ok());
        assert!(matches!(
            validate_entity_access(&repo, &user, 11),
            Err(AuthorizationError::Forbidden)
        ));
    }

    #[test]
    fn project_reviewer_requires_explicit_reviewer_when_pool_exists() {
        let user = user(7, false);
        let mut repo = DieselRepoMock::default();
        repo.project_members
            .push(project_member(10, 7, crate::permissions::ROLE_REVIEWER));
        repo.project_reviewers.insert(10, vec![8]);

        assert!(matches!(
            require_project_reviewer(&repo, &user, 10),
            Err(AuthorizationError::Forbidden)
        ));

        repo.project_reviewers.insert(10, vec![7]);
        assert!(require_project_reviewer(&repo, &user, 10).is_ok());
    }

    #[test]
    fn author_in_reviewer_pool_may_change_review_gates() {
        let user = user(7, false);
        let mut repo = DieselRepoMock::default();
        repo.project_members
            .push(project_member(10, 7, crate::permissions::ROLE_AUTHOR));
        repo.project_reviewers.insert(10, vec![7]);

        assert!(require_project_reviewer(&repo, &user, 10).is_ok());
    }

    #[test]
    fn reviewer_role_outside_pool_cannot_change_review_gates() {
        let user = user(7, false);
        let mut repo = DieselRepoMock::default();
        repo.project_members
            .push(project_member(10, 7, crate::permissions::ROLE_REVIEWER));
        repo.project_reviewers.insert(10, vec![8]);

        assert!(matches!(
            require_project_reviewer(&repo, &user, 10),
            Err(AuthorizationError::Forbidden)
        ));
    }

    #[test]
    fn empty_reviewer_pool_allows_only_site_admin() {
        let repo = DieselRepoMock::default();

        assert!(require_project_reviewer(&repo, &user(1, true), 10).is_ok());
        assert!(matches!(
            require_project_reviewer(&repo, &user(7, false), 10),
            Err(AuthorizationError::Forbidden)
        ));
    }

    #[test]
    fn group_permission_allows_and_denies() {
        let user = user(7, false);
        let mut repo = DieselRepoMock::default();
        repo.group_members
            .push(group_member(20, 7, GROUP_ROLE_OWNER));

        assert!(
            require_group_permission(&repo, &user, 20, GroupPermission::ManageGroupMembers).is_ok()
        );
        assert!(matches!(
            require_group_permission(&repo, &user, 21, GroupPermission::ManageGroupMembers),
            Err(AuthorizationError::Forbidden)
        ));
    }
}

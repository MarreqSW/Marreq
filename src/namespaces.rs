// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Namespace helpers for user/group rooted HTML URLs.

use crate::errors::ValidationError;
use crate::models::{Group, Project, User};
use crate::repository::errors::RepoError;
use crate::repository::{GroupsRepository, UserRepository};

/// Top-level path segments that are reserved for system routes and cannot be used as namespaces.
pub const RESERVED_NAMESPACE_SEGMENTS: &[&str] = &[
    "admin",
    "api",
    "cache",
    "change_password",
    "cleanup_logs",
    "error",
    "export_logs",
    "groups",
    "log_analytics",
    "login",
    "logout",
    "logs",
    "new_project",
    "profile",
    "projects",
    "static",
    "status",
    "user",
];

#[derive(Debug, Clone)]
pub enum NamespaceEntity {
    User(User),
    Group(Group),
}

impl NamespaceEntity {
    pub fn segment(&self) -> &str {
        match self {
            Self::User(user) => &user.username,
            Self::Group(group) => &group.slug,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::User(user) => &user.name,
            Self::Group(group) => &group.name,
        }
    }

    pub fn is_group(&self) -> bool {
        matches!(self, Self::Group(_))
    }
}

pub fn normalize_namespace_segment(segment: &str) -> String {
    segment.trim().to_lowercase()
}

pub fn is_reserved_namespace_segment(segment: &str) -> bool {
    let normalized = normalize_namespace_segment(segment);
    RESERVED_NAMESPACE_SEGMENTS
        .iter()
        .any(|candidate| *candidate == normalized)
}

pub fn validate_namespace_segment(segment: &str, field: &str) -> Result<(), ValidationError> {
    if is_reserved_namespace_segment(segment) {
        return Err(ValidationError::InvalidFormat {
            field: field.to_string(),
            message: format!(
                "{} is reserved for a system route and cannot be used as a namespace",
                segment.trim()
            ),
        });
    }

    Ok(())
}

pub fn resolve_namespace_entity<R>(repo: &R, segment: &str) -> Result<NamespaceEntity, RepoError>
where
    R: UserRepository + GroupsRepository,
{
    if is_reserved_namespace_segment(segment) {
        return Err(RepoError::NotFound);
    }

    let normalized = normalize_namespace_segment(segment);
    let user = repo.get_user_by_username(&normalized)?;
    let group = match repo.get_group_by_slug(&normalized) {
        Ok(group) => Some(group),
        Err(RepoError::NotFound) => None,
        Err(error) => return Err(error),
    };

    match (user, group) {
        (Some(_), Some(_)) => Err(RepoError::BadInput(format!(
            "namespace '{normalized}' is ambiguous"
        ))),
        (Some(user), None) => Ok(NamespaceEntity::User(user)),
        (None, Some(group)) => Ok(NamespaceEntity::Group(group)),
        (None, None) => Err(RepoError::NotFound),
    }
}

pub fn project_namespace_segment<R>(repo: &R, project: &Project) -> Result<String, RepoError>
where
    R: UserRepository + GroupsRepository,
{
    if let Some(group_id) = project.group_id {
        return repo.get_group_by_id(group_id).map(|group| group.slug);
    }

    if let Some(owner_id) = project.owner_id {
        return repo.get_user_by_id(owner_id).map(|user| user.username);
    }

    Err(RepoError::BadInput(format!(
        "project {} has no user or group namespace",
        project.id
    )))
}

pub fn project_route_slug<R>(repo: &R, project: &Project) -> Result<String, RepoError>
where
    R: UserRepository + GroupsRepository,
{
    Ok(format!(
        "{}/{}",
        project_namespace_segment(repo, project)?,
        project.slug
    ))
}

pub fn project_base_path<R>(repo: &R, project: &Project) -> Result<String, RepoError>
where
    R: UserRepository + GroupsRepository,
{
    Ok(format!("/{}", project_route_slug(repo, project)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use chrono::{NaiveDate, NaiveDateTime};

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    #[test]
    fn reserved_namespace_segments_are_case_insensitive() {
        assert!(is_reserved_namespace_segment("Projects"));
        assert!(is_reserved_namespace_segment("LOGIN"));
        assert!(!is_reserved_namespace_segment("mission_team"));
    }

    #[test]
    fn resolve_namespace_entity_prefers_existing_kind() {
        let mut repo = DieselRepoMock::default();
        let user = DieselRepoMock::make_user(1, "alice", "");
        repo.users.insert(user.id, user);

        let group = Group {
            id: 2,
            name: "Flight Systems".into(),
            slug: "flight-systems".into(),
            description: None,
            owner_id: Some(1),
            created_at: timestamp(),
            updated_at: timestamp(),
        };
        repo.groups.insert(group.id, group);

        let user_namespace = resolve_namespace_entity(&repo, "ALICE").unwrap();
        assert!(matches!(user_namespace, NamespaceEntity::User(_)));

        let group_namespace = resolve_namespace_entity(&repo, "flight-systems").unwrap();
        assert!(matches!(group_namespace, NamespaceEntity::Group(_)));
    }
}

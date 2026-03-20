// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Service handling group-level operations.

use crate::app::{AppState, DieselCachedRepo};
use crate::helper_functions::utils::slugify_project_name;
use crate::logger::{LogCtx, Logger};
use crate::models::{Group, GroupMember, NewGroup, NewGroupMember, NewGroupRow, UpdateGroup, User};
use crate::repository::errors::RepoError;
use crate::repository::{
    GroupMembersRepository, GroupsRepository, PooledConnectionWrapper, UserRepository,
};
use crate::validation::{sanitize_optional_string, sanitize_string};

/// High-level group operations backed by the shared [`AppState`].
pub struct GroupService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> GroupService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve all groups.
    pub fn list_all(&self) -> Result<Vec<Group>, RepoError> {
        self.state.repo_read().get_groups_all()
    }

    /// Retrieve a group by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<Group, RepoError> {
        self.state.repo_read().get_group_by_id(id)
    }

    /// Retrieve a group by slug.
    pub fn get_by_slug(&self, slug: &str) -> Result<Group, RepoError> {
        self.state.repo_read().get_group_by_slug(slug)
    }

    /// Retrieve all groups that the specified user is a member of.
    pub fn get_by_user_id(&self, user_id: i32) -> Result<Vec<Group>, RepoError> {
        let repo = self.state.repo_read();
        let memberships = repo.get_groups_for_user(user_id)?;

        let mut groups = Vec::with_capacity(memberships.len());
        for membership in memberships {
            match repo.get_group_by_id(membership.group_id) {
                Ok(group) => groups.push(group),
                Err(RepoError::NotFound) => continue,
                Err(err) => return Err(err),
            }
        }

        groups.sort_by_key(|g| g.name.to_lowercase());
        Ok(groups)
    }

    /// Retrieve all members belonging to a group.
    pub fn list_members(&self, group_id: i32) -> Result<Vec<GroupMember>, RepoError> {
        self.state.repo_read().get_members_by_group(group_id)
    }

    /// Create a new group and log the action. The actor becomes the owner.
    pub fn create(&self, actor: &User, mut payload: NewGroup) -> Result<i32, RepoError> {
        if payload.owner_id.is_none() {
            payload.owner_id = Some(actor.id);
        }

        self.prepare_new_payload(&mut payload)?;
        let slug = self.generate_slug(&payload.name)?;

        let owner_id = payload.owner_id.unwrap_or(actor.id);
        let id = {
            let mut repo = self.state.repo_write();
            let id = repo.insert_new_group(&NewGroupRow {
                name: payload.name.clone(),
                slug,
                description: payload.description.clone(),
                owner_id: payload.owner_id,
            })?;
            // Add the owner as the first group member with Owner role (1)
            repo.add_group_member(&NewGroupMember {
                group_id: id,
                user_id: owner_id,
                role: 1, // Owner
            })?;
            id
        };

        if let Ok(group) = self.get_by_id(id) {
            self.log_created(actor, id, &group);
        }
        Ok(id)
    }

    /// Update an existing group entry and log the change.
    pub fn update(
        &self,
        actor: &User,
        id: i32,
        mut payload: UpdateGroup,
    ) -> Result<Group, RepoError> {
        let before = self.get_by_id(id)?;

        if payload.owner_id.is_none() {
            payload.owner_id = before.owner_id.or(Some(actor.id));
        }

        self.prepare_update_payload(&mut payload)?;

        {
            let mut repo = self.state.repo_write();
            let updated = repo.edit_group(id, &payload)?;
            if !updated {
                return Err(RepoError::NotFound);
            }
        }

        let after = self.get_by_id(id)?;
        self.log_updated(actor, &before, &after);
        Ok(after)
    }

    /// Delete a group entry and log the removal.
    pub fn delete(&self, actor: &User, id: i32) -> Result<Group, RepoError> {
        let removed = {
            let mut repo = self.state.repo_write();
            let projects = repo.get_projects_by_group(id)?;
            if !projects.is_empty() {
                return Err(RepoError::BadInput(
                    "cannot delete a group that still has projects attached; reassign or detach those projects first"
                        .into(),
                ));
            }
            repo.delete_group(id)?
        };

        self.log_deleted(actor, &removed);
        Ok(removed)
    }

    /// Assign or update a member role while preserving the requirement that a group always has at least one Owner.
    pub fn set_member_role(&self, group_id: i32, user_id: i32, role: i32) -> Result<(), RepoError> {
        if !(1..=4).contains(&role) {
            return Err(RepoError::BadInput(
                "role must be 1 (Owner), 2 (Maintainer), 3 (Contributor), or 4 (Viewer)".into(),
            ));
        }

        let mut repo = self.state.repo_write();
        let _group = repo.get_group_by_id(group_id)?;
        let _user = repo.get_user_by_id(user_id)?;
        let members = repo.get_members_by_group(group_id)?;

        let owner_count = members.iter().filter(|member| member.role == 1).count();
        let existing = members.iter().find(|member| member.user_id == user_id);

        if let Some(member) = existing {
            let demoting_last_owner = member.role == 1 && role != 1 && owner_count <= 1;
            if demoting_last_owner {
                return Err(RepoError::BadInput(
                    "cannot change the last Owner to a different role; assign another Owner first"
                        .into(),
                ));
            }

            repo.update_group_member_role(group_id, user_id, role)?;
        } else {
            repo.add_group_member(&NewGroupMember {
                group_id,
                user_id,
                role,
            })?;
        }

        Ok(())
    }

    /// Remove a group member while preserving the requirement that a group always has at least one Owner.
    pub fn remove_member(&self, group_id: i32, user_id: i32) -> Result<(), RepoError> {
        let mut repo = self.state.repo_write();
        let members = repo.get_members_by_group(group_id)?;
        let owner_count = members.iter().filter(|member| member.role == 1).count();
        let target = members.iter().find(|member| member.user_id == user_id);

        match target {
            Some(member) if member.role == 1 && owner_count <= 1 => Err(RepoError::BadInput(
                "cannot remove the last Owner; assign another Owner first".into(),
            )),
            Some(_) => repo.remove_group_member(group_id, user_id),
            None => Err(RepoError::NotFound),
        }
    }

    fn prepare_new_payload(&self, payload: &mut NewGroup) -> Result<(), RepoError> {
        sanitize_string(&mut payload.name);
        sanitize_optional_string(&mut payload.description);

        crate::validation::validate_group(payload)
            .map_err(|err| RepoError::BadInput(err.to_string()))
    }

    fn prepare_update_payload(&self, payload: &mut UpdateGroup) -> Result<(), RepoError> {
        sanitize_string(&mut payload.name);
        sanitize_optional_string(&mut payload.description);

        let clone = NewGroup {
            name: payload.name.clone(),
            description: payload.description.clone(),
            owner_id: payload.owner_id,
        };
        crate::validation::validate_group(&clone)
            .map_err(|err| RepoError::BadInput(err.to_string()))
    }

    fn generate_slug(&self, name: &str) -> Result<String, RepoError> {
        let existing = self
            .state
            .repo_read()
            .get_groups_all()?
            .into_iter()
            .map(|g| g.slug)
            .collect::<Vec<_>>();

        let base = slugify_project_name(name);
        let existing_set: std::collections::HashSet<String> = existing.into_iter().collect();

        if !existing_set.contains(&base) {
            return Ok(base);
        }

        let mut occurrence = 2;
        loop {
            let suffix = format!("-{occurrence}");
            let candidate = format!("{}{}", &base[..base.len().min(255 - suffix.len())], suffix);
            if !existing_set.contains(&candidate) {
                return Ok(candidate);
            }
            occurrence += 1;
        }
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_created(&self, actor: &User, id: i32, entity: &Group) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log group creation {id}: {_err}");
            }
        }
    }

    fn log_updated(&self, actor: &User, before: &Group, after: &Group) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::updated(conn.as_mut(), &ctx, before, after) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log group update {} -> {}: {_err}",
                    before.id, after.id
                );
            }
        }
    }

    fn log_deleted(&self, actor: &User, entity: &Group) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log group deletion {}: {_err}", entity.id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::GroupMember;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::sync::{Arc, RwLock};

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn state_with_repo(repo: DieselRepoMock) -> AppState<DieselCachedRepo> {
        AppState {
            repo: Arc::new(RwLock::new(DieselCachedRepo::new(repo, 0))),
        }
    }

    fn actor() -> User {
        DieselRepoMock::make_user(7, "actor", "")
    }

    fn group(id: i32, name: &str) -> Group {
        Group {
            id,
            name: name.into(),
            slug: name.to_lowercase().replace(' ', "-"),
            description: Some("A test group".into()),
            owner_id: Some(1),
            created_at: timestamp(),
            updated_at: timestamp(),
        }
    }

    #[test]
    fn create_trims_input() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = GroupService::new(&state);

        let payload = NewGroup {
            name: "  Avionics Systems  ".into(),
            description: Some("   ".into()),
            owner_id: Some(1),
        };

        let id = service.create(&actor(), payload).unwrap();
        let stored = service.get_by_id(id).unwrap();

        assert_eq!(stored.name, "Avionics Systems");
        assert_eq!(stored.description, None);
    }

    #[test]
    fn create_rejects_empty_name() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = GroupService::new(&state);

        let payload = NewGroup {
            name: " ".into(),
            description: None,
            owner_id: None,
        };

        let err = service.create(&actor(), payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn create_adds_owner_as_member() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = GroupService::new(&state);

        let payload = NewGroup {
            name: "Test Group".into(),
            description: None,
            owner_id: Some(7),
        };

        let id = service.create(&actor(), payload).unwrap();

        let members = state.repo_read().get_members_by_group(id).unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].user_id, 7);
        assert_eq!(members[0].role, 1);
    }

    #[test]
    fn update_persists_changes() {
        let mut repo = DieselRepoMock::default();
        repo.groups.insert(1, group(1, "Legacy"));
        let state = state_with_repo(repo);
        let service = GroupService::new(&state);

        let payload = UpdateGroup {
            name: "  Modernized  ".into(),
            description: Some("  Updated description  ".into()),
            owner_id: Some(2),
        };

        let updated = service.update(&actor(), 1, payload).unwrap();
        assert_eq!(updated.name, "Modernized");
        assert_eq!(updated.description.as_deref(), Some("Updated description"));
        assert_eq!(updated.owner_id, Some(2));
    }

    #[test]
    fn delete_removes_group() {
        let mut repo = DieselRepoMock::default();
        repo.groups.insert(4, group(4, "To remove"));
        let state = state_with_repo(repo);
        let service = GroupService::new(&state);

        let deleted = service.delete(&actor(), 4).unwrap();
        assert_eq!(deleted.id, 4);
        assert!(matches!(service.get_by_id(4), Err(RepoError::NotFound)));
    }

    #[test]
    fn list_all_returns_all_groups() {
        let mut repo = DieselRepoMock::default();
        repo.groups.insert(1, group(1, "A"));
        repo.groups.insert(2, group(2, "B"));
        let state = state_with_repo(repo);
        let service = GroupService::new(&state);

        let mut groups = service.list_all().unwrap();
        groups.sort_by_key(|g| g.id);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "A");
        assert_eq!(groups[1].name, "B");
    }

    #[test]
    fn get_by_user_id_returns_sorted_groups() {
        let mut repo = DieselRepoMock::default();
        repo.groups.insert(1, group(1, "Beta Group"));
        repo.groups.insert(2, group(2, "Alpha Group"));

        let now = timestamp();
        repo.group_members.push(GroupMember {
            group_id: 1,
            user_id: 42,
            role: 1,
            created_at: now,
            updated_at: now,
        });
        repo.group_members.push(GroupMember {
            group_id: 2,
            user_id: 42,
            role: 2,
            created_at: now,
            updated_at: now,
        });

        let state = state_with_repo(repo);
        let service = GroupService::new(&state);

        let groups = service.get_by_user_id(42).unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "Alpha Group");
        assert_eq!(groups[1].name, "Beta Group");
    }

    #[test]
    fn slug_collision_appends_suffix() {
        let mut repo = DieselRepoMock::default();
        repo.groups.insert(1, group(1, "avionics"));
        let state = state_with_repo(repo);
        let service = GroupService::new(&state);

        let payload = NewGroup {
            name: "Avionics".into(),
            description: None,
            owner_id: Some(1),
        };

        let id = service.create(&actor(), payload).unwrap();
        let stored = service.get_by_id(id).unwrap();
        assert_eq!(stored.slug, "avionics-2");
    }

    #[test]
    fn set_member_role_rejects_demoting_last_owner() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(7, actor());
        repo.groups.insert(1, group(1, "Alpha"));
        repo.group_members.push(GroupMember {
            group_id: 1,
            user_id: 7,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        let state = state_with_repo(repo);
        let service = GroupService::new(&state);

        let error = service.set_member_role(1, 7, 2).unwrap_err();
        assert!(matches!(error, RepoError::BadInput(_)));
    }

    #[test]
    fn set_member_role_allows_demoting_owner_after_promoting_another_owner() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(7, actor());
        repo.users
            .insert(8, DieselRepoMock::make_user(8, "copilot", ""));
        repo.groups.insert(1, group(1, "Alpha"));
        repo.group_members.push(GroupMember {
            group_id: 1,
            user_id: 7,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo.group_members.push(GroupMember {
            group_id: 1,
            user_id: 8,
            role: 3,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        let state = state_with_repo(repo);
        let service = GroupService::new(&state);

        service.set_member_role(1, 8, 1).unwrap();
        service.set_member_role(1, 7, 2).unwrap();

        let members = service.list_members(1).unwrap();
        let owner_roles: Vec<(i32, i32)> = members
            .into_iter()
            .map(|member| (member.user_id, member.role))
            .collect();
        assert!(owner_roles.contains(&(7, 2)));
        assert!(owner_roles.contains(&(8, 1)));
    }

    #[test]
    fn delete_rejects_groups_with_attached_projects() {
        let mut repo = DieselRepoMock::default();
        repo.groups.insert(1, group(1, "Alpha"));
        repo.projects.insert(
            2,
            crate::models::Project {
                id: 2,
                name: "Orbiter".into(),
                description: None,
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status: crate::status_enums::ProjectStatus::Active,
                owner_id: Some(7),
                slug: "orbiter".into(),
                group_id: Some(1),
            },
        );

        let state = state_with_repo(repo);
        let service = GroupService::new(&state);

        let error = service.delete(&actor(), 1).unwrap_err();
        assert!(matches!(error, RepoError::BadInput(_)));
    }
}

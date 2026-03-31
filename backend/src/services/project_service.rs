// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Service handling project level operations.

use crate::app::{AppState, DieselCachedRepo};
use crate::helper_functions::generate_unique_project_slug;
use crate::models::NewVerificationMethod;
use crate::models::{
    NewApplicability, NewCategory, NewProject, NewProjectMember, NewProjectRow, Project,
    UpdateProject, User,
};
use crate::namespaces::{resolve_project_namespace_entity, NamespaceEntity};
use crate::repository::errors::RepoError;
use crate::repository::{LookupRepository, ProjectMembersRepository, ProjectsRepository};
use crate::services::status_service::StatusService;
use crate::services::AuditLog;
use crate::services::{ApplicabilityService, CategoryService};
use crate::validation::{sanitize_optional_string, sanitize_string, validate_project};

/// High level project operations backed by the shared [`AppState`].
pub struct ProjectService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> ProjectService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve all projects.
    pub fn list_all(&self) -> Result<Vec<Project>, RepoError> {
        self.state.repo_read().get_projects_all()
    }

    /// Retrieve a project by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<Project, RepoError> {
        self.state.repo_read().get_project_by_id(id)
    }

    /// Retrieve a project by slug.
    pub fn get_by_slug(&self, slug: &str) -> Result<Project, RepoError> {
        self.state.repo_read().get_project_by_slug(slug)
    }

    pub fn get_by_user_namespace_and_slug(
        &self,
        username: &str,
        slug: &str,
    ) -> Result<Project, RepoError> {
        self.state
            .repo_read()
            .get_project_by_user_namespace_and_slug(username, slug)
    }

    pub fn get_by_group_namespace_and_slug(
        &self,
        group_slug: &str,
        slug: &str,
    ) -> Result<Project, RepoError> {
        self.state
            .repo_read()
            .get_project_by_group_namespace_and_slug(group_slug, slug)
    }

    pub fn get_by_namespace_and_slug(
        &self,
        namespace: &str,
        slug: &str,
    ) -> Result<Project, RepoError> {
        let repo = self.state.repo_read();
        match resolve_project_namespace_entity(&*repo, namespace)? {
            NamespaceEntity::User(user) => {
                repo.get_project_by_user_namespace_and_slug(&user.username, slug)
            }
            NamespaceEntity::Group(group) => {
                repo.get_project_by_group_namespace_and_slug(&group.slug, slug)
            }
        }
    }

    /// Retrieve all projects that the specified user is a member of.
    pub fn get_by_user_id(&self, id: i32) -> Result<Vec<Project>, RepoError> {
        let repo = self.state.repo_read();
        let memberships = repo.get_projects_for_user(id)?;

        let mut projects = Vec::with_capacity(memberships.len());

        for membership in memberships {
            match repo.get_project_by_id(membership.project_id) {
                Ok(project) => projects.push(project),
                Err(RepoError::NotFound) => continue,
                Err(err) => return Err(err),
            }
        }

        projects.sort_by_key(|a| a.name.to_lowercase());

        Ok(projects)
    }

    /// Create a new project entry and log the action.
    ///
    /// This method creates the project and automatically initializes default
    /// requirement and test statuses based on the hardcoded enums.
    pub fn create(&self, actor: &User, mut payload: NewProject) -> Result<i32, RepoError> {
        // If no owner provided, assign the actor as owner
        if payload.owner_id.is_none() {
            payload.owner_id = Some(actor.id);
        }

        self.prepare_new_payload(&mut payload)?;
        let slug = self.generate_slug(&payload.name, payload.owner_id, payload.group_id, None)?;

        let owner_id = payload.owner_id.unwrap_or(actor.id);
        let id = {
            let mut repo = self.state.repo_write();
            let id = repo.insert_new_project(&NewProjectRow {
                name: payload.name.clone(),
                slug,
                description: payload.description.clone(),
                owner_id: payload.owner_id,
                status: payload.status,
                group_id: payload.group_id,
            })?;
            repo.add_project_member(&NewProjectMember {
                project_id: id,
                user_id: owner_id,
                role: 1, // Owner
            })?;
            id
        };

        // Initialize default statuses for the new project
        let status_service = StatusService::new(self.state);
        status_service.initialize_default_statuses(id)?;

        // Initialize default verification methods for the new project
        let default_methods = [
            (
                "Inspection",
                "Nondestructive examination of a system or component",
                "INSP",
            ),
            ("Test", "Execution-based verification", "TEST"),
            ("Analysis", "Analysis-based verification", "ANALYSIS"),
            ("Review", "Review-based verification", "REVIEW"),
        ];
        {
            let mut repo = self.state.repo_write();
            for (title, description, tag) in default_methods {
                repo.insert_new_verification_method(&NewVerificationMethod {
                    id: None,
                    title: title.to_string(),
                    description: description.to_string(),
                    tag: tag.to_string(),
                    project_id: id,
                })?;
            }
        }

        // One default category and applicability so the new-requirement form can be submitted
        let category_service = CategoryService::new(self.state);
        category_service.create(
            actor,
            NewCategory {
                id: None,
                title: "Default".into(),
                description: "Default category".into(),
                tag: "DEF".into(),
                project_id: id,
            },
        )?;
        let applicability_service = ApplicabilityService::new(self.state);
        applicability_service.create(
            actor,
            NewApplicability {
                id: None,
                title: "Default".into(),
                description: "Default applicability".into(),
                tag: "DEF".into(),
                project_id: id,
            },
        )?;

        if let Ok(project) = self.get_by_id(id) {
            self.audit_created(actor, id, &project);
        }
        Ok(id)
    }

    /// Update an existing project entry and log the change.
    pub fn update(
        &self,
        actor: &User,
        id: i32,
        mut payload: UpdateProject,
    ) -> Result<Project, RepoError> {
        let before = self.get_by_id(id)?;

        // Handle owner_id logic before validation
        if payload.owner_id.is_none() {
            // If no owner provided in payload, use existing owner or assign actor
            payload.owner_id = before.owner_id.or(Some(actor.id));
        }

        self.prepare_update_payload(&mut payload)?;
        payload.slug = None;

        if self.namespace_changed(&before, &payload) {
            let next_slug = self.generate_slug(
                &before.slug,
                payload.owner_id,
                payload.group_id,
                Some(before.id),
            )?;
            if next_slug != before.slug {
                payload.slug = Some(next_slug);
            }
        }

        {
            let mut repo = self.state.repo_write();
            let updated = repo.edit_project(id, &payload)?;
            if !updated {
                return Err(RepoError::NotFound);
            }
        }

        let after = self.get_by_id(id)?;
        self.audit_updated(actor, &before, &after);
        Ok(after)
    }

    /// Delete a project entry and log the removal.
    pub fn delete(&self, actor: &User, id: i32) -> Result<Project, RepoError> {
        let removed = {
            let mut repo = self.state.repo_write();
            repo.delete_project(id)?
        };

        self.audit_deleted(actor, &removed);
        Ok(removed)
    }

    fn prepare_new_payload(&self, payload: &mut NewProject) -> Result<(), RepoError> {
        if matches!(payload.group_id, Some(group_id) if group_id <= 0) {
            payload.group_id = None;
        }

        sanitize_string(&mut payload.name);
        sanitize_optional_string(&mut payload.description);

        validate_project(payload).map_err(|err| RepoError::BadInput(err.to_string()))
    }

    fn prepare_update_payload(&self, payload: &mut UpdateProject) -> Result<(), RepoError> {
        if matches!(payload.group_id, Some(group_id) if group_id <= 0) {
            payload.group_id = None;
        }

        sanitize_string(&mut payload.name);
        sanitize_optional_string(&mut payload.description);

        let mut clone = NewProject {
            name: payload.name.clone(),
            description: payload.description.clone(),
            status: payload.status.unwrap_or_default(),
            owner_id: payload.owner_id,
            group_id: payload.group_id,
        };
        self.prepare_new_payload(&mut clone)
    }

    fn generate_slug(
        &self,
        name_or_slug_seed: &str,
        owner_id: Option<i32>,
        group_id: Option<i32>,
        exclude_project_id: Option<i32>,
    ) -> Result<String, RepoError> {
        let existing = self.existing_namespace_slugs(owner_id, group_id, exclude_project_id)?;

        Ok(generate_unique_project_slug(name_or_slug_seed, existing))
    }

    fn existing_namespace_slugs(
        &self,
        owner_id: Option<i32>,
        group_id: Option<i32>,
        exclude_project_id: Option<i32>,
    ) -> Result<Vec<String>, RepoError> {
        let projects = self.state.repo_read().get_projects_all()?;
        Ok(projects
            .into_iter()
            .filter(|project| exclude_project_id != Some(project.id))
            .filter(|project| self.project_in_namespace(project, owner_id, group_id))
            .map(|project| project.slug)
            .collect())
    }

    fn project_in_namespace(
        &self,
        project: &Project,
        owner_id: Option<i32>,
        group_id: Option<i32>,
    ) -> bool {
        if let Some(group_id) = group_id {
            return project.group_id == Some(group_id);
        }

        project.group_id.is_none() && project.owner_id == owner_id
    }

    fn namespace_changed(&self, before: &Project, payload: &UpdateProject) -> bool {
        before.group_id != payload.group_id
            || (payload.group_id.is_none() && before.owner_id != payload.owner_id)
    }
}

impl AuditLog for ProjectService<'_> {
    fn app_state(&self) -> &AppState<DieselCachedRepo> {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ProjectMember;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::services::StatusService;
    use crate::status_enums::ProjectStatus;
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

    fn project(id: i32, name: &str) -> Project {
        Project {
            id,
            name: name.into(),
            description: Some("Existing description".into()),
            creation_date: Some(timestamp()),
            update_date: Some(timestamp()),
            status: ProjectStatus::Active,
            owner_id: Some(1),
            slug: name.to_lowercase().replace(' ', "-"),
            group_id: None,
        }
    }

    #[test]
    fn create_trims_input_and_drops_blank_description() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let payload = NewProject {
            name: "  Project Phoenix  ".into(),
            description: Some("   ".into()),
            status: ProjectStatus::Active,
            owner_id: Some(1),
            group_id: None,
        };

        let id = service.create(&actor(), payload).unwrap();
        let stored = service.get_by_id(id).unwrap();

        assert_eq!(stored.name, "Project Phoenix");
        assert_eq!(stored.description, None);
        assert_eq!(stored.status, ProjectStatus::Active);
    }

    #[test]
    fn create_rejects_invalid_name() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let payload = NewProject {
            name: " ".into(),
            description: None,
            status: ProjectStatus::Completed,
            owner_id: None,
            group_id: None,
        };

        let err = service.create(&actor(), payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn create_treats_non_positive_group_id_as_none() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let payload = NewProject {
            name: "Grouped Project".into(),
            description: None,
            status: ProjectStatus::Active,
            owner_id: Some(1),
            group_id: Some(0),
        };

        let id = service.create(&actor(), payload).unwrap();
        let stored = service.get_by_id(id).unwrap();

        assert_eq!(stored.group_id, None);
    }

    #[test]
    fn create_initializes_default_statuses() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);
        let status_service = StatusService::new(&state);

        let payload = NewProject {
            name: "Test Project".into(),
            description: None,
            status: ProjectStatus::Active,
            owner_id: Some(1),
            group_id: None,
        };

        let project_id = service.create(&actor(), payload).unwrap();

        // Verify requirement statuses were created
        let req_statuses = status_service.list_requirement_statuses().unwrap();
        let project_req_statuses: Vec<_> = req_statuses
            .iter()
            .filter(|s| s.project_id == project_id)
            .collect();
        assert_eq!(
            project_req_statuses.len(),
            6,
            "Should have 6 requirement statuses"
        );

        // Verify test statuses were created
        let test_statuses = status_service.list_verification_statuses().unwrap();
        let project_test_statuses: Vec<_> = test_statuses
            .iter()
            .filter(|s| s.project_id == project_id)
            .collect();
        assert_eq!(
            project_test_statuses.len(),
            4,
            "Should have 4 test statuses"
        );

        // Verify specific status titles exist
        let req_titles: Vec<_> = project_req_statuses
            .iter()
            .map(|s| s.title.as_str())
            .collect();
        assert!(req_titles.contains(&"Draft"));
        assert!(req_titles.contains(&"Accepted"));
        assert!(req_titles.contains(&"Finished"));

        let test_titles: Vec<_> = project_test_statuses
            .iter()
            .map(|s| s.title.as_str())
            .collect();
        assert!(test_titles.contains(&"Passed"));
        assert!(test_titles.contains(&"Failed"));
        assert!(test_titles.contains(&"Pending"));
        assert!(test_titles.contains(&"In Progress"));
    }

    #[test]
    fn update_trims_and_persists_changes() {
        let mut repo = DieselRepoMock::default();
        repo.projects.insert(1, project(1, "Legacy"));
        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let payload = UpdateProject {
            name: "  Modernized  ".into(),
            description: Some("  Updated description  ".into()),
            status: Some(ProjectStatus::OnHold),
            owner_id: Some(2),
            slug: None,
            group_id: None,
        };

        let updated = service.update(&actor(), 1, payload).unwrap();
        assert_eq!(updated.name, "Modernized");
        assert_eq!(updated.description.as_deref(), Some("Updated description"));
        assert_eq!(updated.status, ProjectStatus::OnHold);
        assert_eq!(updated.owner_id, Some(2));
    }

    #[test]
    fn update_treats_non_positive_group_id_as_none() {
        let mut repo = DieselRepoMock::default();
        let mut existing = project(1, "Legacy");
        existing.group_id = Some(4);
        repo.projects.insert(1, existing);
        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let payload = UpdateProject {
            name: "Legacy".into(),
            description: Some("Still grouped".into()),
            status: Some(ProjectStatus::Active),
            owner_id: Some(1),
            slug: None,
            group_id: Some(0),
        };

        let updated = service.update(&actor(), 1, payload).unwrap();
        assert_eq!(updated.group_id, None);
    }

    #[test]
    fn update_returns_not_found_for_missing_project() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let payload = UpdateProject {
            name: "Valid".into(),
            description: Some("Desc".into()),
            status: Some(ProjectStatus::Active),
            owner_id: None,
            slug: None,
            group_id: None,
        };

        let err = service.update(&actor(), 99, payload).unwrap_err();
        assert!(matches!(err, RepoError::NotFound));
    }

    #[test]
    fn update_preserves_existing_owner_when_none_provided() {
        let mut repo = DieselRepoMock::default();
        repo.projects.insert(1, project(1, "Legacy"));
        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let payload = UpdateProject {
            name: "Legacy".into(),
            description: Some("Still around".into()),
            status: Some(ProjectStatus::Active),
            owner_id: None,
            slug: None,
            group_id: None,
        };

        let updated = service.update(&actor(), 1, payload).unwrap();
        assert_eq!(updated.owner_id, Some(1));
    }

    #[test]
    fn update_assigns_actor_when_owner_missing_from_existing_record() {
        let mut repo = DieselRepoMock::default();
        let mut orphaned = project(2, "Orphaned");
        orphaned.owner_id = None;
        repo.projects.insert(2, orphaned);
        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let mut editor = actor();
        editor.id = 314;

        let payload = UpdateProject {
            name: "Orphaned".into(),
            description: Some("Needs owner".into()),
            status: Some(ProjectStatus::Active),
            owner_id: None,
            slug: None,
            group_id: None,
        };

        let updated = service.update(&editor, 2, payload).unwrap();
        assert_eq!(updated.owner_id, Some(314));
    }

    #[test]
    fn get_by_user_id_returns_sorted_membership_projects() {
        let mut repo = DieselRepoMock::default();
        repo.projects.insert(1, project(1, "Beta Initiative"));
        repo.projects.insert(2, project(2, "Alpha Mission"));
        repo.projects.insert(3, project(3, "Gamma Plan"));

        let now = timestamp();
        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 42,
            role: 1,
            created_at: now,
            updated_at: now,
        });
        repo.project_members.push(ProjectMember {
            project_id: 2,
            user_id: 42,
            role: 2,
            created_at: now,
            updated_at: now,
        });
        repo.project_members.push(ProjectMember {
            project_id: 3,
            user_id: 7,
            role: 1,
            created_at: now,
            updated_at: now,
        });
        repo.project_members.push(ProjectMember {
            project_id: 99,
            user_id: 42,
            role: 1,
            created_at: now,
            updated_at: now,
        });

        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let projects = service.get_by_user_id(42).unwrap();

        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].name, "Alpha Mission");
        assert_eq!(projects[1].name, "Beta Initiative");
    }

    #[test]
    fn delete_removes_project() {
        let mut repo = DieselRepoMock::default();
        repo.projects.insert(4, project(4, "To remove"));
        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let deleted = service.delete(&actor(), 4).unwrap();
        assert_eq!(deleted.id, 4);
        assert!(matches!(service.get_by_id(4), Err(RepoError::NotFound)));
    }

    #[test]
    fn list_all_reads_all_projects() {
        let mut repo = DieselRepoMock::default();
        repo.projects.insert(1, project(1, "A"));
        repo.projects.insert(2, project(2, "B"));
        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let mut projects = service.list_all().unwrap();
        projects.sort_by_key(|p| p.id);
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].name, "A");
        assert_eq!(projects[1].name, "B");
    }

    #[test]
    fn get_by_user_id_returns_empty_for_user_without_projects() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let projects = service.get_by_user_id(42).unwrap();
        assert_eq!(projects.len(), 0);
    }

    #[test]
    fn get_by_user_id_returns_user_projects_sorted_by_name() {
        let mut repo = DieselRepoMock::default();
        repo.projects.insert(1, project(1, "Zebra Project"));
        repo.projects.insert(2, project(2, "Alpha Project"));
        repo.projects.insert(3, project(3, "Beta Project"));

        let now = timestamp();
        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 42,
            role: 1,
            created_at: now,
            updated_at: now,
        });
        repo.project_members.push(ProjectMember {
            project_id: 2,
            user_id: 42,
            role: 2,
            created_at: now,
            updated_at: now,
        });
        repo.project_members.push(ProjectMember {
            project_id: 3,
            user_id: 99,
            role: 1,
            created_at: now,
            updated_at: now,
        });

        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let projects = service.get_by_user_id(42).unwrap();
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].name, "Alpha Project");
        assert_eq!(projects[1].name, "Zebra Project");
    }

    #[test]
    fn get_by_user_id_handles_case_insensitive_sorting() {
        let mut repo = DieselRepoMock::default();
        repo.projects.insert(1, project(1, "beta project"));
        repo.projects.insert(2, project(2, "Alpha Project"));
        repo.projects.insert(3, project(3, "CHARLIE PROJECT"));

        let now = timestamp();
        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 42,
            role: 1,
            created_at: now,
            updated_at: now,
        });
        repo.project_members.push(ProjectMember {
            project_id: 2,
            user_id: 42,
            role: 2,
            created_at: now,
            updated_at: now,
        });
        repo.project_members.push(ProjectMember {
            project_id: 3,
            user_id: 42,
            role: 3,
            created_at: now,
            updated_at: now,
        });

        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let projects = service.get_by_user_id(42).unwrap();
        assert_eq!(projects.len(), 3);
        assert_eq!(projects[0].name, "Alpha Project");
        assert_eq!(projects[1].name, "beta project");
        assert_eq!(projects[2].name, "CHARLIE PROJECT");
    }

    #[test]
    fn get_by_user_id_skips_deleted_projects() {
        let mut repo = DieselRepoMock::default();
        repo.projects.insert(1, project(1, "Existing Project"));

        let now = timestamp();
        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 42,
            role: 1,
            created_at: now,
            updated_at: now,
        });
        repo.project_members.push(ProjectMember {
            project_id: 99,
            user_id: 42,
            role: 2,
            created_at: now,
            updated_at: now,
        });

        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        let projects = service.get_by_user_id(42).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "Existing Project");
    }

    #[test]
    fn get_by_user_id_handles_repo_error_gracefully() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        // This should not panic and should return an error or empty result
        let result = service.get_by_user_id(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn get_by_user_id_returns_projects_in_consistent_order() {
        let mut repo = DieselRepoMock::default();
        repo.projects.insert(1, project(1, "Project B"));
        repo.projects.insert(2, project(2, "Project A"));
        repo.projects.insert(3, project(3, "Project C"));

        let now = timestamp();
        for project_id in [1, 2, 3] {
            repo.project_members.push(ProjectMember {
                project_id,
                user_id: 42,
                role: 1,
                created_at: now,
                updated_at: now,
            });
        }

        let state = state_with_repo(repo);
        let service = ProjectService::new(&state);

        // Call multiple times to ensure consistent ordering
        let projects1 = service.get_by_user_id(42).unwrap();
        let projects2 = service.get_by_user_id(42).unwrap();
        let projects3 = service.get_by_user_id(42).unwrap();

        assert_eq!(projects1.len(), 3);
        assert_eq!(projects2.len(), 3);
        assert_eq!(projects3.len(), 3);

        for i in 0..3 {
            assert_eq!(projects1[i].name, projects2[i].name);
            assert_eq!(projects2[i].name, projects3[i].name);
        }

        assert_eq!(projects1[0].name, "Project A");
        assert_eq!(projects1[1].name, "Project B");
        assert_eq!(projects1[2].name, "Project C");
    }
}

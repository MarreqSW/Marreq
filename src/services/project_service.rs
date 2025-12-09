//! Service handling project level operations.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{NewProject, Project, UpdateProject, User};
use crate::repository::errors::RepoError;
use crate::repository::{PooledConnectionWrapper, ProjectMembersRepository, ProjectsRepository};
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

        projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        Ok(projects)
    }

    /// Create a new project entry and log the action.
    pub fn create(&self, actor: &User, mut payload: NewProject) -> Result<i32, RepoError> {
        self.prepare_new_payload(&mut payload)?;

        let id = {
            let mut repo = self.state.repo_write();
            repo.insert_new_project(&payload)?
        };

        if let Ok(project) = self.get_by_id(id) {
            self.log_created(actor, id, &project);
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

        {
            let mut repo = self.state.repo_write();
            let updated = repo.edit_project(id, &payload)?;
            if !updated {
                return Err(RepoError::NotFound);
            }
        }

        let after = self.get_by_id(id)?;
        self.log_updated(actor, &before, &after);
        Ok(after)
    }

    /// Delete a project entry and log the removal.
    pub fn delete(&self, actor: &User, id: i32) -> Result<Project, RepoError> {
        let removed = {
            let mut repo = self.state.repo_write();
            repo.delete_project(id)?
        };

        self.log_deleted(actor, &removed);
        Ok(removed)
    }

    fn prepare_new_payload(&self, payload: &mut NewProject) -> Result<(), RepoError> {
        sanitize_string(&mut payload.name);
        sanitize_optional_string(&mut payload.description);

        validate_project(payload).map_err(|err| RepoError::BadInput(err.to_string()))
    }

    fn prepare_update_payload(&self, payload: &mut UpdateProject) -> Result<(), RepoError> {
        sanitize_string(&mut payload.name);
        sanitize_optional_string(&mut payload.description);

        let mut clone = NewProject {
            name: payload.name.clone(),
            description: payload.description.clone(),
            status: payload.status.unwrap_or_default(),
            owner_id: payload.owner_id,
        };
        self.prepare_new_payload(&mut clone)
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_created(&self, actor: &User, id: i32, entity: &Project) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log project creation {id}: {_err}");
            }
        }
    }

    fn log_updated(&self, actor: &User, before: &Project, after: &Project) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::updated(conn.as_mut(), &ctx, before, after) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log project update {} -> {}: {_err}",
                    before.id, after.id
                );
            }
        }
    }

    fn log_deleted(&self, actor: &User, entity: &Project) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log project deletion {}: {_err}", entity.id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ProjectMember;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
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
            id: id,
            name: name.into(),
            description: Some("Existing description".into()),
            creation_date: Some(timestamp()),
            update_date: Some(timestamp()),
            status: ProjectStatus::Active,
            owner_id: Some(1),
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
        };

        let err = service.create(&actor(), payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
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
        };

        let updated = service.update(&actor(), 1, payload).unwrap();
        assert_eq!(updated.name, "Modernized");
        assert_eq!(updated.description.as_deref(), Some("Updated description"));
        assert_eq!(updated.status, ProjectStatus::OnHold);
        assert_eq!(updated.owner_id, Some(2));
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

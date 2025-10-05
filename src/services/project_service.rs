//! Service handling project level operations.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{NewProject, Project, UpdateProject, User};
use crate::repository::errors::RepoError;
use crate::repository::{PooledConnectionWrapper, ProjectsRepository};
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
        self.prepare_update_payload(&mut payload)?;

        let before = self.get_by_id(id)?;

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
        sanitize_string(&mut payload.project_name);
        sanitize_optional_string(&mut payload.project_description);

        validate_project(payload).map_err(|err| RepoError::BadInput(err.to_string()))
    }

    fn prepare_update_payload(&self, payload: &mut UpdateProject) -> Result<(), RepoError> {
        sanitize_string(&mut payload.project_name);
        sanitize_optional_string(&mut payload.project_description);

        let mut clone = NewProject {
            project_name: payload.project_name.clone(),
            project_description: payload.project_description.clone(),
            project_status: payload.project_status.clone(),
            project_owner_id: payload.project_owner_id,
        };
        self.prepare_new_payload(&mut clone)
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_created(&self, actor: &User, id: i32, entity: &Project) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.user_id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log project creation {id}: {_err}");
            }
        }
    }

    fn log_updated(&self, actor: &User, before: &Project, after: &Project) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.user_id);
            if let Err(_err) = Logger::updated(conn.as_mut(), &ctx, before, after) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log project update {} -> {}: {_err}",
                    before.project_id, after.project_id
                );
            }
        }
    }

    fn log_deleted(&self, actor: &User, entity: &Project) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.user_id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log project deletion {}: {_err}",
                    entity.project_id
                );
            }
        }
    }
}

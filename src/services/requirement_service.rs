//! Service providing requirement related operations.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{NewRequirement, Requirement, User};
use crate::repository::errors::RepoError;
use crate::repository::{PooledConnectionWrapper, RequirementsRepository};
use crate::validation::{sanitize_optional_string, sanitize_string, validate_requirement};

/// High level operations for requirements backed by the shared [`AppState`].
pub struct RequirementService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> RequirementService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve all requirements.
    pub fn list_all(&self) -> Result<Vec<Requirement>, RepoError> {
        self.state.repo_read().get_requirements_all()
    }

    /// Retrieve requirements scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<Requirement>, RepoError> {
        self.state
            .repo_read()
            .get_requirements_by_project(project_id)
    }

    /// Retrieve a single requirement by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<Requirement, RepoError> {
        self.state.repo_read().get_requirement_by_id(id)
    }

    /// Create a new requirement entry and log the action.
    pub fn create(&self, user: &User, mut payload: NewRequirement) -> Result<i32, RepoError> {
        self.prepare_payload(&mut payload)?;

        let id = {
            let mut repo = self.state.repo_write();
            repo.insert_new_requirement(&payload)?
        };

        self.log_created(user, id, &payload);
        Ok(id)
    }

    /// Update an existing requirement entry and log the change.
    pub fn update(
        &self,
        user: &User,
        id: i32,
        mut payload: NewRequirement,
    ) -> Result<Requirement, RepoError> {
        self.prepare_payload(&mut payload)?;
        payload.req_id = Some(id);

        let before = self.get_by_id(id)?;

        {
            let mut repo = self.state.repo_write();
            let updated = repo.edit_requirement(&payload)?;
            if !updated {
                return Err(RepoError::NotFound);
            }
        }

        let after = self.get_by_id(id)?;
        self.log_updated(user, &before, &after);
        Ok(after)
    }

    /// Delete an requirement entry and log the removal.
    pub fn delete(&self, user: &User, id: i32) -> Result<Requirement, RepoError> {
        let removed = {
            let mut repo = self.state.repo_write();
            repo.delete_requirement(id)?
        };

        self.log_deleted(user, &removed);
        Ok(removed)
    }

    fn prepare_payload(&self, payload: &mut NewRequirement) -> Result<(), RepoError> {
        sanitize_string(&mut payload.req_title);
        sanitize_string(&mut payload.req_description);
        sanitize_string(&mut payload.req_reference);
        sanitize_string(&mut payload.req_link);
        sanitize_optional_string(&mut payload.req_justification);

        validate_requirement(payload).map_err(|err| RepoError::BadInput(err.to_string()))
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_created(&self, user: &User, id: i32, entity: &NewRequirement) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.user_id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log requirement creation {id}: {_err}");
            }
        }
    }

    fn log_updated(&self, user: &User, before: &Requirement, after: &Requirement) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.user_id);
            if let Err(_err) = Logger::updated(conn.as_mut(), &ctx, before, after) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log requirement update {} -> {}: {_err}",
                    before.req_id, after.req_id
                );
            }
        }
    }

    fn log_deleted(&self, user: &User, entity: &Requirement) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.user_id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log requirement deletion {}: {_err}",
                    entity.req_id
                );
            }
        }
    }
}

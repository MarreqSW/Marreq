//! Applicability service centralizing CRUD logic and logging.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{Applicability, NewApplicability, User};
use crate::repository::errors::RepoError;
use crate::repository::{LookupRepository, PooledConnectionWrapper};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref TAG_REGEX: Regex =
        Regex::new(r"^[A-Za-z0-9_]+$").expect("valid applicability tag regex");
}

/// Service wrapper that provides applicability operations backed by the shared AppState.
pub struct ApplicabilityService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> ApplicabilityService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve all applicability entries.
    pub fn list_all(&self) -> Result<Vec<Applicability>, RepoError> {
        self.state.repo_read().get_applicability_all()
    }

    /// Retrieve applicability entries scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<Applicability>, RepoError> {
        self.state
            .repo_read()
            .get_applicability_by_project(project_id)
    }

    /// Retrieve a single applicability by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<Applicability, RepoError> {
        self.state.repo_read().get_applicability_by_id(id)
    }

    /// Create a new applicability entry and log the action.
    pub fn create(&self, user: &User, mut new_app: NewApplicability) -> Result<i32, RepoError> {
        self.prepare_payload(&mut new_app)?;

        let id = {
            let mut repo = self.state.repo_write();
            repo.insert_new_applicability(&new_app)?
        };

        self.log_created(user, id, &new_app);
        Ok(id)
    }

    /// Update an existing applicability entry and log the change.
    pub fn update(
        &self,
        user: &User,
        id: i32,
        mut updated_app: NewApplicability,
    ) -> Result<Applicability, RepoError> {
        let before = self.get_by_id(id)?;

        updated_app.app_id = Some(id);
        self.prepare_payload(&mut updated_app)?;

        {
            let mut repo = self.state.repo_write();
            let updated = repo.edit_applicability(&updated_app)?;
            if !updated {
                return Err(RepoError::NotFound);
            }
        }

        let after = self.get_by_id(id)?;
        self.log_updated(user, &before, &after);
        Ok(after)
    }

    /// Delete an applicability entry and log the removal.
    pub fn delete(&self, user: &User, id: i32) -> Result<Applicability, RepoError> {
        let deleted = {
            let mut repo = self.state.repo_write();
            repo.delete_applicability(id)?
        };

        self.log_deleted(user, &deleted);
        Ok(deleted)
    }

    fn prepare_payload(&self, payload: &mut NewApplicability) -> Result<(), RepoError> {
        sanitize(&mut payload.app_title);
        sanitize(&mut payload.app_description);
        sanitize(&mut payload.app_tag);

        validate(payload)
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_created(&self, user: &User, id: i32, entity: &NewApplicability) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.user_id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log applicability creation {id}: {_err}");
            }
        }
    }

    fn log_updated(&self, user: &User, before: &Applicability, after: &Applicability) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.user_id);
            if let Err(_err) = Logger::updated(conn.as_mut(), &ctx, before, after) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log applicability update {} -> {}: {_err}",
                    before.app_id, after.app_id
                );
            }
        }
    }

    fn log_deleted(&self, user: &User, entity: &Applicability) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.user_id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log applicability deletion {}: {_err}",
                    entity.app_id
                );
            }
        }
    }
}

fn sanitize(value: &mut String) {
    *value = value.trim().to_string();
}

fn validate(payload: &NewApplicability) -> Result<(), RepoError> {
    if payload.app_title.is_empty() {
        return Err(bad_input("app_title is required"));
    }
    if payload.app_title.len() > 100 {
        return Err(bad_input("app_title must be at most 100 characters"));
    }
    if payload.app_title.len() < 2 {
        return Err(bad_input("app_title must be at least 2 characters"));
    }

    if payload.app_description.is_empty() {
        return Err(bad_input("app_description is required"));
    }
    if payload.app_description.len() > 500 {
        return Err(bad_input("app_description must be at most 500 characters"));
    }

    if payload.app_tag.is_empty() {
        return Err(bad_input("app_tag is required"));
    }
    if payload.app_tag.len() > 50 {
        return Err(bad_input("app_tag must be at most 50 characters"));
    }
    if !TAG_REGEX.is_match(&payload.app_tag) {
        return Err(bad_input(
            "app_tag must only contain letters, numbers, or underscores",
        ));
    }

    if payload.project_id <= 0 {
        return Err(bad_input("project_id must be positive"));
    }

    Ok(())
}

fn bad_input(message: impl Into<String>) -> RepoError {
    RepoError::BadInput(message.into())
}

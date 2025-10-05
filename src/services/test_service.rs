//! Test service for managing test cases business logic.
//!
//! This service handles all test-related operations including CRUD operations,
//! validation, caching, and audit logging.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{NewTest, Test, User};
use crate::repository::errors::RepoError;
use crate::repository::PooledConnectionWrapper;
use crate::repository::TestsRepository;

/// Service wrapper that provides test operations backed by the shared AppState.
pub struct TestService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> TestService<'a> {
    /// Create a new service instance bound to the provided test state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve all Test entries.
    pub fn list_all(&self) -> Result<Vec<Test>, RepoError> {
        self.state.repo_read().get_tests_all()
    }

    /// Retrieve Test entries scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<Test>, RepoError> {
        self.state.repo_read().get_tests_by_project(project_id)
    }

    /// Retrieve a single Test by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<Test, RepoError> {
        self.state.repo_read().get_test_by_id(id)
    }

    /// Get tests by status
    pub async fn get_by_status(&self, _status_id: i32) -> Result<Vec<Test>, RepoError> {
        todo!()
    }

    /// Get tests by parent (hierarchical structure)
    pub async fn get_by_parent(&self, _parent_id: i32) -> Result<Vec<Test>, RepoError> {
        todo!()
    }

    /// Create a new test entry and log the action.
    pub fn create(&self, user: &User, new_test: NewTest) -> Result<i32, RepoError> {
        let id = {
            let mut repo = self.state.repo_write();
            repo.insert_test(&new_test)?
        };

        self.log_created(user, id, &new_test);
        Ok(id)
    }

    /// Update an existing test entry and log the change.
    pub fn update(
        &self,
        user: &User,
        id: i32,
        mut updated_test: NewTest,
    ) -> Result<Test, RepoError> {
        let before = self.get_by_id(id)?;

        updated_test.test_id = Some(id);
        {
            let mut repo = self.state.repo_write();
            let updated = repo.edit_test(&updated_test)?;
            if !updated {
                return Err(RepoError::NotFound);
            }
        }

        let after = self.get_by_id(id)?;
        self.log_updated(user, &before, &after);
        Ok(after)
    }

    /// Delete an test entry and log the removal.
    pub fn delete(&self, user: &User, id: i32) -> Result<Test, RepoError> {
        let deleted = {
            let mut repo = self.state.repo_write();
            repo.delete_test(id)?
        };

        self.log_deleted(user, &deleted);
        Ok(deleted)
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_created(&self, user: &User, id: i32, entity: &NewTest) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.user_id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log applicability creation {id}: {_err}");
            }
        }
    }

    fn log_updated(&self, user: &User, before: &Test, after: &Test) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.user_id);
            if let Err(_err) = Logger::updated(conn.as_mut(), &ctx, before, after) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log applicability update {} -> {}: {_err}",
                    before.test_id, after.test_id
                );
            }
        }
    }

    fn log_deleted(&self, user: &User, entity: &Test) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.user_id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log applicability deletion {}: {_err}",
                    entity.test_id
                );
            }
        }
    }
}

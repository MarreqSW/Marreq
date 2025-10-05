//! Category service for managing requirement categories business logic.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{Category, NewCategory, User};
use crate::repository::errors::RepoError;
use crate::repository::{LookupRepository, PooledConnectionWrapper};

pub struct CategoryService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> CategoryService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve all category entries.
    pub fn list_all(&self) -> Result<Vec<Category>, RepoError> {
        self.state.repo_read().get_categories_all()
    }

    /// Retrieve Category entries scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<Category>, RepoError> {
        self.state.repo_read().get_categories_by_project(project_id)
    }

    /// Retrieve a single Category by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<Category, RepoError> {
        self.state.repo_read().get_category_by_id(id)
    }

    /// Create a new Category entry and log the action.
    pub fn create(&self, user: &User, new_cat: NewCategory) -> Result<i32, RepoError> {
        let id = {
            let mut repo = self.state.repo_write();
            repo.insert_new_category(&new_cat)?
        };

        self.log_created(user, id, &new_cat);
        Ok(id)
    }

    /// Update an existing Category entry and log the change.
    pub fn update(
        &self,
        user: &User,
        id: i32,
        mut updated_cat: NewCategory,
    ) -> Result<Category, RepoError> {
        let before = self.get_by_id(id)?;

        updated_cat.cat_id = Some(id);

        {
            let mut repo = self.state.repo_write();
            let updated = repo.edit_category(&updated_cat)?;
            if !updated {
                return Err(RepoError::NotFound);
            }
        }

        let after = self.get_by_id(id)?;
        self.log_updated(user, &before, &after);
        Ok(after)
    }

    /// Delete an Category entry and log the removal.
    pub fn delete(&self, user: &User, id: i32) -> Result<Category, RepoError> {
        let deleted = {
            let mut repo = self.state.repo_write();
            repo.delete_category(id)?
        };

        self.log_deleted(user, &deleted);
        Ok(deleted)
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_created(&self, user: &User, id: i32, entity: &NewCategory) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.user_id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log category creation {id}: {_err}");
            }
        }
    }

    fn log_updated(&self, user: &User, before: &Category, after: &Category) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.user_id);
            if let Err(_err) = Logger::updated(conn.as_mut(), &ctx, before, after) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log category update {} -> {}: {_err}",
                    before.cat_id, after.cat_id
                );
            }
        }
    }

    fn log_deleted(&self, user: &User, entity: &Category) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.user_id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log category deletion {}: {_err}", entity.cat_id);
            }
        }
    }
}

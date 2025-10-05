//! Service encapsulating user related operations.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{NewUser, User};
use crate::repository::errors::RepoError;
use crate::repository::{PooledConnectionWrapper, UserRepository};
use crate::validation::sanitize_string;

/// High level user operations backed by the shared [`AppState`].
pub struct UserService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> UserService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve all users.
    pub fn list_all(&self) -> Result<Vec<User>, RepoError> {
        self.state.repo_read().get_users_all()
    }

    /// Retrieve a single user by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<User, RepoError> {
        self.state.repo_read().get_user_by_id(id)
    }

    /// Create a new user entry and log the action.
    pub fn create(&self, actor: &User, mut payload: NewUser) -> Result<i32, RepoError> {
        sanitize_string(&mut payload.user_username);
        sanitize_string(&mut payload.user_name);
        sanitize_string(&mut payload.user_email);

        let id = {
            let mut repo = self.state.repo_write();
            repo.insert_user(&payload)?
        };

        self.log_created(actor, id, &payload);
        Ok(id)
    }

    /// Delete a user entry and log the removal.
    pub fn delete(&self, actor: &User, id: i32) -> Result<User, RepoError> {
        let removed = {
            let mut repo = self.state.repo_write();
            repo.delete_user(id)?
        };

        self.log_deleted(actor, &removed);
        Ok(removed)
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_created(&self, actor: &User, id: i32, entity: &NewUser) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.user_id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log user creation {id}: {_err}");
            }
        }
    }

    fn log_deleted(&self, actor: &User, entity: &User) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.user_id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log user deletion {}: {_err}", entity.user_id);
            }
        }
    }
}

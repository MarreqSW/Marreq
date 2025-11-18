//! Service encapsulating user related operations.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{NewUser, UpdateUser, User};
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

    /// Retrieve a vector of users members of a project.
    pub fn get_by_project(&self, id: i32) -> Result<Vec<User>, RepoError> {
        use crate::repository::ProjectMembersRepository;
        self.state
            .repo_read()
            .get_members_by_project(id)?
            .into_iter()
            .map(|member| self.get_by_id(member.user_id))
            .collect()
    }

    /// Retrieve a single user by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<User, RepoError> {
        self.state.repo_read().get_user_by_id(id)
    }

    /// Create a new user entry and log the action.
    pub fn create(&self, actor: &User, mut payload: NewUser) -> Result<i32, RepoError> {
        sanitize_string(&mut payload.username);
        sanitize_string(&mut payload.name);
        sanitize_string(&mut payload.email);

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

    /// Update a user's non-password fields and log the change.
    pub fn update_without_password(
        &self,
        actor: &User,
        payload: &UpdateUser,
    ) -> Result<bool, RepoError> {
        let id = payload.id.ok_or(RepoError::NotFound)?;
        if !actor.is_admin && actor.id != id {
            return Err(RepoError::Unauthorized);
        }
        let old = self.get_by_id(id)?;

        let updated = {
            let mut repo = self.state.repo_write();
            repo.update_user_without_password(payload)?
        };

        if updated {
            if let Ok(mut conn) = self.db_connection() {
                let ctx = LogCtx::new(actor.id);
                if let Err(_err) =
                    Logger::updated(conn.as_mut(), &ctx, &old, &self.get_by_id(old.id)?)
                {
                    #[cfg(debug_assertions)]
                    eprintln!("Failed to log user update {}: {_err}", old.id);
                }
            }
        }

        Ok(updated)
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_created(&self, actor: &User, id: i32, entity: &NewUser) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log user creation {id}: {_err}");
            }
        }
    }

    fn log_deleted(&self, actor: &User, entity: &User) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log user deletion {}: {_err}", entity.id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use std::sync::{Arc, RwLock};

    fn state_with_repo(repo: DieselRepoMock) -> AppState<DieselCachedRepo> {
        AppState {
            repo: Arc::new(RwLock::new(DieselCachedRepo::new(repo, 0))),
        }
    }

    fn actor() -> User {
        DieselRepoMock::make_user(99, "admin", "")
    }

    fn new_user_payload() -> NewUser {
        NewUser {
            id: None,
            username: "  alice  ".into(),
            name: "  Alice Example  ".into(),
            email: "  alice@example.com  ".into(),
            password_hash: "secret".into(),
            is_admin: false,
        }
    }

    fn sample_user(id: i32, username: &str) -> User {
        let mut user = DieselRepoMock::make_user(id, username, "hash");
        user.name = "Existing".into();
        user.email = "existing@example.com".into();
        user
    }

    #[test]
    fn create_sanitizes_strings_before_inserting() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let payload = new_user_payload();
        let id = service.create(&actor(), payload).unwrap();

        let stored = service.get_by_id(id).unwrap();
        assert_eq!(stored.username, "alice");
        assert_eq!(stored.name, "Alice Example");
        assert_eq!(stored.email, "alice@example.com");
    }

    #[test]
    fn delete_removes_user() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, sample_user(1, "bob"));
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let removed = service.delete(&actor(), 1).unwrap();
        assert_eq!(removed.id, 1);
        assert!(matches!(service.get_by_id(1), Err(RepoError::NotFound)));
    }

    #[test]
    fn list_all_returns_all_users() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, sample_user(1, "bob"));
        repo.users.insert(2, sample_user(2, "carol"));
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let mut users = service.list_all().unwrap();
        users.sort_by_key(|u| u.id);
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].username, "bob");
        assert_eq!(users[1].username, "carol");
    }

    #[test]
    fn non_admin_cannot_update_another_user() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, sample_user(1, "alice"));
        repo.users.insert(2, sample_user(2, "carol"));
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let actor = sample_user(1, "alice");
        let update = UpdateUser {
            id: Some(2),
            username: "carol".into(),
            name: "Carol Updated".into(),
            email: "carol.updated@example.com".into(),
            is_admin: false,
        };

        let err = service
            .update_without_password(&actor, &update)
            .expect_err("non-admin should not update other users");

        assert!(matches!(err, RepoError::Unauthorized));
    }

    #[test]
    fn admin_can_update_other_users() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, sample_user(1, "alice"));
        repo.users.insert(2, sample_user(2, "carol"));
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let mut admin = sample_user(1, "alice");
        admin.is_admin = true;

        let update = UpdateUser {
            id: Some(2),
            username: "carol".into(),
            name: "Carol Updated".into(),
            email: "carol.updated@example.com".into(),
            is_admin: true,
        };

        let updated = service
            .update_without_password(&admin, &update)
            .expect("admin should update other users");
        assert!(updated);

        let stored = service.get_by_id(2).unwrap();
        assert_eq!(stored.name, "Carol Updated");
        assert_eq!(stored.email, "carol.updated@example.com");
        assert!(stored.is_admin);
    }
}

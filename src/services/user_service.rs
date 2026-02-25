// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

//! Service encapsulating user related operations.

use crate::app::{AppState, DieselCachedRepo};
use crate::auth::password::hash_password;
use crate::auth::password_policy::{validate_password, PasswordContext};
use crate::logger::{LogCtx, Logger};
use crate::models::{NewUser, UpdateUser, User, UserCreateRequest};
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

    /// Create a new user from a request containing a plain password.
    ///
    /// This method validates input, hashes the password, and creates the user.
    /// Always provide plain text passwords - they will be hashed server-side.
    pub fn create(&self, actor: &User, request: UserCreateRequest) -> Result<i32, RepoError> {
        validate_password(
            &request.password,
            PasswordContext {
                username: Some(&request.username),
                email: Some(&request.email),
                full_name: Some(&request.name),
            },
        )
        .map_err(|e| RepoError::BadInput(e.to_string()))?;

        let password_hash = hash_password(&request.password)
            .map_err(|e| RepoError::BadInput(format!("Password hashing failed: {}", e)))?;

        let mut payload = NewUser {
            id: None,
            username: request.username,
            name: request.name,
            email: request.email,
            password_hash,
            is_admin: request.is_admin,
        };

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

    fn new_user_payload() -> UserCreateRequest {
        UserCreateRequest {
            username: "  alice  ".into(),
            name: "  Alice Example  ".into(),
            email: "  alice@example.com  ".into(),
            password: "CobaltRiver!Vacuum88".into(),
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

    #[test]
    fn create_hashes_password_and_creates_user() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let request = UserCreateRequest {
            username: "  bob  ".into(),
            name: "  Bob Example  ".into(),
            email: "  bob@example.com  ".into(),
            password: "Skyline!Current_2026".into(),
            is_admin: false,
        };

        let id = service.create(&actor(), request).unwrap();
        let stored = service.get_by_id(id).unwrap();

        assert_eq!(stored.username, "bob");
        assert_eq!(stored.name, "Bob Example");
        assert_eq!(stored.email, "bob@example.com");
        // Password should be hashed (argon2 hashes start with $argon2)
        assert!(stored.password_hash.starts_with("$argon2"));
        assert_ne!(stored.password_hash, "Skyline!Current_2026");
    }

    #[test]
    fn get_by_id_returns_not_found_for_nonexistent_user() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let result = service.get_by_id(999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn get_by_project_returns_project_members() {
        let mut repo = DieselRepoMock::default();
        use crate::models::ProjectMember;
        use chrono::NaiveDate;

        let timestamp = NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        repo.users.insert(1, sample_user(1, "alice"));
        repo.users.insert(2, sample_user(2, "bob"));

        repo.project_members.push(ProjectMember {
            project_id: 10,
            user_id: 1,
            role: 1,
            created_at: timestamp,
            updated_at: timestamp,
        });
        repo.project_members.push(ProjectMember {
            project_id: 10,
            user_id: 2,
            role: 2,
            created_at: timestamp,
            updated_at: timestamp,
        });

        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let users = service.get_by_project(10).unwrap();
        assert_eq!(users.len(), 2);
        assert!(users.iter().any(|u| u.id == 1));
        assert!(users.iter().any(|u| u.id == 2));
    }

    #[test]
    fn get_by_project_returns_empty_for_nonexistent_project() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let users = service.get_by_project(999).unwrap();
        assert_eq!(users.len(), 0);
    }

    #[test]
    fn get_by_project_handles_missing_users_gracefully() {
        let mut repo = DieselRepoMock::default();
        use crate::models::ProjectMember;
        use chrono::NaiveDate;

        let timestamp = NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        // Add membership for user that doesn't exist
        repo.project_members.push(ProjectMember {
            project_id: 10,
            user_id: 999,
            role: 1,
            created_at: timestamp,
            updated_at: timestamp,
        });

        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        // Should propagate the NotFound error
        let result = service.get_by_project(10);
        assert!(result.is_err());
    }

    #[test]
    fn update_without_password_requires_id() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let update = UpdateUser {
            id: None, // Missing ID
            username: "test".into(),
            name: "Test".into(),
            email: "test@example.com".into(),
            is_admin: false,
        };

        let err = service
            .update_without_password(&actor(), &update)
            .unwrap_err();
        assert!(matches!(err, RepoError::NotFound));
    }

    #[test]
    fn update_without_password_allows_user_to_update_self() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, sample_user(1, "alice"));

        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let actor = sample_user(1, "alice");
        let update = UpdateUser {
            id: Some(1),
            username: "alice".into(),
            name: "Alice Updated".into(),
            email: "alice.updated@example.com".into(),
            is_admin: false,
        };

        let updated = service
            .update_without_password(&actor, &update)
            .expect("user should update themselves");
        assert!(updated);

        let stored = service.get_by_id(1).unwrap();
        assert_eq!(stored.name, "Alice Updated");
    }

    #[test]
    fn update_without_password_returns_not_found_for_missing_user() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let mut admin_actor = actor();
        admin_actor.is_admin = true; // Ensure actor is admin

        let update = UpdateUser {
            id: Some(999),
            username: "nonexistent".into(),
            name: "Nonexistent".into(),
            email: "nonexistent@example.com".into(),
            is_admin: false,
        };

        // get_by_id is called first, which will return NotFound
        let err = service
            .update_without_password(&admin_actor, &update)
            .unwrap_err();
        // The error comes from get_by_id, which returns NotFound
        assert!(matches!(err, RepoError::NotFound));
    }

    #[test]
    fn delete_returns_not_found_for_missing_user() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let result = service.delete(&actor(), 999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn list_all_returns_empty_when_no_users() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let users = service.list_all().unwrap();
        assert_eq!(users.len(), 0);
    }

    #[test]
    fn create_rejects_invalid_password() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let request = UserCreateRequest {
            username: "test".into(),
            name: "Test".into(),
            email: "test@example.com".into(),
            password: "".into(),
            is_admin: false,
        };

        let result = service.create(&actor(), request);
        assert!(matches!(result, Err(RepoError::BadInput(_))));
    }

    #[test]
    fn create_rejects_common_passwords() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let request = UserCreateRequest {
            username: "test".into(),
            name: "Test User".into(),
            email: "test@example.com".into(),
            password: "password1".into(),
            is_admin: false,
        };

        let result = service.create(&actor(), request);
        assert!(matches!(result, Err(RepoError::BadInput(_))));
    }

    #[test]
    fn create_rejects_context_specific_passwords() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let request = UserCreateRequest {
            username: "alice".into(),
            name: "Alice Example".into(),
            email: "alice@example.com".into(),
            password: "alice-secure-pass-2026".into(),
            is_admin: false,
        };

        let result = service.create(&actor(), request);
        assert!(matches!(result, Err(RepoError::BadInput(_))));
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Service encapsulating user related operations.

use crate::app::{AppState, DieselCachedRepo};
use crate::auth::errors::AuthError;
use crate::auth::password::{admin_set_user_password, hash_password, verify_password};
use crate::auth::password_policy::{validate_password, PasswordContext};
use crate::logger::{LogCtx, Logger};
use crate::models::{
    ActionType, EntityType, NewUser, ProfileUpdate, UpdateUser, User, UserCreateRequest,
};
use crate::namespaces::{ensure_namespace_segment_available, NamespaceAvailabilityOptions};
use crate::repository::errors::RepoError;
use crate::repository::{SessionRepository, UserRepository};
use crate::services::AuditLog;
use crate::validation::{sanitize_string, validate_user};
use diesel::result::{DatabaseErrorKind, Error as DieselError};

const LAST_ADMIN_MESSAGE: &str = "At least one administrator is required.";
const USER_STILL_REFERENCED_MESSAGE: &str = "User still owns or authored records (groups, baselines, saved views, requirements, …); reassign them before deleting the user.";

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
        let repo = self.state.repo_read();
        let members = repo.get_members_by_project(id)?;
        members
            .into_iter()
            .map(|member| repo.get_user_by_id(member.user_id))
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
        // Cloud mode: API-driven user creation that grants admin is forbidden.
        // The site admin is bootstrapped from environment variables instead.
        if request.is_admin && !crate::deployment::current().allows_admin_promotion() {
            return Err(RepoError::BadInput(
                "admin promotion is disabled in this deployment mode".into(),
            ));
        }

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
            password_hash: Some(password_hash),
            is_admin: request.is_admin,
            email_verified: None,
        };

        sanitize_string(&mut payload.username);
        sanitize_string(&mut payload.name);
        sanitize_string(&mut payload.email);
        // Enforce case-insensitive identity: always persist lowercase so that the
        // functional unique indexes (lower(username), lower(email)) are deterministic.
        payload.username = payload.username.to_lowercase();
        payload.email = payload.email.to_lowercase();

        validate_user(&payload).map_err(|e| RepoError::BadInput(e.to_string()))?;
        self.ensure_username_namespace_available(&payload.username, None)?;

        let id = {
            let mut repo = self.state.repo_write();
            repo.insert_user(&payload)?
        };

        self.audit_created(actor, id, &payload);
        Ok(id)
    }

    /// Delete a user entry and log the removal.
    ///
    /// Refuses to delete the actor's own account or the last administrator, and
    /// reports rows that still reference the user as a conflict.
    pub fn delete(&self, actor: &User, id: i32) -> Result<User, RepoError> {
        if actor.id == id {
            return Err(RepoError::BadInput(
                "You cannot delete your own account.".into(),
            ));
        }
        let target = self.get_by_id(id)?;
        if target.is_admin && self.admin_count()? <= 1 {
            return Err(RepoError::Duplicate(LAST_ADMIN_MESSAGE.into()));
        }

        let removed = {
            let mut repo = self.state.repo_write();
            repo.delete_user(id).map_err(|err| match err {
                RepoError::Db(DieselError::DatabaseError(
                    DatabaseErrorKind::ForeignKeyViolation,
                    _,
                )) => RepoError::Duplicate(USER_STILL_REFERENCED_MESSAGE.into()),
                other => other,
            })?
        };

        self.audit_deleted(actor, &removed);
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

        // Normalize identity fields before persisting so they match the
        // lower(username) / lower(email) unique indexes.
        let normalized = UpdateUser {
            id: payload.id,
            username: payload.username.trim().to_lowercase(),
            name: payload.name.trim().to_string(),
            email: payload.email.trim().to_lowercase(),
            is_admin: payload.is_admin,
        };

        let validation_payload = NewUser {
            id: normalized.id,
            username: normalized.username.clone(),
            name: normalized.name.clone(),
            email: normalized.email.clone(),
            password_hash: old.password_hash.clone(),
            is_admin: normalized.is_admin,
            email_verified: None,
        };
        validate_user(&validation_payload).map_err(|e| RepoError::BadInput(e.to_string()))?;
        self.ensure_username_namespace_available(&normalized.username, Some(id))?;

        let updated = {
            let mut repo = self.state.repo_write();
            repo.update_user_without_password(&normalized)?
        };

        if updated {
            let after = self.get_by_id(old.id)?;
            self.audit_updated(actor, &old, &after);
        }

        Ok(updated)
    }

    /// Update another user's profile and admin flag on behalf of a site administrator.
    ///
    /// Changing the admin flag follows the deployment mode, and the last
    /// administrator (or the actor themselves) cannot be demoted.
    pub fn admin_update(
        &self,
        actor: &User,
        id: i32,
        payload: UpdateUser,
    ) -> Result<User, RepoError> {
        let current = self.get_by_id(id)?;
        if payload.is_admin != current.is_admin {
            if !crate::deployment::current().allows_admin_promotion() {
                return Err(RepoError::BadInput(
                    "admin promotion is disabled in this deployment mode".into(),
                ));
            }
            if !payload.is_admin {
                if actor.id == id {
                    return Err(RepoError::BadInput(
                        "You cannot remove your own administrator rights.".into(),
                    ));
                }
                if self.admin_count()? <= 1 {
                    return Err(RepoError::Duplicate(LAST_ADMIN_MESSAGE.into()));
                }
            }
        }

        self.update_without_password(
            actor,
            &UpdateUser {
                id: Some(id),
                ..payload
            },
        )?;
        self.get_by_id(id)
    }

    /// Let a user change their own display name and email (`PUT /api/auth/me`).
    ///
    /// Username and admin flag are pinned to the stored values. Changing the email
    /// requires `email_changes_allowed` (false where emails must be verified) and,
    /// when the account has a password, the correct `current_password`.
    pub fn update_own_profile(
        &self,
        actor: &User,
        change: ProfileUpdate,
        email_changes_allowed: bool,
    ) -> Result<User, RepoError> {
        // Username lookup bypasses the cache, which drops `password_hash`.
        let stored = self
            .state
            .repo_read()
            .get_user_by_username(&actor.username)?
            .ok_or(RepoError::NotFound)?;

        let name = change.name.trim().to_string();
        let email = change.email.trim().to_lowercase();
        if email.is_empty() {
            return Err(RepoError::BadInput("Email is required".into()));
        }

        if email != stored.email.trim().to_lowercase() {
            if !email_changes_allowed {
                return Err(RepoError::BadInput(
                    "Email changes are not available in this deployment; contact your administrator."
                        .into(),
                ));
            }
            if let Some(hash) = stored.password_hash.as_deref() {
                let password = change
                    .current_password
                    .as_deref()
                    .filter(|p| !p.is_empty())
                    .ok_or_else(|| {
                        RepoError::BadInput(
                            "Current password is required to change your email".into(),
                        )
                    })?;
                if !verify_password(password, hash).unwrap_or(false) {
                    return Err(RepoError::BadInput("Current password is incorrect".into()));
                }
            }
        }

        self.update_without_password(
            &stored,
            &UpdateUser {
                id: Some(stored.id),
                username: stored.username.clone(),
                name,
                email,
                is_admin: stored.is_admin,
            },
        )?;
        self.get_by_id(stored.id)
    }

    /// Set another user's password as a site administrator (no current password).
    ///
    /// Signs the target out of every session unless they are the actor.
    pub fn admin_set_password(
        &self,
        actor: &User,
        id: i32,
        new_password: &str,
        confirm_password: &str,
    ) -> Result<(), RepoError> {
        if new_password != confirm_password {
            return Err(RepoError::BadInput("New passwords do not match".into()));
        }
        let target = self.get_by_id(id)?;

        {
            let mut repo = self.state.repo_write();
            admin_set_user_password(&mut *repo, id, new_password).map_err(|err| match err {
                AuthError::PasswordPolicy(msg) => RepoError::BadInput(msg),
                AuthError::Repo(err) => err,
                other => RepoError::BadInput(other.to_string()),
            })?;
            if id != actor.id {
                let _ = repo.delete_user_sessions(id);
            }
        }

        self.audit_password_set(actor, &target);
        Ok(())
    }

    fn admin_count(&self) -> Result<usize, RepoError> {
        Ok(self
            .state
            .repo_read()
            .get_users_all()?
            .iter()
            .filter(|u| u.is_admin)
            .count())
    }

    fn audit_password_set(&self, actor: &User, target: &User) {
        if let Ok(mut conn) = self.audit_conn() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::log_custom(
                conn.as_mut(),
                &ctx,
                ActionType::Update,
                EntityType::User,
                Some(target.id),
                None,
                None,
                None,
                Some(format!(
                    "Password set by administrator for {}",
                    target.username
                )),
            ) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "audit: failed to log password set for user {}: {_err}",
                    target.id
                );
            }
        }
    }

    fn ensure_username_namespace_available(
        &self,
        username: &str,
        exclude_user_id: Option<i32>,
    ) -> Result<(), RepoError> {
        let repo = self.state.repo_read();
        ensure_namespace_segment_available(
            &*repo,
            username,
            NamespaceAvailabilityOptions {
                exclude_user_id,
                exclude_group_id: None,
            },
        )
    }
}

impl AuditLog for UserService<'_> {
    fn app_state(&self) -> &AppState<DieselCachedRepo> {
        self.state
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
        assert!(stored
            .password_hash
            .as_deref()
            .is_some_and(|hash| hash.starts_with("$argon2")));
        assert_ne!(
            stored.password_hash.as_deref(),
            Some("Skyline!Current_2026")
        );
    }

    #[test]
    fn create_rejects_username_taken_by_other_user() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, sample_user(1, "alice"));
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let err = service
            .create(
                &actor(),
                UserCreateRequest {
                    username: "alice".into(),
                    name: "Alice Two".into(),
                    email: "alice2@example.com".into(),
                    password: "Turbine!Signal_2026".into(),
                    is_admin: false,
                },
            )
            .unwrap_err();

        assert!(matches!(
            err,
            RepoError::Duplicate(message)
                if message == crate::namespaces::TAKEN_NAMESPACE_MESSAGE
        ));
    }

    #[test]
    fn create_rejects_username_taken_by_group_namespace() {
        let mut repo = DieselRepoMock::default();
        repo.groups.insert(
            1,
            crate::models::Group {
                id: 1,
                name: "FlightSystems".into(),
                slug: "flightsystems".into(),
                description: None,
                owner_id: Some(1),
                created_at: chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                updated_at: chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
            },
        );
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let err = service
            .create(
                &actor(),
                UserCreateRequest {
                    username: "flightsystems".into(),
                    name: "Pilot".into(),
                    email: "pilot@example.com".into(),
                    password: "Turbine!Signal_2026".into(),
                    is_admin: false,
                },
            )
            .unwrap_err();

        assert!(matches!(
            err,
            RepoError::Duplicate(message)
                if message == crate::namespaces::TAKEN_NAMESPACE_MESSAGE
        ));
    }

    #[test]
    fn update_rejects_username_taken_by_other_user() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, sample_user(1, "alice"));
        repo.users.insert(2, sample_user(2, "bob"));
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let mut admin = sample_user(1, "alice");
        admin.is_admin = true;

        let err = service
            .update_without_password(
                &admin,
                &UpdateUser {
                    id: Some(2),
                    username: "alice".into(),
                    name: "Bob".into(),
                    email: "bob@example.com".into(),
                    is_admin: false,
                },
            )
            .unwrap_err();

        assert!(matches!(
            err,
            RepoError::Duplicate(message)
                if message == crate::namespaces::TAKEN_NAMESPACE_MESSAGE
        ));
    }

    #[test]
    fn update_rejects_username_taken_by_group_namespace() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, sample_user(1, "alice"));
        repo.users.insert(2, sample_user(2, "bob"));
        repo.groups.insert(
            3,
            crate::models::Group {
                id: 3,
                name: "MissionOps".into(),
                slug: "missionops".into(),
                description: None,
                owner_id: Some(1),
                created_at: chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                updated_at: chrono::NaiveDate::from_ymd_opt(2024, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
            },
        );
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let mut admin = sample_user(1, "alice");
        admin.is_admin = true;

        let err = service
            .update_without_password(
                &admin,
                &UpdateUser {
                    id: Some(2),
                    username: "missionops".into(),
                    name: "Bob".into(),
                    email: "bob@example.com".into(),
                    is_admin: false,
                },
            )
            .unwrap_err();

        assert!(matches!(
            err,
            RepoError::Duplicate(message)
                if message == crate::namespaces::TAKEN_NAMESPACE_MESSAGE
        ));
    }

    #[test]
    fn update_allows_keeping_current_username() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, sample_user(1, "alice"));
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let updated = service
            .update_without_password(
                &sample_user(1, "alice"),
                &UpdateUser {
                    id: Some(1),
                    username: "alice".into(),
                    name: "Alice Updated".into(),
                    email: "alice.updated@example.com".into(),
                    is_admin: false,
                },
            )
            .unwrap();

        assert!(updated);
        let stored = service.get_by_id(1).unwrap();
        assert_eq!(stored.username, "alice");
        assert_eq!(stored.name, "Alice Updated");
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

    fn admin_user(id: i32, username: &str) -> User {
        let mut user = sample_user(id, username);
        user.is_admin = true;
        user
    }

    #[test]
    fn admin_update_refuses_to_demote_last_admin() {
        crate::deployment::install_test_server_mode();
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, admin_user(1, "root"));
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        let result = service.admin_update(
            &actor(),
            1,
            UpdateUser {
                id: None,
                username: "root".into(),
                name: "Root".into(),
                email: "root@example.com".into(),
                is_admin: false,
            },
        );
        assert!(matches!(result, Err(RepoError::Duplicate(msg)) if msg == LAST_ADMIN_MESSAGE));
        assert!(service.get_by_id(1).unwrap().is_admin);
    }

    #[test]
    fn admin_update_refuses_self_demotion() {
        crate::deployment::install_test_server_mode();
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, admin_user(1, "root"));
        repo.users.insert(2, admin_user(2, "second"));
        let state = state_with_repo(repo);
        let service = UserService::new(&state);
        let me = service.get_by_id(1).unwrap();

        let result = service.admin_update(
            &me,
            1,
            UpdateUser {
                id: None,
                username: "root".into(),
                name: "Root".into(),
                email: "root@example.com".into(),
                is_admin: false,
            },
        );
        assert!(matches!(result, Err(RepoError::BadInput(_))));
    }

    #[test]
    fn delete_refuses_self_and_last_admin() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, admin_user(1, "root"));
        let state = state_with_repo(repo);
        let service = UserService::new(&state);
        let me = service.get_by_id(1).unwrap();

        assert!(matches!(
            service.delete(&me, 1),
            Err(RepoError::BadInput(_))
        ));
        assert!(matches!(
            service.delete(&actor(), 1),
            Err(RepoError::Duplicate(_))
        ));
        assert!(service.get_by_id(1).is_ok());
    }

    #[test]
    fn admin_set_password_rejects_mismatch_and_policy() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(1, sample_user(1, "bob"));
        let state = state_with_repo(repo);
        let service = UserService::new(&state);

        assert!(matches!(
            service.admin_set_password(&actor(), 1, "Orbit!Delta_2026", "Other!Delta_2026"),
            Err(RepoError::BadInput(msg)) if msg == "New passwords do not match"
        ));
        assert!(matches!(
            service.admin_set_password(&actor(), 1, "short", "short"),
            Err(RepoError::BadInput(_))
        ));
        service
            .admin_set_password(&actor(), 1, "Orbit!Delta_2026", "Orbit!Delta_2026")
            .unwrap();
        let hash = service.get_by_id(1).unwrap().password_hash.unwrap();
        assert!(crate::auth::password::verify_password("Orbit!Delta_2026", &hash).unwrap());
    }

    fn profile_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();
        let mut me =
            DieselRepoMock::make_user(5, "eng_jones", &hash_password("Orbit!Delta_2026").unwrap());
        me.name = "Mike Jones".into();
        me.email = "mike@example.com".into();
        repo.users.insert(5, me);
        let mut other = DieselRepoMock::make_user(6, "dr_smith", "hash");
        other.email = "sarah@example.com".into();
        repo.users.insert(6, other);
        repo
    }

    fn profile(name: &str, email: &str, password: Option<&str>) -> ProfileUpdate {
        ProfileUpdate {
            name: name.into(),
            email: email.into(),
            current_password: password.map(str::to_string),
        }
    }

    #[test]
    fn update_own_profile_changes_name_without_password() {
        let state = state_with_repo(profile_repo());
        let service = UserService::new(&state);
        let me = service.get_by_id(5).unwrap();

        let updated = service
            .update_own_profile(
                &me,
                profile("  Michael Jones ", "MIKE@example.com", None),
                false,
            )
            .unwrap();
        assert_eq!(updated.name, "Michael Jones");
        assert_eq!(updated.email, "mike@example.com");
        assert_eq!(updated.username, "eng_jones");
        assert!(!updated.is_admin);
    }

    #[test]
    fn update_own_profile_email_needs_current_password() {
        let state = state_with_repo(profile_repo());
        let service = UserService::new(&state);
        let me = service.get_by_id(5).unwrap();

        let missing =
            service.update_own_profile(&me, profile("Mike Jones", "new@example.com", None), true);
        assert!(matches!(missing, Err(RepoError::BadInput(msg)) if msg.contains("required")));
        let wrong = service.update_own_profile(
            &me,
            profile("Mike Jones", "new@example.com", Some("nope")),
            true,
        );
        assert!(
            matches!(wrong, Err(RepoError::BadInput(msg)) if msg == "Current password is incorrect")
        );

        let ok = service
            .update_own_profile(
                &me,
                profile("Mike Jones", "new@example.com", Some("Orbit!Delta_2026")),
                true,
            )
            .unwrap();
        assert_eq!(ok.email, "new@example.com");
    }

    #[test]
    fn update_own_profile_sso_only_account_changes_email_without_password() {
        let mut repo = profile_repo();
        repo.users.get_mut(&5).unwrap().password_hash = None;
        let state = state_with_repo(repo);
        let service = UserService::new(&state);
        let me = service.get_by_id(5).unwrap();

        let ok = service
            .update_own_profile(&me, profile("Mike Jones", "sso@example.com", None), true)
            .unwrap();
        assert_eq!(ok.email, "sso@example.com");
    }

    #[test]
    fn update_own_profile_rejects_email_change_when_disallowed_and_bad_input() {
        let state = state_with_repo(profile_repo());
        let service = UserService::new(&state);
        let me = service.get_by_id(5).unwrap();

        let blocked = service.update_own_profile(
            &me,
            profile("Mike Jones", "new@example.com", Some("Orbit!Delta_2026")),
            false,
        );
        assert!(matches!(blocked, Err(RepoError::BadInput(msg)) if msg.contains("not available")));
        assert!(matches!(
            service.update_own_profile(&me, profile("Mike Jones", "  ", None), true),
            Err(RepoError::BadInput(_))
        ));
        assert!(matches!(
            service.update_own_profile(&me, profile("M", "mike@example.com", None), true),
            Err(RepoError::BadInput(_))
        ));
        let taken = service.update_own_profile(
            &me,
            profile("Mike Jones", "sarah@example.com", Some("Orbit!Delta_2026")),
            true,
        );
        assert!(matches!(taken, Err(RepoError::Duplicate(_))));
        assert_eq!(service.get_by_id(5).unwrap().email, "mike@example.com");
    }
}

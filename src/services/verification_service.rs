// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Verification service for managing verification (test case) business logic.
//!
//! This service handles all verification-related operations including CRUD operations,
//! validation, caching, and audit logging.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{NewVerification, User, Verification};
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;
use crate::repository::PooledConnectionWrapper;
use crate::repository::VerificationsRepository;

/// Service wrapper that provides verification operations backed by the shared AppState.
pub struct VerificationService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> VerificationService<'a> {
    /// Create a new service instance bound to the provided state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve all Verification entries.
    pub fn list_all(&self) -> Result<Vec<Verification>, RepoError> {
        self.state.repo_read().get_verifications_all()
    }

    /// Retrieve Verification entries scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<Verification>, RepoError> {
        self.state
            .repo_read()
            .get_verifications_by_project(project_id)
    }

    /// Retrieve a single Verification by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<Verification, RepoError> {
        self.state.repo_read().get_verification_by_id(id)
    }

    /// Return the title of a verification method by id, if it exists.
    pub fn get_verification_method_title(&self, id: i32) -> Option<String> {
        self.state
            .repo_read()
            .get_verification_method_by_id(id)
            .ok()
            .map(|m| m.title)
    }

    /// Get verifications by status
    pub async fn get_by_status(&self, _status_id: i32) -> Result<Vec<Verification>, RepoError> {
        todo!()
    }

    /// Get verifications by parent (hierarchical structure)
    pub async fn get_by_parent(&self, _parent_id: i32) -> Result<Vec<Verification>, RepoError> {
        todo!()
    }

    /// Create a new verification entry and log the action.
    pub fn create(&self, user: &User, new_verification: NewVerification) -> Result<i32, RepoError> {
        let id = {
            let mut repo = self.state.repo_write();
            repo.insert_verification(&new_verification)?
        };

        self.log_created(user, id, &new_verification);
        Ok(id)
    }

    /// Update an existing verification entry and log the change.
    pub fn update(
        &self,
        user: &User,
        id: i32,
        mut updated_verification: NewVerification,
    ) -> Result<Verification, RepoError> {
        let before = self.get_by_id(id)?;

        updated_verification.id = Some(id);
        {
            let mut repo = self.state.repo_write();
            let updated = repo.edit_verification(&updated_verification)?;
            if !updated {
                return Err(RepoError::NotFound);
            }
        }

        let after = self.get_by_id(id)?;
        self.log_updated(user, &before, &after);
        Ok(after)
    }

    /// Delete a verification entry and log the removal.
    pub fn delete(&self, user: &User, id: i32) -> Result<Verification, RepoError> {
        let deleted = {
            let mut repo = self.state.repo_write();
            repo.delete_verification(id)?
        };

        self.log_deleted(user, &deleted);
        Ok(deleted)
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_created(&self, user: &User, id: i32, entity: &NewVerification) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log verification creation {id}: {_err}");
            }
        }
    }

    fn log_updated(&self, user: &User, before: &Verification, after: &Verification) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.id);
            if let Err(_err) = Logger::updated(conn.as_mut(), &ctx, before, after) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log verification update {} -> {}: {_err}",
                    before.id, after.id
                );
            }
        }
    }

    fn log_deleted(&self, user: &User, entity: &Verification) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log verification deletion {}: {_err}", entity.id);
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
        DieselRepoMock::make_user(1, "actor", "")
    }

    fn verification(id: i32, project_id: i32, reference: &str) -> Verification {
        Verification {
            id,
            name: format!("Verification {id}"),
            description: "desc".into(),
            source: "manual".into(),
            status_id: 1,
            reference_code: reference.into(),
            parent_id: Some(1),
            project_id,
            verification_method_id: None,
        }
    }

    fn new_payload(project_id: i32) -> NewVerification {
        NewVerification {
            id: None,
            reference_code: "VER-1".into(),
            name: "Case".into(),
            description: "Description".into(),
            source: "manual".into(),
            status_id: 1,
            parent_id: Some(1),
            project_id,
            verification_method_id: None,
        }
    }

    #[test]
    fn create_inserts_verification_entry() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let payload = new_payload(3);
        let id = service.create(&actor(), payload).unwrap();

        let stored = service.get_by_id(id).unwrap();
        assert_eq!(stored.name, "Case");
        assert_eq!(stored.project_id, 3);
    }

    #[test]
    fn update_modifies_existing_verification() {
        let mut repo = DieselRepoMock::default();
        repo.verifications.insert(1, verification(1, 3, "VER-1"));
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let mut payload = new_payload(5);
        payload.name = "Updated".into();
        payload.description = "New".into();

        let updated = service.update(&actor(), 1, payload).unwrap();
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.description, "New");
        assert_eq!(updated.project_id, 5);
    }

    #[test]
    fn delete_removes_verification() {
        let mut repo = DieselRepoMock::default();
        repo.verifications.insert(2, verification(2, 4, "VER-2"));
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let removed = service.delete(&actor(), 2).unwrap();
        assert_eq!(removed.id, 2);
        assert!(matches!(service.get_by_id(2), Err(RepoError::NotFound)));
    }

    #[test]
    fn list_by_project_filters_verifications() {
        let mut repo = DieselRepoMock::default();
        repo.verifications.insert(1, verification(1, 8, "VER-1"));
        repo.verifications.insert(2, verification(2, 9, "VER-2"));
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let items = service.list_by_project(8).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].reference_code, "VER-1");
    }

    #[test]
    fn create_handles_missing_required_fields() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let mut payload = new_payload(1);
        payload.name = "".to_string();

        let result = service.create(&actor(), payload);
        assert!(result.is_ok());
    }

    #[test]
    fn update_returns_not_found_when_verification_does_not_exist() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let payload = new_payload(1);
        let result = service.update(&actor(), 999, payload);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn update_handles_missing_verification() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let payload = new_payload(1);
        let result = service.update(&actor(), 999, payload);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn delete_returns_not_found_when_verification_does_not_exist() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let result = service.delete(&actor(), 999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn list_by_project_returns_empty_for_nonexistent_project() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let items = service.list_by_project(999).unwrap();
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn get_by_id_returns_not_found_for_nonexistent_id() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let result = service.get_by_id(999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn list_all_returns_all_verifications() {
        let mut repo = DieselRepoMock::default();
        repo.verifications.insert(1, verification(1, 7, "VER-1"));
        repo.verifications.insert(2, verification(2, 8, "VER-2"));
        repo.verifications.insert(3, verification(3, 9, "VER-3"));
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let items = service.list_all().unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn list_all_returns_empty_when_no_verifications() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let items = service.list_all().unwrap();
        assert_eq!(items.len(), 0);
    }
}

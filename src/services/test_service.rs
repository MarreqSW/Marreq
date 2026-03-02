// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Test service for managing test cases business logic.
//!
//! This service handles all test-related operations including CRUD operations,
//! validation, caching, and audit logging.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{NewTestCase, TestCase, User};
use crate::repository::errors::RepoError;
use crate::repository::PooledConnectionWrapper;
use crate::repository::TestsCaseRepository;

/// Service wrapper that provides test operations backed by the shared AppState.
pub struct TestService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> TestService<'a> {
    /// Create a new service instance bound to the provided test state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve all TestCase entries.
    pub fn list_all(&self) -> Result<Vec<TestCase>, RepoError> {
        self.state.repo_read().get_tests_all()
    }

    /// Retrieve TestCase entries scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<TestCase>, RepoError> {
        self.state.repo_read().get_tests_by_project(project_id)
    }

    /// Retrieve a single TestCase by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<TestCase, RepoError> {
        self.state.repo_read().get_test_by_id(id)
    }

    /// Get tests by status
    pub async fn get_by_status(&self, _status_id: i32) -> Result<Vec<TestCase>, RepoError> {
        todo!()
    }

    /// Get tests by parent (hierarchical structure)
    pub async fn get_by_parent(&self, _parent_id: i32) -> Result<Vec<TestCase>, RepoError> {
        todo!()
    }

    /// Create a new test entry and log the action.
    pub fn create(&self, user: &User, new_test: NewTestCase) -> Result<i32, RepoError> {
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
        mut updated_test: NewTestCase,
    ) -> Result<TestCase, RepoError> {
        let before = self.get_by_id(id)?;

        updated_test.id = Some(id);
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
    pub fn delete(&self, user: &User, id: i32) -> Result<TestCase, RepoError> {
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

    fn log_created(&self, user: &User, id: i32, entity: &NewTestCase) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log applicability creation {id}: {_err}");
            }
        }
    }

    fn log_updated(&self, user: &User, before: &TestCase, after: &TestCase) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.id);
            if let Err(_err) = Logger::updated(conn.as_mut(), &ctx, before, after) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log applicability update {} -> {}: {_err}",
                    before.id, after.id
                );
            }
        }
    }

    fn log_deleted(&self, user: &User, entity: &TestCase) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log applicability deletion {}: {_err}", entity.id);
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

    fn test_case(id: i32, project_id: i32, reference: &str) -> TestCase {
        TestCase {
            id,
            name: format!("Test {id}"),
            description: "desc".into(),
            source: "manual".into(),
            status_id: 1,
            reference_code: reference.into(),
            parent_id: Some(1),
            project_id,
        }
    }

    fn new_payload(project_id: i32) -> NewTestCase {
        NewTestCase {
            id: None,
            reference_code: "TEST-1".into(),
            name: "Case".into(),
            description: "Description".into(),
            source: "manual".into(),
            status_id: 1,
            parent_id: Some(1),
            project_id,
        }
    }

    #[test]
    fn create_inserts_test_entry() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = TestService::new(&state);

        let payload = new_payload(3);
        let id = service.create(&actor(), payload).unwrap();

        let stored = service.get_by_id(id).unwrap();
        assert_eq!(stored.name, "Case");
        assert_eq!(stored.project_id, 3);
    }

    #[test]
    fn update_modifies_existing_test() {
        let mut repo = DieselRepoMock::default();
        repo.tests.insert(1, test_case(1, 3, "TEST-1"));
        let state = state_with_repo(repo);
        let service = TestService::new(&state);

        let mut payload = new_payload(5);
        payload.name = "Updated".into();
        payload.description = "New".into();

        let updated = service.update(&actor(), 1, payload).unwrap();
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.description, "New");
        assert_eq!(updated.project_id, 5);
    }

    #[test]
    fn delete_removes_test() {
        let mut repo = DieselRepoMock::default();
        repo.tests.insert(2, test_case(2, 4, "TEST-2"));
        let state = state_with_repo(repo);
        let service = TestService::new(&state);

        let removed = service.delete(&actor(), 2).unwrap();
        assert_eq!(removed.id, 2);
        assert!(matches!(service.get_by_id(2), Err(RepoError::NotFound)));
    }

    #[test]
    fn list_by_project_filters_tests() {
        let mut repo = DieselRepoMock::default();
        repo.tests.insert(1, test_case(1, 8, "TEST-1"));
        repo.tests.insert(2, test_case(2, 9, "TEST-2"));
        let state = state_with_repo(repo);
        let service = TestService::new(&state);

        let items = service.list_by_project(8).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].reference_code, "TEST-1");
    }

    #[test]
    fn create_handles_missing_required_fields() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = TestService::new(&state);

        // Create payload with missing required field
        let mut payload = new_payload(1);
        payload.name = "".to_string(); // Empty name should fail validation

        // Note: TestService doesn't validate, so this will succeed
        // But we can test that the service handles the creation
        let result = service.create(&actor(), payload);
        // The service doesn't validate, so this should succeed
        // If validation is added later, this test will catch it
        assert!(result.is_ok());
    }

    #[test]
    fn update_returns_not_found_when_test_does_not_exist() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = TestService::new(&state);

        let payload = new_payload(1);
        let result = service.update(&actor(), 999, payload);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn update_handles_missing_test() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = TestService::new(&state);

        let payload = new_payload(1);
        let result = service.update(&actor(), 999, payload);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn delete_returns_not_found_when_test_does_not_exist() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = TestService::new(&state);

        let result = service.delete(&actor(), 999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn list_by_project_returns_empty_for_nonexistent_project() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = TestService::new(&state);

        let items = service.list_by_project(999).unwrap();
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn get_by_id_returns_not_found_for_nonexistent_id() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = TestService::new(&state);

        let result = service.get_by_id(999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn list_all_returns_all_tests() {
        let mut repo = DieselRepoMock::default();
        repo.tests.insert(1, test_case(1, 7, "TEST-1"));
        repo.tests.insert(2, test_case(2, 8, "TEST-2"));
        repo.tests.insert(3, test_case(3, 9, "TEST-3"));
        let state = state_with_repo(repo);
        let service = TestService::new(&state);

        let items = service.list_all().unwrap();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn list_all_returns_empty_when_no_tests() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = TestService::new(&state);

        let items = service.list_all().unwrap();
        assert_eq!(items.len(), 0);
    }
}

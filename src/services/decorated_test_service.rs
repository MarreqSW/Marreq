//! Service providing test related operations with decorated responses.
//!
//! The service wraps core repository functionality with presentation-friendly
//! formatting so handlers can focus on HTTP concerns while still reusing
//! existing business logic.

use super::{RequirementService, StatusService, TestService};
use crate::app::{AppState, DieselCachedRepo};
use crate::models::{DecoratedTest, NewTest, Test, User};
use crate::repository::errors::RepoError;

/// High level operations for tests backed by the shared [`AppState`].
pub struct DecoratedTestService<'a> {
    test_service: TestService<'a>,
    status_service: StatusService<'a>,
    requirement_service: RequirementService<'a>,
}

impl<'a> DecoratedTestService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self {
            test_service: TestService::new(state),
            status_service: StatusService::new(state),
            requirement_service: RequirementService::new(state),
        }
    }

    /// Retrieve all tests.
    pub fn list_all(&self) -> Result<Vec<DecoratedTest>, RepoError> {
        self.decorate_vec(self.test_service.list_all()?)
    }

    /// Retrieve tests scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<DecoratedTest>, RepoError> {
        self.decorate_vec(self.test_service.list_by_project(project_id)?)
    }

    /// Retrieve a single test by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<DecoratedTest, RepoError> {
        let test = self.test_service.get_by_id(id)?;
        self.decorate(&test)
    }

    /// Retrieve child tests for a given parent identifier.
    pub fn get_by_parent_id(&self, parent_id: i32) -> Result<Vec<DecoratedTest>, RepoError> {
        let children: Vec<Test> = self
            .test_service
            .list_all()?
            .into_iter()
            .filter(|t| t.test_parent == parent_id)
            .collect();
        self.decorate_vec(children)
    }

    /// Retrieve tests linked to a requirement and decorate the result.
    pub fn get_linked_to_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<DecoratedTest>, RepoError> {
        self.decorate_vec(self.requirement_service.get_linked_tests(requirement_id)?)
    }

    /// Create a new test entry and log the action.
    pub fn create(&self, actor: &User, payload: NewTest) -> Result<i32, RepoError> {
        self.test_service.create(actor, payload)
    }

    /// Update an existing test entry and log the change.
    pub fn update(
        &self,
        actor: &User,
        id: i32,
        payload: NewTest,
    ) -> Result<Test, RepoError> {
        self.test_service.update(actor, id, payload)
    }

    /// Delete a test entry and log the removal.
    pub fn delete(&self, actor: &User, id: i32) -> Result<Test, RepoError> {
        self.test_service.delete(actor, id)
    }

    fn decorate_vec(&self, tests: Vec<Test>) -> Result<Vec<DecoratedTest>, RepoError> {
        tests.iter().map(|t| self.decorate(t)).collect()
    }

    fn decorate(&self, test: &Test) -> Result<DecoratedTest, RepoError> {
        let status = self
            .status_service
            .get_test_status(test.test_status)
            .map(|s| s.test_st_title)
            .unwrap_or_else(|_| format!("Unknown Status ({})", test.test_status));

        let parent_title = if test.test_parent != 0 {
            match self.test_service.get_by_id(test.test_parent) {
                Ok(parent) => parent.test_name,
                Err(_) => String::new(),
            }
        } else {
            String::new()
        };

        Ok(DecoratedTest {
            test_id: test.test_id,
            test_reference: test.test_reference.clone(),
            test_name: test.test_name.clone(),
            test_description: test.test_description.clone(),
            test_source: test.test_source.clone(),
            test_status: status,
            test_status_id: test.test_status,
            test_parent_id: test.test_parent,
            test_parent_title: parent_title,
            project_id: test.project_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Matrix, Requirement, TestStatus};
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::sync::{Arc, RwLock};

    fn ts() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn state_with_repo(repo: DieselRepoMock) -> AppState<DieselCachedRepo> {
        AppState {
            repo: Arc::new(RwLock::new(DieselCachedRepo::new(repo, 0))),
        }
    }

    fn make_test(id: i32, parent: i32, status: i32) -> Test {
        Test {
            test_id: id,
            test_name: format!("Test {id}"),
            test_description: "desc".into(),
            test_source: "manual".into(),
            test_status: status,
            test_reference: format!("TEST-{id}"),
            test_parent: parent,
            project_id: 99,
        }
    }

    #[test]
    fn decorate_returns_status_and_parent_name() {
        let mut repo = DieselRepoMock::default();
        repo.test_statuses.insert(
            1,
            TestStatus {
                test_st_id: 1,
                test_st_title: "Open".into(),
                test_st_description: String::new(),
                test_st_short_name: String::new(),
            },
        );
        repo.tests.insert(5, make_test(5, 0, 1));
        repo.tests.insert(6, make_test(6, 5, 1));

        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let decorated = service.get_by_id(6).unwrap();
        assert_eq!(decorated.test_status, "Open");
        assert_eq!(decorated.test_parent_title, "Test 5");
    }

    #[test]
    fn decorate_handles_missing_status_and_parent() {
        let mut repo = DieselRepoMock::default();
        repo.tests.insert(1, make_test(1, 999, 77));

        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let decorated = service.get_by_id(1).unwrap();
        assert_eq!(decorated.test_status, "Unknown Status (77)");
        assert_eq!(decorated.test_parent_title, "");
    }

    #[test]
    fn get_linked_to_requirement_decorates_results() {
        let mut repo = DieselRepoMock::default();
        repo.test_statuses.insert(
            1,
            TestStatus {
                test_st_id: 1,
                test_st_title: "Ready".into(),
                test_st_description: String::new(),
                test_st_short_name: String::new(),
            },
        );
        repo.tests.insert(10, make_test(10, 0, 1));
        repo.requirements.insert(
            3,
            Requirement {
                req_id: 3,
                req_title: "Req".into(),
                req_description: String::new(),
                req_verification: 0,
                req_current_status: 0,
                req_author: 0,
                req_reviewer: 0,
                req_link: String::new(),
                req_reference: String::new(),
                req_category: 0,
                req_parent: 0,
                req_creation_date: ts(),
                req_update_date: ts(),
                req_deadline_date: ts(),
                req_applicability: 0,
                req_justification: None,
                project_id: 99,
            },
        );
        repo.matrices.push(Matrix {
            matrix_req_id: 3,
            matrix_test_id: 10,
            matrix_creation_date: ts(),
            project_id: 99,
        });

        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let items = service.get_linked_to_requirement(3).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].test_status, "Ready");
        assert_eq!(items[0].test_reference, "TEST-10");
    }
}

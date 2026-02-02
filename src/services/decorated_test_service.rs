//! Service providing test related operations with decorated responses.
//!
//! The service wraps core repository functionality with presentation-friendly
//! formatting so handlers can focus on HTTP concerns while still reusing
//! existing business logic.

use super::{RequirementService, StatusService, TestService};
use crate::app::{AppState, DieselCachedRepo};
use crate::models::{DecoratedTestCase, NewTestCase, TestCase, User};
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
    pub fn list_all(&self) -> Result<Vec<DecoratedTestCase>, RepoError> {
        self.decorate_vec(self.test_service.list_all()?)
    }

    /// Retrieve tests scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<DecoratedTestCase>, RepoError> {
        self.decorate_vec(self.test_service.list_by_project(project_id)?)
    }

    /// Retrieve a single test by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<DecoratedTestCase, RepoError> {
        let test = self.test_service.get_by_id(id)?;
        self.decorate(&test)
    }

    /// Retrieve child tests for a given parent identifier.
    pub fn get_by_parent_id(&self, parent_id: i32) -> Result<Vec<DecoratedTestCase>, RepoError> {
        let children: Vec<TestCase> = self
            .test_service
            .list_all()?
            .into_iter()
            .filter(|t| t.parent_id == Some(parent_id))
            .collect();
        self.decorate_vec(children)
    }

    /// Retrieve tests linked to a requirement and decorate the result.
    pub fn get_linked_to_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<DecoratedTestCase>, RepoError> {
        self.decorate_vec(self.requirement_service.get_linked_tests(requirement_id)?)
    }

    /// Create a new test entry and log the action.
    pub fn create(&self, actor: &User, payload: NewTestCase) -> Result<i32, RepoError> {
        self.test_service.create(actor, payload)
    }

    /// Update an existing test entry and log the change.
    pub fn update(
        &self,
        actor: &User,
        id: i32,
        payload: NewTestCase,
    ) -> Result<TestCase, RepoError> {
        self.test_service.update(actor, id, payload)
    }

    /// Delete a test entry and log the removal.
    pub fn delete(&self, actor: &User, id: i32) -> Result<TestCase, RepoError> {
        self.test_service.delete(actor, id)
    }

    fn decorate_vec(&self, tests: Vec<TestCase>) -> Result<Vec<DecoratedTestCase>, RepoError> {
        tests.iter().map(|t| self.decorate(t)).collect()
    }

    fn decorate(&self, test: &TestCase) -> Result<DecoratedTestCase, RepoError> {
        let status = self
            .status_service
            .get_test_status(test.status_id)
            .map(|s| s.title)
            .unwrap_or_else(|_| format!("Unknown Status ({})", test.status_id));

        let parent_title = if let Some(parent_id) = test.parent_id {
            match self.test_service.get_by_id(parent_id) {
                Ok(parent_test) => parent_test.name,
                Err(_) => "[Deleted Parent]".to_string(),
            }
        } else {
            String::new()
        };

        Ok(DecoratedTestCase {
            id: test.id,
            reference_code: test.reference_code.clone(),
            name: test.name.clone(),
            description: test.description.clone(),
            source: test.source.clone(),
            status_id: status,
            test_status_id: test.status_id,
            test_parent_id: test.parent_id,
            test_parent_title: parent_title,
            project_id: test.project_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MatrixLink, Requirement, TestCase, TestStatus};
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

    fn make_test(id: i32, parent: i32, status: i32) -> TestCase {
        TestCase {
            id: id,
            name: format!("Test {id}"),
            description: "desc".into(),
            source: "manual".into(),
            status_id: status,
            reference_code: format!("TEST-{id}"),
            parent_id: Some(parent),
            project_id: 99,
        }
    }

    #[test]
    fn decorate_returns_status_and_parent_name() {
        let mut repo = DieselRepoMock::default();
        repo.test_statuses.insert(
            1,
            TestStatus {
                id: 1,
                title: "Open".into(),
                description: String::new(),
                tag: String::new(),
                project_id: 1,
            },
        );
        repo.tests.insert(1, make_test(1, 0, 1));
        repo.tests.insert(5, make_test(5, 0, 1));
        repo.tests.insert(6, make_test(6, 5, 1));

        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let decorated = service.get_by_id(6).unwrap();
        assert_eq!(decorated.status_id, "Open");
        assert_eq!(decorated.test_parent_title, "Test 5");
    }

    #[test]
    fn decorate_handles_missing_status_and_parent() {
        let mut repo = DieselRepoMock::default();
        repo.tests.insert(1, make_test(1, 999, 77));

        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let decorated = service.get_by_id(1).unwrap();
        assert_eq!(decorated.status_id, "Unknown Status (77)");
        assert_eq!(decorated.test_parent_title, "[Deleted Parent]");
    }

    #[test]
    fn get_linked_to_requirement_decorates_results() {
        let mut repo = DieselRepoMock::default();
        repo.test_statuses.insert(
            1,
            TestStatus {
                id: 1,
                title: "Ready".into(),
                description: String::new(),
                tag: String::new(),
                project_id: 1,
            },
        );
        repo.tests.insert(10, make_test(10, 0, 1));
        repo.requirements.insert(
            3,
            Requirement {
                id: 3,
                title: "Req".into(),
                description: String::new(),
                status_id: 0,
                author_id: 0,
                reviewer_id: 0,
                reference_code: String::new(),
                category_id: 0,
                parent_id: None,
                creation_date: ts(),
                update_date: ts(),
                deadline_date: Some(ts()),
                applicability_id: 0,
                justification: None,
                project_id: 99,
            },
        );
        repo.matrices.push(MatrixLink {
            req_id: 3,
            test_id: 10,
            creation_date: ts(),
            project_id: 99,
        });

        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let items = service.get_linked_to_requirement(3).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status_id, "Ready");
        assert_eq!(items[0].reference_code, "TEST-10");
    }

    #[test]
    fn list_all_decorates_all_tests() {
        let mut repo = DieselRepoMock::default();
        repo.test_statuses.insert(
            1,
            TestStatus {
                id: 1,
                title: "Open".into(),
                description: String::new(),
                tag: String::new(),
                project_id: 1,
            },
        );
        repo.tests.insert(1, make_test(1, 0, 1));
        repo.tests.insert(2, make_test(2, 0, 1));

        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let decorated = service.list_all().unwrap();
        assert_eq!(decorated.len(), 2);
        // Order may vary, so check that both IDs are present
        let ids: Vec<i32> = decorated.iter().map(|t| t.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
    }

    #[test]
    fn list_by_project_decorates_filtered_tests() {
        let mut repo = DieselRepoMock::default();
        repo.test_statuses.insert(
            1,
            TestStatus {
                id: 1,
                title: "Open".into(),
                description: String::new(),
                tag: String::new(),
                project_id: 1,
            },
        );
        let mut test1 = make_test(1, 0, 1);
        test1.project_id = 1;
        let mut test2 = make_test(2, 0, 1);
        test2.project_id = 2;

        repo.tests.insert(1, test1);
        repo.tests.insert(2, test2);

        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let decorated = service.list_by_project(1).unwrap();
        assert_eq!(decorated.len(), 1);
        assert_eq!(decorated[0].project_id, 1);
    }

    #[test]
    fn get_by_parent_id_decorates_children() {
        let mut repo = DieselRepoMock::default();
        repo.test_statuses.insert(
            1,
            TestStatus {
                id: 1,
                title: "Open".into(),
                description: String::new(),
                tag: String::new(),
                project_id: 1,
            },
        );
        repo.tests.insert(1, make_test(1, 0, 1));
        repo.tests.insert(2, make_test(2, 1, 1));
        repo.tests.insert(3, make_test(3, 1, 1));

        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let decorated = service.get_by_parent_id(1).unwrap();
        assert_eq!(decorated.len(), 2);
        assert!(decorated.iter().any(|t| t.id == 2));
        assert!(decorated.iter().any(|t| t.id == 3));
    }

    #[test]
    fn get_by_parent_id_returns_empty_when_no_children() {
        let mut repo = DieselRepoMock::default();
        repo.test_statuses.insert(
            1,
            TestStatus {
                id: 1,
                title: "Open".into(),
                description: String::new(),
                tag: String::new(),
                project_id: 1,
            },
        );
        repo.tests.insert(1, make_test(1, 0, 1));

        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let decorated = service.get_by_parent_id(999).unwrap();
        assert_eq!(decorated.len(), 0);
    }

    #[test]
    fn decorate_handles_no_parent() {
        let mut repo = DieselRepoMock::default();
        repo.test_statuses.insert(
            1,
            TestStatus {
                id: 1,
                title: "Open".into(),
                description: String::new(),
                tag: String::new(),
                project_id: 1,
            },
        );
        let mut test = make_test(1, 0, 1);
        test.parent_id = None;

        repo.tests.insert(1, test);

        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let decorated = service.get_by_id(1).unwrap();
        assert_eq!(decorated.test_parent_id, None);
        assert_eq!(decorated.test_parent_title, "");
    }

    #[test]
    fn create_delegates_to_test_service() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let actor = DieselRepoMock::make_user(1, "actor", "");
        let payload = NewTestCase {
            id: None,
            reference_code: "TEST-NEW".into(),
            name: "New Test".into(),
            description: "Description".into(),
            source: "manual".into(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
        };

        let id = service.create(&actor, payload).unwrap();
        assert!(id >= 0);
    }

    #[test]
    fn update_delegates_to_test_service() {
        let mut repo = DieselRepoMock::default();
        repo.tests.insert(1, make_test(1, 0, 1));

        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let actor = DieselRepoMock::make_user(1, "actor", "");
        let payload = NewTestCase {
            id: Some(1),
            reference_code: "TEST-1".into(),
            name: "Updated Test".into(),
            description: "Updated Description".into(),
            source: "automated".into(),
            status_id: 1,
            parent_id: None,
            project_id: 99,
        };

        let updated = service.update(&actor, 1, payload).unwrap();
        assert_eq!(updated.name, "Updated Test");
    }

    #[test]
    fn delete_delegates_to_test_service() {
        let mut repo = DieselRepoMock::default();
        repo.tests.insert(1, make_test(1, 0, 1));

        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let actor = DieselRepoMock::make_user(1, "actor", "");
        let deleted = service.delete(&actor, 1).unwrap();
        assert_eq!(deleted.id, 1);
    }

    #[test]
    fn get_linked_to_requirement_returns_empty_when_no_links() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = DecoratedTestService::new(&state);

        let items = service.get_linked_to_requirement(999).unwrap();
        assert_eq!(items.len(), 0);
    }
}

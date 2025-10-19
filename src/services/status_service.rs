//! Service exposing helpers for requirement and test statuses.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{NewStatus, RequirementStatus, Status, TestStatus};
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;
use crate::validation::{sanitize_string, validate_requirement_status};

/// High level status operations backed by the shared [`AppState`].
pub struct StatusService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> StatusService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve legacy status records (used by the old API representation).
    pub fn list_legacy(&self) -> Result<Vec<Status>, RepoError> {
        self.state.repo_read().get_status_all()
    }

    /// Retrieve requirement statuses.
    pub fn list_requirement_statuses(&self) -> Result<Vec<RequirementStatus>, RepoError> {
        self.state.repo_read().get_requirement_status_all()
    }

    /// Retrieve test statuses.
    pub fn list_test_statuses(&self) -> Result<Vec<TestStatus>, RepoError> {
        self.state.repo_read().get_test_status_all()
    }

    /// Retrieve a single requirement status by identifier.
    pub fn get_requirement_status(&self, id: i32) -> Result<RequirementStatus, RepoError> {
        self.state.repo_read().get_requirement_status_by_id(id)
    }

    pub fn get_status_name(&self, id: i32) -> Result<String, RepoError> {
        let status = self.state.repo_read().get_status_by_id(id)?;
        Ok(status.st_title)
    }

    /// Create a new requirement status entry.
    pub fn create_requirement_status(&self, mut payload: NewStatus) -> Result<i32, RepoError> {
        sanitize_string(&mut payload.req_st_title);
        sanitize_string(&mut payload.req_st_description);
        sanitize_string(&mut payload.req_st_short_name);

        validate_requirement_status(&payload)
            .map_err(|err| RepoError::BadInput(err.to_string()))?;

        let id = {
            let mut repo = self.state.repo_write();
            repo.create_status(&payload)?
        };

        Ok(id)
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

    fn populated_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();
        repo.statuses.insert(
            1,
            Status {
                st_id: 1,
                st_title: "Legacy".into(),
                st_description: "legacy".into(),
                st_short_name: "LEG".into(),
            },
        );
        repo.requirement_statuses.insert(
            2,
            RequirementStatus {
                req_st_id: 2,
                req_st_title: "Draft".into(),
                req_st_description: "draft".into(),
                req_st_short_name: "DRT".into(),
            },
        );
        repo.test_statuses.insert(
            3,
            TestStatus {
                test_st_id: 3,
                test_st_title: "Ready".into(),
                test_st_description: "ready".into(),
                test_st_short_name: "RDY".into(),
            },
        );
        repo
    }

    #[test]
    fn list_methods_forward_to_repository() {
        let repo = populated_repo();
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        assert_eq!(service.list_legacy().unwrap().len(), 1);
        assert_eq!(service.list_requirement_statuses().unwrap().len(), 1);
        assert_eq!(service.list_test_statuses().unwrap().len(), 1);
        assert_eq!(
            service.get_requirement_status(2).unwrap().req_st_title,
            "Draft"
        );
    }

    #[test]
    fn create_requirement_status_trims_input() {
        let repo = populated_repo();
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let payload = NewStatus {
            req_st_title: "  Verified  ".into(),
            req_st_description: "  Description  ".into(),
            req_st_short_name: "  VFD  ".into(),
        };

        let id = service.create_requirement_status(payload).unwrap();

        let repo_guard = state.repo_read();
        let stored = repo_guard.inner_repo().statuses.get(&id).unwrap();
        assert_eq!(stored.st_title, "Verified");
        assert_eq!(stored.st_description, "Description");
        assert_eq!(stored.st_short_name, "VFD");
    }

    #[test]
    fn create_requirement_status_rejects_invalid_title() {
        let repo = populated_repo();
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let payload = NewStatus {
            req_st_title: " ".into(),
            req_st_description: "Desc".into(),
            req_st_short_name: "DRT".into(),
        };

        let err = service.create_requirement_status(payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }
}

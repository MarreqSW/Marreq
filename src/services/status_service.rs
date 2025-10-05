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

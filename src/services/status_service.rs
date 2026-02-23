// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

//! Service exposing helpers for requirement and test statuses.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{NewRequirementStatus, NewTestStatus, RequirementStatus, TestStatus};
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;
use crate::status_enums::{RequirementStatusEnum, TestStatusEnum};
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

    /// Retrieve requirement statuses.
    pub fn list_requirement_statuses(&self) -> Result<Vec<RequirementStatus>, RepoError> {
        self.state.repo_read().get_requirement_status_all()
    }

    /// Retrieve requirement statuses for a specific project.
    pub fn list_requirement_statuses_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<RequirementStatus>, RepoError> {
        let all_statuses = self.state.repo_read().get_requirement_status_all()?;
        Ok(all_statuses
            .into_iter()
            .filter(|s| s.project_id == project_id)
            .collect())
    }

    /// Retrieve test statuses.
    pub fn list_test_statuses(&self) -> Result<Vec<TestStatus>, RepoError> {
        self.state.repo_read().get_test_status_all()
    }

    /// Retrieve test statuses for a specific project.
    pub fn list_test_statuses_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<TestStatus>, RepoError> {
        let all_statuses = self.state.repo_read().get_test_status_all()?;
        Ok(all_statuses
            .into_iter()
            .filter(|s| s.project_id == project_id)
            .collect())
    }

    /// Retrieve a single requirement status by identifier.
    pub fn get_requirement_status(&self, id: i32) -> Result<RequirementStatus, RepoError> {
        self.state.repo_read().get_requirement_status_by_id(id)
    }

    /// Retrieve a single test status by identifier.
    pub fn get_test_status(&self, id: i32) -> Result<TestStatus, RepoError> {
        self.state.repo_read().get_test_status_by_id(id)
    }

    pub fn get_status_name(&self, id: i32) -> Result<String, RepoError> {
        let status = self.state.repo_read().get_requirement_status_by_id(id)?;
        Ok(status.title)
    }

    /// Create a new requirement status entry (user-created; is_system is always false).
    pub fn create_requirement_status(
        &self,
        mut payload: NewRequirementStatus,
    ) -> Result<i32, RepoError> {
        payload.is_system = false;
        self.create_system_requirement_status(payload)
    }

    /// Create a requirement status with explicit is_system (used for default statuses).
    fn create_system_requirement_status(
        &self,
        mut payload: NewRequirementStatus,
    ) -> Result<i32, RepoError> {
        sanitize_string(&mut payload.title);
        sanitize_string(&mut payload.description);
        sanitize_string(&mut payload.tag);

        validate_requirement_status(&payload)
            .map_err(|err| RepoError::BadInput(err.to_string()))?;

        let id = {
            let mut repo = self.state.repo_write();
            repo.create_requirement_status(&payload)?
        };

        Ok(id)
    }

    /// Create a new test status entry (user-created; is_system is always false).
    pub fn create_test_status(&self, mut payload: NewTestStatus) -> Result<i32, RepoError> {
        payload.is_system = false;
        self.create_system_test_status(payload)
    }

    /// Create a test status with explicit is_system (used for default statuses).
    fn create_system_test_status(&self, mut payload: NewTestStatus) -> Result<i32, RepoError> {
        sanitize_string(&mut payload.title);
        sanitize_string(&mut payload.description);
        sanitize_string(&mut payload.tag);

        // Reusing validation logic
        validate_requirement_status(&NewRequirementStatus {
            id: payload.id,
            title: payload.title.clone(),
            description: payload.description.clone(),
            tag: payload.tag.clone(),
            project_id: payload.project_id,
            is_system: payload.is_system,
            tag_color: payload.tag_color.clone(),
        })
        .map_err(|err| RepoError::BadInput(err.to_string()))?;

        let id = {
            let mut repo = self.state.repo_write();
            repo.create_test_status(&payload)?
        };

        Ok(id)
    }

    /// Update a requirement status (title, description, tag). Fails if the status is system.
    pub fn update_requirement_status(
        &self,
        id: i32,
        payload: &NewRequirementStatus,
    ) -> Result<bool, RepoError> {
        let status = self.get_requirement_status(id)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot modify system status".into()));
        }
        let mut payload = payload.clone();
        sanitize_string(&mut payload.title);
        sanitize_string(&mut payload.description);
        sanitize_string(&mut payload.tag);
        validate_requirement_status(&payload)
            .map_err(|err| RepoError::BadInput(err.to_string()))?;
        let mut repo = self.state.repo_write();
        repo.update_requirement_status(id, &payload)
    }

    /// Delete a requirement status. Fails if system or in use.
    pub fn delete_requirement_status(&self, id: i32) -> Result<RequirementStatus, RepoError> {
        let status = self.get_requirement_status(id)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot delete system status".into()));
        }
        let mut repo = self.state.repo_write();
        repo.delete_requirement_status(id)
    }

    /// Update a test status (title, description, tag). Fails if the status is system.
    pub fn update_test_status(&self, id: i32, payload: &NewTestStatus) -> Result<bool, RepoError> {
        let status = self.get_test_status(id)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot modify system status".into()));
        }
        let mut payload = payload.clone();
        sanitize_string(&mut payload.title);
        sanitize_string(&mut payload.description);
        sanitize_string(&mut payload.tag);
        validate_requirement_status(&NewRequirementStatus {
            id: payload.id,
            title: payload.title.clone(),
            description: payload.description.clone(),
            tag: payload.tag.clone(),
            project_id: payload.project_id,
            is_system: payload.is_system,
            tag_color: payload.tag_color.clone(),
        })
        .map_err(|err| RepoError::BadInput(err.to_string()))?;
        let mut repo = self.state.repo_write();
        repo.update_test_status(id, &payload)
    }

    /// Delete a test status. Fails if system or in use.
    pub fn delete_test_status(&self, id: i32) -> Result<TestStatus, RepoError> {
        let status = self.get_test_status(id)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot delete system status".into()));
        }
        let mut repo = self.state.repo_write();
        repo.delete_test_status(id)
    }

    /// Initialize default requirement and test statuses for a new project.
    ///
    /// This method creates the standard set of statuses defined in the
    /// `RequirementStatusEnum` and `TestStatusEnum` enums for the given project.
    ///
    /// # Arguments
    /// * `project_id` - The ID of the project to initialize statuses for
    ///
    /// # Returns
    /// * `Ok(())` if all statuses were created successfully
    /// * `Err(RepoError)` if any status creation failed
    pub fn initialize_default_statuses(&self, project_id: i32) -> Result<(), RepoError> {
        // Initialize requirement statuses from enum
        for status_enum in RequirementStatusEnum::all() {
            let payload = NewRequirementStatus {
                id: None,
                title: status_enum.title().to_string(),
                description: status_enum.description().to_string(),
                tag: status_enum.short_name().to_string(),
                project_id,
                is_system: true,
                tag_color: None,
            };
            self.create_system_requirement_status(payload)?;
        }

        // Initialize test statuses from enum
        for status_enum in TestStatusEnum::all() {
            let payload = NewTestStatus {
                id: None,
                title: status_enum.title().to_string(),
                description: status_enum.description().to_string(),
                tag: status_enum.short_name().to_string(),
                project_id,
                is_system: true,
                tag_color: None,
            };
            self.create_system_test_status(payload)?;
        }

        Ok(())
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
            RequirementStatus {
                id: 1,
                title: "Legacy".into(),
                description: "legacy".into(),
                tag: "LEG".into(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            },
        );
        repo.requirement_statuses.insert(
            2,
            RequirementStatus {
                id: 2,
                title: "Draft".into(),
                description: "draft".into(),
                tag: "DRT".into(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            },
        );
        repo.test_statuses.insert(
            3,
            TestStatus {
                id: 3,
                title: "Ready".into(),
                description: "ready".into(),
                tag: "RDY".into(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            },
        );
        repo
    }

    #[test]
    fn list_methods_forward_to_repository() {
        let repo = populated_repo();
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        assert_eq!(service.list_requirement_statuses().unwrap().len(), 1);
        assert_eq!(service.list_test_statuses().unwrap().len(), 1);
        assert_eq!(service.get_requirement_status(2).unwrap().title, "Draft");
        assert_eq!(service.get_test_status(3).unwrap().title, "Ready");
    }

    #[test]
    fn create_requirement_status_trims_input() {
        let repo = populated_repo();
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let payload = NewRequirementStatus {
            id: None,
            title: "  Verified  ".into(),
            description: "  Description  ".into(),
            tag: "  VFD  ".into(),
            project_id: 1,
            is_system: false,
            tag_color: None,
        };

        let id = service.create_requirement_status(payload).unwrap();

        let repo_guard = state.repo_read();
        let stored = repo_guard.inner_repo().statuses.get(&id).unwrap();
        assert_eq!(stored.title, "Verified");
        assert_eq!(stored.description, "Description");
        assert_eq!(stored.tag, "VFD");
    }

    #[test]
    fn create_requirement_status_rejects_invalid_title() {
        let repo = populated_repo();
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let payload = NewRequirementStatus {
            id: None,
            title: " ".into(),
            description: "Desc".into(),
            tag: "DRT".into(),
            project_id: 1,
            is_system: false,
            tag_color: None,
        };

        let err = service.create_requirement_status(payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn initialize_default_statuses_creates_all_standard_statuses() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let project_id = 42;
        let result = service.initialize_default_statuses(project_id);
        assert!(result.is_ok());

        // Verify all requirement statuses were created
        let req_statuses = service.list_requirement_statuses().unwrap();
        let project_req_statuses: Vec<_> = req_statuses
            .iter()
            .filter(|s| s.project_id == project_id)
            .collect();
        assert_eq!(project_req_statuses.len(), 6); // Draft, Proposal, Accepted, Rejected, Cancelled, Finished

        // Verify all test statuses were created
        let test_statuses = service.list_test_statuses().unwrap();
        let project_test_statuses: Vec<_> = test_statuses
            .iter()
            .filter(|s| s.project_id == project_id)
            .collect();
        assert_eq!(project_test_statuses.len(), 4); // Passed, Failed, Pending, In Progress

        // Verify specific statuses exist with correct titles
        let req_titles: Vec<_> = project_req_statuses
            .iter()
            .map(|s| s.title.as_str())
            .collect();
        assert!(req_titles.contains(&"Draft"));
        assert!(req_titles.contains(&"Accepted"));
        assert!(req_titles.contains(&"Finished"));

        let test_titles: Vec<_> = project_test_statuses
            .iter()
            .map(|s| s.title.as_str())
            .collect();
        assert!(test_titles.contains(&"Passed"));
        assert!(test_titles.contains(&"Failed"));
        assert!(test_titles.contains(&"Pending"));
    }

    #[test]
    fn list_requirement_statuses_by_project_filters_correctly() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                id: 1,
                title: "Draft".into(),
                description: "draft".into(),
                tag: "DRT".into(),
                project_id: 10,
                is_system: false,
                tag_color: None,
            },
        );
        repo.requirement_statuses.insert(
            2,
            RequirementStatus {
                id: 2,
                title: "Accepted".into(),
                description: "accepted".into(),
                tag: "ACC".into(),
                project_id: 10,
                is_system: false,
                tag_color: None,
            },
        );
        repo.requirement_statuses.insert(
            3,
            RequirementStatus {
                id: 3,
                title: "Finished".into(),
                description: "finished".into(),
                tag: "FIN".into(),
                project_id: 20,
                is_system: false,
                tag_color: None,
            },
        );

        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let statuses = service.list_requirement_statuses_by_project(10).unwrap();
        assert_eq!(statuses.len(), 2);
        let titles: Vec<&str> = statuses.iter().map(|s| s.title.as_str()).collect();
        assert!(titles.contains(&"Draft"));
        assert!(titles.contains(&"Accepted"));
    }

    #[test]
    fn list_test_statuses_by_project_filters_correctly() {
        let mut repo = DieselRepoMock::default();
        repo.test_statuses.insert(
            1,
            TestStatus {
                id: 1,
                title: "Passed".into(),
                description: "passed".into(),
                tag: "PASS".into(),
                project_id: 10,
                is_system: false,
                tag_color: None,
            },
        );
        repo.test_statuses.insert(
            2,
            TestStatus {
                id: 2,
                title: "Failed".into(),
                description: "failed".into(),
                tag: "FAIL".into(),
                project_id: 10,
                is_system: false,
                tag_color: None,
            },
        );
        repo.test_statuses.insert(
            3,
            TestStatus {
                id: 3,
                title: "Pending".into(),
                description: "pending".into(),
                tag: "PEND".into(),
                project_id: 20,
                is_system: false,
                tag_color: None,
            },
        );

        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let statuses = service.list_test_statuses_by_project(10).unwrap();
        assert_eq!(statuses.len(), 2);
        let titles: Vec<&str> = statuses.iter().map(|s| s.title.as_str()).collect();
        assert!(titles.contains(&"Passed"));
        assert!(titles.contains(&"Failed"));
    }

    #[test]
    fn get_status_name_returns_title() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                id: 1,
                title: "Draft".into(),
                description: "draft".into(),
                tag: "DRT".into(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            },
        );

        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let name = service.get_status_name(1).unwrap();
        assert_eq!(name, "Draft");
    }

    #[test]
    fn get_status_name_returns_not_found_for_missing_status() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let result = service.get_status_name(999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn create_test_status_trims_input() {
        let repo = populated_repo();
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let payload = NewTestStatus {
            id: None,
            title: "  In Progress  ".into(),
            description: "  Description  ".into(),
            tag: "  PROG  ".into(),
            project_id: 1,
            is_system: false,
            tag_color: None,
        };

        let id = service.create_test_status(payload).unwrap();

        let repo_guard = state.repo_read();
        let stored = repo_guard.inner_repo().test_statuses.get(&id).unwrap();
        assert_eq!(stored.title, "In Progress");
        assert_eq!(stored.description, "Description");
        assert_eq!(stored.tag, "PROG");
    }

    #[test]
    fn create_test_status_rejects_invalid_title() {
        let repo = populated_repo();
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let payload = NewTestStatus {
            id: None,
            title: " ".into(),
            description: "Desc".into(),
            tag: "TAG".into(),
            project_id: 1,
            is_system: false,
            tag_color: None,
        };

        let err = service.create_test_status(payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn list_requirement_statuses_by_project_returns_empty_for_nonexistent_project() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let statuses = service.list_requirement_statuses_by_project(999).unwrap();
        assert_eq!(statuses.len(), 0);
    }

    #[test]
    fn list_test_statuses_by_project_returns_empty_for_nonexistent_project() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let statuses = service.list_test_statuses_by_project(999).unwrap();
        assert_eq!(statuses.len(), 0);
    }

    #[test]
    fn get_requirement_status_returns_not_found_for_missing_status() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let result = service.get_requirement_status(999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn get_test_status_returns_not_found_for_missing_status() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let result = service.get_test_status(999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn update_requirement_status_rejects_system_status() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                id: 1,
                title: "Draft".into(),
                description: "".into(),
                tag: "Drf".into(),
                project_id: 1,
                is_system: true,
                tag_color: None,
            },
        );
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let payload = NewRequirementStatus {
            id: Some(1),
            title: "Custom".into(),
            description: "".into(),
            tag: "Cus".into(),
            project_id: 1,
            is_system: false,
            tag_color: None,
        };
        let err = service.update_requirement_status(1, &payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
        assert!(err.to_string().to_lowercase().contains("system"));
    }

    #[test]
    fn delete_requirement_status_rejects_system_status() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                id: 1,
                title: "Draft".into(),
                description: "".into(),
                tag: "Drf".into(),
                project_id: 1,
                is_system: true,
                tag_color: None,
            },
        );
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let err = service.delete_requirement_status(1).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
        assert!(err.to_string().to_lowercase().contains("system"));
    }

    #[test]
    fn update_test_status_rejects_system_status() {
        let mut repo = DieselRepoMock::default();
        repo.test_statuses.insert(
            1,
            TestStatus {
                id: 1,
                title: "Passed".into(),
                description: "".into(),
                tag: "Pass".into(),
                project_id: 1,
                is_system: true,
                tag_color: None,
            },
        );
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let payload = NewTestStatus {
            id: Some(1),
            title: "Custom".into(),
            description: "".into(),
            tag: "Cus".into(),
            project_id: 1,
            is_system: false,
            tag_color: None,
        };
        let err = service.update_test_status(1, &payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
        assert!(err.to_string().to_lowercase().contains("system"));
    }

    #[test]
    fn delete_test_status_rejects_system_status() {
        let mut repo = DieselRepoMock::default();
        repo.test_statuses.insert(
            1,
            TestStatus {
                id: 1,
                title: "Passed".into(),
                description: "".into(),
                tag: "Pass".into(),
                project_id: 1,
                is_system: true,
                tag_color: None,
            },
        );
        let state = state_with_repo(repo);
        let service = StatusService::new(&state);

        let err = service.delete_test_status(1).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
        assert!(err.to_string().to_lowercase().contains("system"));
    }
}

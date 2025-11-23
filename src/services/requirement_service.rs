//! Service providing requirement related operations.
//!
//! The service is intentionally lightweight and wraps repository calls with
//! validation and logging so that route handlers can remain focused on HTTP
//! concerns.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{NewRequirement, Requirement, TestCase, User};
use crate::repository::errors::RepoError;
use crate::repository::{PooledConnectionWrapper, RequirementsRepository, TestsCaseRepository};
use crate::validation::{sanitize_optional_string, sanitize_string, validate_requirement};

/// High level operations for requirements backed by the shared [`AppState`].
pub struct RequirementService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> RequirementService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve all requirements.
    pub fn list_all(&self) -> Result<Vec<Requirement>, RepoError> {
        self.repo_read().get_requirements_all()
    }

    /// Retrieve requirements scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<Requirement>, RepoError> {
        self.repo_read().get_requirements_by_project(project_id)
    }

    pub fn list_by_project_filtered(
        &self,
        project_id: i32,
        status_filter: Option<i32>,
        verification_filter: Option<i32>,
        category_filter: Option<i32>,
        applicability_filter: Option<i32>,
    ) -> Result<Vec<Requirement>, RepoError> {
        Ok(Self::filter(
            self.list_by_project(project_id)?,
            status_filter,
            verification_filter,
            category_filter,
            applicability_filter,
        ))
    }

    /// Retrieve a single requirement by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<Requirement, RepoError> {
        self.repo_read().get_requirement_by_id(id)
    }

    pub fn get_by_parent_id(&self, parent_id: i32) -> Result<Vec<Requirement>, RepoError> {
        Ok(self
            .list_all()?
            .into_iter()
            .filter(|r| r.parent_id == Some(parent_id))
            .collect())
    }

    pub fn get_linked_tests(&self, id: i32) -> Result<Vec<TestCase>, RepoError> {
        self.repo_read().get_tests_for_requirement(id)
    }

    /// Create a new requirement entry and log the action.
    pub fn create(&self, actor: &User, mut payload: NewRequirement) -> Result<i32, RepoError> {
        self.prepare_payload(&mut payload)?;

        let id = {
            let mut repo = self.repo_write();
            repo.insert_new_requirement(&payload)?
        };

        self.log_created(actor, id, &payload);
        Ok(id)
    }

    /// Update an existing requirement entry and log the change.
    pub fn update(
        &self,
        actor: &User,
        id: i32,
        mut payload: NewRequirement,
    ) -> Result<Requirement, RepoError> {
        self.prepare_payload(&mut payload)?;
        payload.id = Some(id);

        let before = self.get_by_id(id)?;

        {
            let mut repo = self.repo_write();
            let updated = repo.edit_requirement(&payload)?;
            if !updated {
                return Err(RepoError::NotFound);
            }
        }

        let after = self.get_by_id(id)?;
        self.log_updated(actor, &before, &after);
        Ok(after)
    }

    /// Delete an requirement entry and log the removal.
    pub fn delete(&self, actor: &User, id: i32) -> Result<Requirement, RepoError> {
        let removed = {
            let mut repo = self.repo_write();
            repo.delete_requirement(id)?
        };

        self.log_deleted(actor, &removed);
        Ok(removed)
    }

    fn filter(
        requirements: Vec<Requirement>,
        status_filter: Option<i32>,
        verification_filter: Option<i32>,
        category_filter: Option<i32>,
        applicability_filter: Option<i32>,
    ) -> Vec<Requirement> {
        let mut filtered_requirements: Vec<Requirement> = requirements
            .into_iter()
            .filter(|req| {
                let status_match =
                    status_filter.map_or(true, |status_id| req.status_id == status_id);
                let verification_match =
                    verification_filter.map_or(true, |id| req.verification_method_id == id);
                let category_match =
                    category_filter.map_or(true, |category_id| req.category_id == category_id);
                let applicability_match = applicability_filter.map_or(true, |applicability_id| {
                    req.applicability_id == applicability_id
                });
                status_match && verification_match && category_match && applicability_match
            })
            .collect();

        filtered_requirements.sort_by(|a, b| {
            match (a.reference_code.is_empty(), b.reference_code.is_empty()) {
                (false, false) => a.reference_code.cmp(&b.reference_code),
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                (true, true) => a.id.cmp(&b.id),
            }
        });

        filtered_requirements
    }

    fn repo_read(&self) -> std::sync::RwLockReadGuard<'_, DieselCachedRepo> {
        self.state.repo.read().expect("repo lock poisoned")
    }

    fn repo_write(&self) -> std::sync::RwLockWriteGuard<'_, DieselCachedRepo> {
        self.state.repo.write().expect("repo lock poisoned")
    }

    fn prepare_payload(&self, payload: &mut NewRequirement) -> Result<(), RepoError> {
        sanitize_string(&mut payload.title);
        sanitize_string(&mut payload.description);
        sanitize_string(&mut payload.reference_code);
        sanitize_optional_string(&mut payload.justification);

        validate_requirement(payload).map_err(|err| RepoError::BadInput(err.to_string()))
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_created(&self, actor: &User, id: i32, entity: &NewRequirement) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log requirement creation {id}: {_err}");
            }
        }
    }

    fn log_updated(&self, actor: &User, before: &Requirement, after: &Requirement) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::updated(conn.as_mut(), &ctx, before, after) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log requirement update {} -> {}: {_err}",
                    before.id, after.id
                );
            }
        }
    }

    fn log_deleted(&self, actor: &User, entity: &Requirement) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log requirement deletion {}: {_err}", entity.id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::sync::{Arc, RwLock};

    fn timestamp() -> NaiveDateTime {
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

    fn actor() -> User {
        DieselRepoMock::make_user(1, "actor", "")
    }

    fn requirement(id: i32, project_id: i32, reference: &str) -> Requirement {
        Requirement {
            id: id,
            title: format!("Requirement {id}"),
            description: "Existing description".into(),
            verification_method_id: 1,
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: reference.into(),
            category_id: 1,
            parent_id: Some(1),
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            applicability_id: 1,
            justification: Some("because".into()),
            project_id,
        }
    }

    fn new_payload() -> NewRequirement {
        NewRequirement {
            id: None,
            title: "  Title  ".into(),
            description: "  Description  ".into(),
            verification_method_id: 1,
            author_id: 1,
            category_id: 1,
            status_id: 1,
            parent_id: None,
            reference_code: "  REQ-123  ".into(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: Some("   ".into()),
            project_id: 7,
        }
    }

    #[test]
    fn create_sanitizes_payload() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let payload = new_payload();
        let id = service.create(&actor(), payload).unwrap();

        let stored = service.get_by_id(id).unwrap();
        assert_eq!(stored.title, "Title");
        assert_eq!(stored.description, "Description");
        assert_eq!(stored.reference_code, "REQ-123");
        assert!(stored.justification.is_none());
    }

    #[test]
    fn create_rejects_invalid_reference() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let mut payload = new_payload();
        payload.reference_code = "invalid".into();

        let err = service.create(&actor(), payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn update_modifies_existing_requirement() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(1, requirement(1, 7, "REQ-001"));
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let mut payload = new_payload();
        payload.title = "  Updated  ".into();
        payload.description = "  New Description  ".into();
        payload.reference_code = "  REQ-999  ".into();

        let updated = service.update(&actor(), 1, payload).unwrap();
        assert_eq!(updated.title, "Updated");
        assert_eq!(updated.description, "New Description");
        assert_eq!(updated.reference_code, "REQ-999");
    }

    #[test]
    fn delete_removes_requirement() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(2, requirement(2, 7, "REQ-002"));
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let removed = service.delete(&actor(), 2).unwrap();
        assert_eq!(removed.id, 2);
        assert!(matches!(service.get_by_id(2), Err(RepoError::NotFound)));
    }

    #[test]
    fn list_by_project_filters_requirements() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(1, requirement(1, 7, "REQ-001"));
        repo.requirements.insert(2, requirement(2, 99, "REQ-002"));
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let items = service.list_by_project(7).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].project_id, 7);
    }
}

//! Service providing requirement related operations.
//!
//! The service is intentionally lightweight and wraps repository calls with
//! validation and logging so that route handlers can remain focused on HTTP
//! concerns.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{NewRequirement, Requirement, DecoratedRequirement, User, Test};
use crate::repository::errors::RepoError;
use crate::repository::{PooledConnectionWrapper, RequirementsRepository, TestsRepository};
use crate::validation::{sanitize_optional_string, sanitize_string, validate_requirement};
use crate::helper_functions::filter_requirements;

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

    pub fn list_by_project_filtered(&self,
        project_id: i32,
        status_filter: Option<i32>,
        verification_filter: Option<i32>,
        category_filter: Option<i32>)
    -> Result<Vec<Requirement>, RepoError> {
        let requirements = self.list_by_project(project_id)?;
        Ok(filter_requirements(
            requirements,
            status_filter,
            verification_filter,
            category_filter,
        ))
    }

    /// Retrieve a single requirement by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<Requirement, RepoError> {
        self.repo_read().get_requirement_by_id(id)
    }

    /// Retrieve a single requirement by identifier.
    pub fn get_by_id_decorated(&self, id: i32) -> Result<DecoratedRequirement, RepoError> {
        use crate::repository::{LookupRepository, UserRepository};
        let req = self.get_by_id(id)?;

        let verification = self.repo_read()
            .get_verification_by_id(req.req_verification)
            .map(|v| v.verification_name)
            .unwrap_or_else(|_| format!("Unknown Verification ({})", req.req_verification));

        let status = self.repo_read()
            .get_requirement_status_by_id(req.req_current_status)
            .map(|s| s.req_st_title)
            .unwrap_or_else(|_| format!("Unknown Status ({})", req.req_current_status));

        let author = self.repo_read().get_user_by_id(req.req_author)
            .map(|u| u.user_name)
            .unwrap_or_default();

        let reviewer = if req.req_reviewer != 0 {
            self.repo_read().get_user_by_id(req.req_reviewer)
                .map(|u| u.user_name)
                .unwrap_or_default()
        } else {
            String::new()
        };

        let category = self.repo_read()
            .get_category_by_id(req.req_category)
            .map(|c| c.cat_title)
            .unwrap_or_else(|_| format!("Unknown Category ({})", req.req_category));

        let applicability = self.repo_read()
            .get_applicability_by_id(req.req_applicability)
            .map(|a| a.app_title)
            .unwrap_or_else(|_| format!("Unknown Applicability ({})", req.req_applicability));

        let parent_title = if req.req_parent != 0 {
            match self.repo_read().get_requirement_by_id(req.req_parent) {
                Ok(parent_req) => parent_req.req_title,
                Err(_) => "[Deleted Parent]".to_string(),
            }
        } else {
            String::new()
        };

        Ok(DecoratedRequirement {
            req_id: req.req_id,
            req_title: req.req_title,
            req_verification: verification,
            req_verification_id: req.req_verification,
            req_description: req.req_description,
            req_current_status: status,
            req_current_status_id: req.req_current_status,
            req_author: author,
            req_author_id: req.req_author,
            req_reviewer: reviewer,
            req_reviewer_id: req.req_reviewer,
            req_link: req.req_link,
            req_reference: req.req_reference,
            req_category: category,
            req_category_id: req.req_category,
            req_applicability: applicability,
            req_applicability_id: req.req_applicability,
            req_parent_id: req.req_parent,
            req_parent_title: parent_title,
            req_creation_date: req.req_creation_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            req_update_date: req.req_update_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            req_deadline_date: req.req_deadline_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            req_justification: req.req_justification,
            project_id: req.project_id,
        })
    }

    pub fn get_linked_tests(&self, id: i32) -> Result<Vec<Test>, RepoError> {
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
        payload.req_id = Some(id);

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

    fn repo_read(&self) -> std::sync::RwLockReadGuard<'_, DieselCachedRepo> {
        self.state.repo.read().expect("repo lock poisoned")
    }

    fn repo_write(&self) -> std::sync::RwLockWriteGuard<'_, DieselCachedRepo> {
        self.state.repo.write().expect("repo lock poisoned")
    }

    fn prepare_payload(&self, payload: &mut NewRequirement) -> Result<(), RepoError> {
        sanitize_string(&mut payload.req_title);
        sanitize_string(&mut payload.req_description);
        sanitize_string(&mut payload.req_reference);
        sanitize_string(&mut payload.req_link);
        sanitize_optional_string(&mut payload.req_justification);

        validate_requirement(payload).map_err(|err| RepoError::BadInput(err.to_string()))
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_created(&self, actor: &User, id: i32, entity: &NewRequirement) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.user_id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log requirement creation {id}: {_err}");
            }
        }
    }

    fn log_updated(&self, actor: &User, before: &Requirement, after: &Requirement) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.user_id);
            if let Err(_err) = Logger::updated(conn.as_mut(), &ctx, before, after) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log requirement update {} -> {}: {_err}",
                    before.req_id, after.req_id
                );
            }
        }
    }

    fn log_deleted(&self, actor: &User, entity: &Requirement) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.user_id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log requirement deletion {}: {_err}",
                    entity.req_id
                );
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
            req_id: id,
            req_title: format!("Requirement {id}"),
            req_description: "Existing description".into(),
            req_verification: 1,
            req_current_status: 1,
            req_author: 1,
            req_reviewer: 1,
            req_link: "https://example.com".into(),
            req_reference: reference.into(),
            req_category: 1,
            req_parent: 1,
            req_creation_date: timestamp(),
            req_update_date: timestamp(),
            req_deadline_date: timestamp(),
            req_applicability: 1,
            req_justification: Some("because".into()),
            project_id,
        }
    }

    fn new_payload() -> NewRequirement {
        NewRequirement {
            req_id: None,
            req_title: "  Title  ".into(),
            req_description: "  Description  ".into(),
            req_verification: 1,
            req_author: 1,
            req_link: "  https://example.com/path  ".into(),
            req_category: 1,
            req_current_status: 1,
            req_parent: 1,
            req_reference: "  REQ-123  ".into(),
            req_reviewer: 1,
            req_applicability: 1,
            req_justification: Some("   ".into()),
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
        assert_eq!(stored.req_title, "Title");
        assert_eq!(stored.req_description, "Description");
        assert_eq!(stored.req_reference, "REQ-123");
        assert_eq!(stored.req_link, "https://example.com/path");
        assert!(stored.req_justification.is_none());
    }

    #[test]
    fn create_rejects_invalid_reference() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let mut payload = new_payload();
        payload.req_reference = "invalid".into();

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
        payload.req_title = "  Updated  ".into();
        payload.req_description = "  New Description  ".into();
        payload.req_reference = "  REQ-999  ".into();

        let updated = service.update(&actor(), 1, payload).unwrap();
        assert_eq!(updated.req_title, "Updated");
        assert_eq!(updated.req_description, "New Description");
        assert_eq!(updated.req_reference, "REQ-999");
        assert_eq!(updated.req_link, "https://example.com/path");
    }

    #[test]
    fn delete_removes_requirement() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(2, requirement(2, 7, "REQ-002"));
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let removed = service.delete(&actor(), 2).unwrap();
        assert_eq!(removed.req_id, 2);
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

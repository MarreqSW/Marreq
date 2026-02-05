//! Applicability service centralizing CRUD logic and logging.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{Applicability, NewApplicability, User};
use crate::repository::errors::RepoError;
use crate::repository::{LookupRepository, PooledConnectionWrapper};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref TAG_REGEX: Regex =
        Regex::new(r"^[A-Za-z0-9_]+$").expect("valid applicability tag regex");
}

/// Service wrapper that provides applicability operations backed by the shared AppState.
pub struct ApplicabilityService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> ApplicabilityService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve all applicability entries.
    pub fn list_all(&self) -> Result<Vec<Applicability>, RepoError> {
        self.state.repo_read().get_applicability_all()
    }

    /// Retrieve applicability entries scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<Applicability>, RepoError> {
        self.state
            .repo_read()
            .get_applicability_by_project(project_id)
    }

    /// Retrieve a single applicability by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<Applicability, RepoError> {
        self.state.repo_read().get_applicability_by_id(id)
    }

    /// Create a new applicability entry and log the action.
    pub fn create(&self, user: &User, mut new_app: NewApplicability) -> Result<i32, RepoError> {
        self.prepare_payload(&mut new_app)?;

        let id = {
            let mut repo = self.state.repo_write();
            repo.insert_new_applicability(&new_app)?
        };

        self.log_created(user, id, &new_app);
        Ok(id)
    }

    /// Update an existing applicability entry and log the change.
    pub fn update(
        &self,
        user: &User,
        id: i32,
        mut updated_app: NewApplicability,
    ) -> Result<Applicability, RepoError> {
        let before = self.get_by_id(id)?;

        updated_app.id = Some(id);
        self.prepare_payload(&mut updated_app)?;

        {
            let mut repo = self.state.repo_write();
            let updated = repo.edit_applicability(&updated_app)?;
            if !updated {
                return Err(RepoError::NotFound);
            }
        }

        let after = self.get_by_id(id)?;
        self.log_updated(user, &before, &after);
        Ok(after)
    }

    /// Delete an applicability entry and log the removal.
    pub fn delete(&self, user: &User, id: i32) -> Result<Applicability, RepoError> {
        let deleted = {
            let mut repo = self.state.repo_write();
            repo.delete_applicability(id)?
        };

        self.log_deleted(user, &deleted);
        Ok(deleted)
    }

    fn prepare_payload(&self, payload: &mut NewApplicability) -> Result<(), RepoError> {
        sanitize(&mut payload.title);
        sanitize(&mut payload.description);
        sanitize(&mut payload.tag);

        validate(payload)
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_created(&self, user: &User, id: i32, entity: &NewApplicability) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log applicability creation {id}: {_err}");
            }
        }
    }

    fn log_updated(&self, user: &User, before: &Applicability, after: &Applicability) {
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

    fn log_deleted(&self, user: &User, entity: &Applicability) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log applicability deletion {}: {_err}", entity.id);
            }
        }
    }
}

fn sanitize(value: &mut String) {
    *value = value.trim().to_string();
}

fn validate(payload: &NewApplicability) -> Result<(), RepoError> {
    if payload.title.is_empty() {
        return Err(bad_input("title is required"));
    }
    if payload.title.len() > 100 {
        return Err(bad_input("title must be at most 100 characters"));
    }
    if payload.title.len() < 2 {
        return Err(bad_input("title must be at least 2 characters"));
    }

    if payload.description.is_empty() {
        return Err(bad_input("description is required"));
    }
    if payload.description.len() > 500 {
        return Err(bad_input("description must be at most 500 characters"));
    }

    if payload.tag.is_empty() {
        return Err(bad_input("tag is required"));
    }
    if payload.tag.len() > 50 {
        return Err(bad_input("tag must be at most 50 characters"));
    }
    if !TAG_REGEX.is_match(&payload.tag) {
        return Err(bad_input(
            "tag must only contain letters, numbers, or underscores",
        ));
    }

    if payload.project_id <= 0 {
        return Err(bad_input("project_id must be positive"));
    }

    Ok(())
}

fn bad_input(message: impl Into<String>) -> RepoError {
    RepoError::BadInput(message.into())
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

    fn sample_app(id: i32, title: &str) -> Applicability {
        Applicability {
            id,
            title: title.into(),
            description: "desc".into(),
            tag: "TAG".into(),
            project_id: 1,
        }
    }

    #[test]
    fn create_trims_payload_and_validates() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "  Applicable  ".into(),
            description: "  Description  ".into(),
            tag: "  PROD_1  ".into(),
            project_id: 3,
        };

        let id = service.create(&actor(), payload).unwrap();
        let stored = service.get_by_id(id).unwrap();

        assert_eq!(stored.title, "Applicable");
        assert_eq!(stored.description, "Description");
        assert_eq!(stored.tag, "PROD_1");
    }

    #[test]
    fn create_rejects_invalid_tag() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "Title".into(),
            description: "Description".into(),
            tag: "invalid tag".into(),
            project_id: 1,
        };

        let err = service.create(&actor(), payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn update_trims_and_updates_existing_entry() {
        let mut repo = DieselRepoMock::default();
        repo.applicability.insert(7, sample_app(7, "Legacy"));
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "  New Title  ".into(),
            description: "  New Description  ".into(),
            tag: "  NEW_TAG  ".into(),
            project_id: 2,
        };

        let updated = service.update(&actor(), 7, payload).unwrap();
        assert_eq!(updated.title, "New Title");
        assert_eq!(updated.description, "New Description");
        assert_eq!(updated.tag, "NEW_TAG");
        assert_eq!(updated.project_id, 2);
    }

    #[test]
    fn delete_removes_entry() {
        let mut repo = DieselRepoMock::default();
        repo.applicability.insert(3, sample_app(3, "Removable"));
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let removed = service.delete(&actor(), 3).unwrap();
        assert_eq!(removed.id, 3);
        assert!(matches!(service.get_by_id(3), Err(RepoError::NotFound)));
    }

    #[test]
    fn list_by_project_filters_entries() {
        let mut repo = DieselRepoMock::default();
        repo.applicability.insert(1, sample_app(1, "Alpha"));
        repo.applicability.insert(
            2,
            Applicability {
                project_id: 99,
                ..sample_app(2, "Beta")
            },
        );
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let items = service.list_by_project(1).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Alpha");
    }

    #[test]
    fn list_all_returns_all_applicability_entries() {
        let mut repo = DieselRepoMock::default();
        repo.applicability.insert(1, sample_app(1, "Alpha"));
        repo.applicability.insert(2, sample_app(2, "Beta"));
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let items = service.list_all().unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn list_all_returns_empty_when_no_entries() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let items = service.list_all().unwrap();
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn get_by_id_returns_not_found_for_nonexistent_entry() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let result = service.get_by_id(999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn create_rejects_empty_title() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "".into(),
            description: "Description".into(),
            tag: "TAG".into(),
            project_id: 1,
        };

        let err = service.create(&actor(), payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn create_rejects_title_too_short() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "A".into(), // Too short (min 2 chars)
            description: "Description".into(),
            tag: "TAG".into(),
            project_id: 1,
        };

        let err = service.create(&actor(), payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn create_rejects_title_too_long() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "A".repeat(101), // Too long (max 100 chars)
            description: "Description".into(),
            tag: "TAG".into(),
            project_id: 1,
        };

        let err = service.create(&actor(), payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn create_rejects_empty_description() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "Title".into(),
            description: "".into(),
            tag: "TAG".into(),
            project_id: 1,
        };

        let err = service.create(&actor(), payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn create_rejects_description_too_long() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "Title".into(),
            description: "A".repeat(501), // Too long (max 500 chars)
            tag: "TAG".into(),
            project_id: 1,
        };

        let err = service.create(&actor(), payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn create_rejects_empty_tag() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "Title".into(),
            description: "Description".into(),
            tag: "".into(),
            project_id: 1,
        };

        let err = service.create(&actor(), payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn create_rejects_tag_too_long() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "Title".into(),
            description: "Description".into(),
            tag: "A".repeat(51), // Too long (max 50 chars)
            project_id: 1,
        };

        let err = service.create(&actor(), payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn create_rejects_tag_with_special_characters() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "Title".into(),
            description: "Description".into(),
            tag: "TAG-1".into(), // Contains hyphen
            project_id: 1,
        };

        let err = service.create(&actor(), payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn create_rejects_invalid_project_id() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "Title".into(),
            description: "Description".into(),
            tag: "TAG1".into(),
            project_id: 0, // Invalid (must be positive)
        };

        let err = service.create(&actor(), payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn create_accepts_valid_tag_with_underscores() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "Title".into(),
            description: "Description".into(),
            tag: "PROD_1".into(), // Valid with underscore
            project_id: 1,
        };

        let id = service.create(&actor(), payload).unwrap();
        let stored = service.get_by_id(id).unwrap();
        assert_eq!(stored.tag, "PROD_1");
    }

    #[test]
    fn update_returns_not_found_for_missing_entry() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "Title".into(),
            description: "Description".into(),
            tag: "TAG1".into(),
            project_id: 1,
        };

        let result = service.update(&actor(), 999, payload);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn delete_returns_not_found_for_missing_entry() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let result = service.delete(&actor(), 999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn update_validates_payload() {
        let mut repo = DieselRepoMock::default();
        repo.applicability.insert(1, sample_app(1, "Existing"));
        let state = state_with_repo(repo);
        let service = ApplicabilityService::new(&state);

        let payload = NewApplicability {
            id: None,
            title: "".into(), // Invalid empty title
            description: "Description".into(),
            tag: "TAG1".into(),
            project_id: 1,
        };

        let result = service.update(&actor(), 1, payload);
        assert!(matches!(result, Err(RepoError::BadInput(_))));
    }
}

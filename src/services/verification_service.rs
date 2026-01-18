//! Service for managing verification methods attached to requirements.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{NewVerificationMethod, VerificationMethod};
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;

/// High-level operations for verification methods backed by the shared [`AppState`].
pub struct VerificationService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> VerificationService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// List verification methods scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<VerificationMethod>, RepoError> {
        self.state
            .repo_read()
            .get_verification_by_project(project_id)
    }

    /// Retrieve a verification method by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<VerificationMethod, RepoError> {
        self.state.repo_read().get_verification_by_id(id)
    }

    pub fn get_verification_name(&self, id: i32) -> Result<String, RepoError> {
        let verification = self.state.repo_read().get_verification_by_id(id)?;
        Ok(verification.title)
    }

    /// Create a new verification entry.
    pub fn create(&self, mut payload: NewVerificationMethod) -> Result<i32, RepoError> {
        sanitize(&mut payload.title);
        sanitize(&mut payload.description);

        validate(&payload)?;

        let id = {
            let mut repo = self.state.repo_write();
            repo.insert_new_verification(&payload)?
        };

        Ok(id)
    }
}

fn sanitize(value: &mut String) {
    *value = value.trim().to_string();
}

fn validate(payload: &NewVerificationMethod) -> Result<(), RepoError> {
    if payload.title.is_empty() {
        return Err(RepoError::BadInput("title is required".to_string()));
    }
    if payload.title.len() > 120 {
        return Err(RepoError::BadInput(
            "title must be at most 120 characters".to_string(),
        ));
    }
    if payload.description.is_empty() {
        return Err(RepoError::BadInput("description is required".to_string()));
    }
    if payload.description.len() > 500 {
        return Err(RepoError::BadInput(
            "description must be at most 500 characters".to_string(),
        ));
    }
    Ok(())
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

    use crate::status_enums::ProjectStatus;

    #[test]
    fn create_trim_and_persists() {
        let mut repo = DieselRepoMock::default();
        repo.projects.insert(
            1,
            crate::models::Project {
                id: 1,
                name: "Demo".into(),
                description: None,
                creation_date: None,
                update_date: None,
                status: ProjectStatus::Active,
                owner_id: None,
            },
        );

        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let payload = NewVerificationMethod {
            id: None,
            title: "  Analysis ".into(),
            description: "  Evaluate expected metrics  ".into(),
            tag: "ANALYSIS".into(),
            project_id: 1,
        };

        let id = service.create(payload).expect("created");
        let stored = service.get_by_id(id).expect("stored");

        assert_eq!(stored.title, "Analysis");
        assert_eq!(stored.description, "Evaluate expected metrics");
    }

    #[test]
    fn reject_empty_payload() {
        let state = state_with_repo(DieselRepoMock::default());
        let service = VerificationService::new(&state);
        let payload = NewVerificationMethod {
            id: None,
            title: "".into(),
            description: "".into(),
            tag: "".into(),
            project_id: 1,
        };

        let result = service.create(payload);
        assert!(matches!(result, Err(RepoError::BadInput(_))));
    }

    #[test]
    fn list_by_project_filters_verification_methods() {
        let mut repo = DieselRepoMock::default();
        repo.verifications.insert(
            1,
            VerificationMethod {
                id: 1,
                title: "Analysis".into(),
                description: "Analysis method".into(),
                tag: "ANALYSIS".into(),
                project_id: 10,
            },
        );
        repo.verifications.insert(
            2,
            VerificationMethod {
                id: 2,
                title: "Test".into(),
                description: "Test method".into(),
                tag: "TEST".into(),
                project_id: 10,
            },
        );
        repo.verifications.insert(
            3,
            VerificationMethod {
                id: 3,
                title: "Review".into(),
                description: "Review method".into(),
                tag: "REVIEW".into(),
                project_id: 20,
            },
        );

        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let methods = service.list_by_project(10).unwrap();
        assert_eq!(methods.len(), 2);
        let titles: Vec<&str> = methods.iter().map(|v| v.title.as_str()).collect();
        assert!(titles.contains(&"Analysis"));
        assert!(titles.contains(&"Test"));
    }

    #[test]
    fn list_by_project_returns_empty_for_nonexistent_project() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let methods = service.list_by_project(999).unwrap();
        assert_eq!(methods.len(), 0);
    }

    #[test]
    fn get_by_id_returns_not_found_for_missing_verification() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let result = service.get_by_id(999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn get_verification_name_returns_title() {
        let mut repo = DieselRepoMock::default();
        repo.verifications.insert(
            1,
            VerificationMethod {
                id: 1,
                title: "Analysis".into(),
                description: "Analysis method".into(),
                tag: "ANALYSIS".into(),
                project_id: 1,
            },
        );

        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let name = service.get_verification_name(1).unwrap();
        assert_eq!(name, "Analysis");
    }

    #[test]
    fn get_verification_name_returns_not_found_for_missing_verification() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let result = service.get_verification_name(999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn create_rejects_title_too_long() {
        let mut repo = DieselRepoMock::default();
        repo.projects.insert(
            1,
            crate::models::Project {
                id: 1,
                name: "Demo".into(),
                description: None,
                creation_date: None,
                update_date: None,
                status: ProjectStatus::Active,
                owner_id: None,
            },
        );

        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let payload = NewVerificationMethod {
            id: None,
            title: "A".repeat(121), // Too long (max 120 chars)
            description: "Description".into(),
            tag: "TAG".into(),
            project_id: 1,
        };

        let err = service.create(payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn create_rejects_description_too_long() {
        let mut repo = DieselRepoMock::default();
        repo.projects.insert(
            1,
            crate::models::Project {
                id: 1,
                name: "Demo".into(),
                description: None,
                creation_date: None,
                update_date: None,
                status: ProjectStatus::Active,
                owner_id: None,
            },
        );

        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let payload = NewVerificationMethod {
            id: None,
            title: "Title".into(),
            description: "A".repeat(501), // Too long (max 500 chars)
            tag: "TAG".into(),
            project_id: 1,
        };

        let err = service.create(payload).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn create_accepts_valid_payload() {
        let mut repo = DieselRepoMock::default();
        repo.projects.insert(
            1,
            crate::models::Project {
                id: 1,
                name: "Demo".into(),
                description: None,
                creation_date: None,
                update_date: None,
                status: ProjectStatus::Active,
                owner_id: None,
            },
        );

        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let payload = NewVerificationMethod {
            id: None,
            title: "Valid Title".into(),
            description: "Valid description".into(),
            tag: "TAG".into(),
            project_id: 1,
        };

        let id = service.create(payload).unwrap();
        let stored = service.get_by_id(id).unwrap();
        assert_eq!(stored.title, "Valid Title");
        assert_eq!(stored.description, "Valid description");
    }
}

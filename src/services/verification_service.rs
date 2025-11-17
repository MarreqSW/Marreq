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
        Ok(verification.name)
    }

    /// Create a new verification entry.
    pub fn create(&self, mut payload: NewVerificationMethod) -> Result<i32, RepoError> {
        sanitize(&mut payload.name);
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
    if payload.name.is_empty() {
        return Err(RepoError::BadInput(
            "name is required".to_string(),
        ));
    }
    if payload.name.len() > 120 {
        return Err(RepoError::BadInput(
            "name must be at most 120 characters".to_string(),
        ));
    }
    if payload.description.is_empty() {
        return Err(RepoError::BadInput(
            "description is required".to_string(),
        ));
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
                status_id: None,
                owner_id: None,
            },
        );

        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let payload = NewVerificationMethod {
            id: None,
            name: "  Analysis ".into(),
            description: "  Evaluate expected metrics  ".into(),
            project_id: 1,
        };

        let id = service.create(payload).expect("created");
        let stored = service.get_by_id(id).expect("stored");

        assert_eq!(stored.name, "Analysis");
        assert_eq!(stored.description, "Evaluate expected metrics");
    }

    #[test]
    fn reject_empty_payload() {
        let state = state_with_repo(DieselRepoMock::default());
        let service = VerificationService::new(&state);
        let payload = NewVerificationMethod {
            id: None,
            name: "".into(),
            description: "".into(),
            project_id: 1,
        };

        let result = service.create(payload);
        assert!(matches!(result, Err(RepoError::BadInput(_))));
    }
}

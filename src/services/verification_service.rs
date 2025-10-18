//! Service for managing verification methods attached to requirements.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{NewVerification, Verification};
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
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<Verification>, RepoError> {
        self.state
            .repo_read()
            .get_verification_by_project(project_id)
    }

    /// Retrieve a verification method by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<Verification, RepoError> {
        self.state.repo_read().get_verification_by_id(id)
    }

    /// Create a new verification entry.
    pub fn create(&self, mut payload: NewVerification) -> Result<i32, RepoError> {
        sanitize(&mut payload.verification_name);
        sanitize(&mut payload.verification_description);

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

fn validate(payload: &NewVerification) -> Result<(), RepoError> {
    if payload.verification_name.is_empty() {
        return Err(RepoError::BadInput(
            "verification_name is required".to_string(),
        ));
    }
    if payload.verification_name.len() > 120 {
        return Err(RepoError::BadInput(
            "verification_name must be at most 120 characters".to_string(),
        ));
    }
    if payload.verification_description.is_empty() {
        return Err(RepoError::BadInput(
            "verification_description is required".to_string(),
        ));
    }
    if payload.verification_description.len() > 500 {
        return Err(RepoError::BadInput(
            "verification_description must be at most 500 characters".to_string(),
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
                project_id: 1,
                project_name: "Demo".into(),
                project_description: None,
                project_creation_date: None,
                project_update_date: None,
                project_status: None,
                project_owner_id: None,
            },
        );

        let state = state_with_repo(repo);
        let service = VerificationService::new(&state);

        let payload = NewVerification {
            verification_id: None,
            verification_name: "  Analysis ".into(),
            verification_description: "  Evaluate expected metrics  ".into(),
            project_id: 1,
        };

        let id = service.create(payload).expect("created");
        let stored = service.get_by_id(id).expect("stored");

        assert_eq!(stored.verification_name, "Analysis");
        assert_eq!(stored.verification_description, "Evaluate expected metrics");
    }

    #[test]
    fn reject_empty_payload() {
        let state = state_with_repo(DieselRepoMock::default());
        let service = VerificationService::new(&state);
        let payload = NewVerification {
            verification_id: None,
            verification_name: "".into(),
            verification_description: "".into(),
            project_id: 1,
        };

        let result = service.create(payload);
        assert!(matches!(result, Err(RepoError::BadInput(_))));
    }
}

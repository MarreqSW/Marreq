//! Service for requirement comments (create and list).
//!
//! Comments are immutable. Permission checks (project membership, lock approved version)
//! are done in the API layer; this service validates requirement/version existence and
//! delegates to the repository.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{NewRequirementComment, RequirementComment};
use crate::repository::errors::RepoError;
use crate::repository::{RequirementCommentsRepository, RequirementsRepository};

pub struct CommentService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> CommentService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Create a comment. Caller must have verified project membership and (if applicable)
    /// that the version is not locked. This validates that the requirement and optional
    /// version exist and that the version belongs to the requirement.
    pub fn create_comment(
        &self,
        requirement_id: i32,
        requirement_version_id: Option<i32>,
        author_id: i32,
        body: String,
    ) -> Result<RequirementComment, RepoError> {
        let _req = self.repo_read().get_requirement_by_id(requirement_id)?;
        if let Some(version_id) = requirement_version_id {
            let version = self.repo_read().get_requirement_version_by_id(version_id)?;
            if version.requirement_id != requirement_id {
                return Err(RepoError::BadInput(
                    "version does not belong to requirement".into(),
                ));
            }
        }
        let body = body.trim();
        if body.is_empty() {
            return Err(RepoError::BadInput("comment body must not be empty".into()));
        }
        let new = NewRequirementComment {
            requirement_id,
            requirement_version_id,
            author_id,
            body: body.to_string(),
        };
        self.repo_write().insert_requirement_comment(&new)
    }

    /// List comments for a requirement, optionally filtered to a version (comments for that
    /// version or requirement-level). Chronological order (created_at ASC).
    pub fn list_comments(
        &self,
        requirement_id: i32,
        version_id: Option<i32>,
    ) -> Result<Vec<RequirementComment>, RepoError> {
        let _req = self.repo_read().get_requirement_by_id(requirement_id)?;
        self.repo_read()
            .list_comments_by_requirement(requirement_id, version_id)
    }

    fn repo_read(&self) -> std::sync::RwLockReadGuard<'_, DieselCachedRepo> {
        self.state.repo.read().expect("repo lock poisoned")
    }

    fn repo_write(&self) -> std::sync::RwLockWriteGuard<'_, DieselCachedRepo> {
        self.state.repo.write().expect("repo lock poisoned")
    }
}

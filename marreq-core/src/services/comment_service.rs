// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Service for requirement comments (create and list).
//!
//! Comments are immutable. Permission checks (project membership, lock approved version)
//! are done in the API layer; this service validates requirement/version existence and
//! delegates to the repository. `@username` mentions of project members are
//! notified when a comment is created.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{NewRequirementComment, RequirementComment, User};
use crate::repository::errors::RepoError;
use crate::repository::{
    ProjectMembersRepository, RequirementCommentsRepository, RequirementsRepository, UserRepository,
};
use crate::services::mentions::extract_mentions;

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
        actor: &User,
        requirement_id: i32,
        requirement_version_id: Option<i32>,
        body: String,
    ) -> Result<RequirementComment, RepoError> {
        self.create_comment_with_idempotency(
            actor,
            requirement_id,
            requirement_version_id,
            body,
            None,
        )
    }

    pub fn create_comment_with_idempotency(
        &self,
        actor: &User,
        requirement_id: i32,
        requirement_version_id: Option<i32>,
        body: String,
        mcp_idempotency_identity: Option<String>,
    ) -> Result<RequirementComment, RepoError> {
        let req = self.repo_read().get_requirement_by_id(requirement_id)?;
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
            author_id: actor.id,
            body: body.to_string(),
            mcp_idempotency_identity,
        };
        let comment = self.repo_write().insert_requirement_comment(&new)?;

        let mentioned_ids = self.mentioned_members(req.project_id, &comment.body);
        let ns = super::NotificationService::new(self.state);
        let mentioned = ns.notify_mentioned(actor, &req, &comment.body, &mentioned_ids);
        ns.notify_comment_added(actor, &req, &comment.body, &mentioned);

        Ok(comment)
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

    /// Ids of the project members mentioned in `body`. Mentions of anyone outside
    /// the project are ignored, so a comment never reveals who has an account.
    fn mentioned_members(&self, project_id: i32, body: &str) -> Vec<i32> {
        let names = extract_mentions(body);
        if names.is_empty() {
            return Vec::new();
        }
        let repo = self.repo_read();
        let members = repo.get_members_by_project(project_id).unwrap_or_default();
        let mut ids: Vec<(usize, i32)> = members
            .iter()
            .filter_map(|m| {
                let user = repo.get_user_by_id(m.user_id).ok()?;
                let pos = names
                    .iter()
                    .position(|n| n.eq_ignore_ascii_case(&user.username))?;
                Some((pos, user.id))
            })
            .collect();
        ids.sort_unstable();
        ids.into_iter().map(|(_, id)| id).collect()
    }

    fn repo_read(&self) -> std::sync::RwLockReadGuard<'_, DieselCachedRepo> {
        self.state.repo.read().expect("repo lock poisoned")
    }

    fn repo_write(&self) -> std::sync::RwLockWriteGuard<'_, DieselCachedRepo> {
        self.state.repo.write().expect("repo lock poisoned")
    }
}

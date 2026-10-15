// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Use cases (issue #398): creation, the shared working draft, publishing
//! and approval. Permission checks are done in the API layer; this service
//! enforces the document invariants, the draft's revision check and the rule
//! that an element id is never reused once a version has dropped it.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{NewUseCase, NewUseCaseVersion, UseCase, UseCaseDraft, UseCaseVersion, User};
use crate::repository::UseCasesRepository;
use crate::repository::errors::RepoError;
use crate::use_case_document::{
    DocumentError, Effect, Operation, OperationError, Stage, UseCaseDocument, apply_operations,
    validate,
};

static CODE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Z]{2,4}(?:-[A-Z0-9]{1,6})+$").unwrap());
static UC_NUMBER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^UC-(\d{1,6})$").unwrap());

#[derive(Debug)]
pub enum UseCaseError {
    Repo(RepoError),
    /// The use case, version or draft does not exist (in this project).
    NotFound(String),
    BadRequest(String),
    /// The document breaks an invariant of the format.
    InvalidDocument(Vec<DocumentError>),
    /// A draft operation cannot be applied.
    Operation(OperationError),
    /// The write was based on a draft revision that is no longer current.
    /// `current` is the draft as it is now, if there is one.
    Stale(Option<UseCaseDraft>),
    /// The request conflicts with the use case's state.
    Conflict(String),
}

impl From<RepoError> for UseCaseError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::NotFound => UseCaseError::NotFound("use case not found".into()),
            other => UseCaseError::Repo(other),
        }
    }
}

pub type UseCaseResult<T> = Result<T, UseCaseError>;

/// The next free `UC-NNN` code among `codes`.
pub fn next_code(codes: &[String]) -> String {
    let last = codes
        .iter()
        .filter_map(|c| UC_NUMBER_RE.captures(c))
        .filter_map(|c| c[1].parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    format!("UC-{:03}", last + 1)
}

pub fn parse_document(value: &serde_json::Value) -> UseCaseResult<UseCaseDocument> {
    serde_json::from_value(value.clone()).map_err(|e| {
        UseCaseError::InvalidDocument(vec![DocumentError {
            path: "document".into(),
            message: e.to_string(),
        }])
    })
}

fn to_value(doc: &UseCaseDocument) -> UseCaseResult<serde_json::Value> {
    serde_json::to_value(doc).map_err(|e| UseCaseError::Repo(RepoError::BadInput(e.to_string())))
}

fn check(doc: &UseCaseDocument, stage: Stage) -> UseCaseResult<()> {
    let errors = validate(doc, stage);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(UseCaseError::InvalidDocument(errors))
    }
}

/// Fields of a new use case.
pub struct CreateUseCase {
    pub reference_code: Option<String>,
    pub title: Option<String>,
    pub primary_actor: Option<String>,
    pub document: Option<serde_json::Value>,
}

/// A draft write: a whole document or a batch of operations.
pub enum DraftChange {
    Replace(serde_json::Value),
    Operations(Vec<Operation>),
}

pub struct UseCaseService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> UseCaseService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// The use case, if it belongs to `project_id`.
    pub fn get_in_project(&self, project_id: i32, id: i32) -> UseCaseResult<UseCase> {
        let use_case = self.state.repo_read().get_use_case(id)?;
        if use_case.project_id != project_id {
            return Err(UseCaseError::NotFound("use case not found".into()));
        }
        Ok(use_case)
    }

    /// A version, if it belongs to the use case.
    pub fn get_version(
        &self,
        use_case: &UseCase,
        version_id: i32,
    ) -> UseCaseResult<UseCaseVersion> {
        let version = self
            .state
            .repo_read()
            .get_use_case_version(version_id)
            .map_err(|e| match e {
                RepoError::NotFound => UseCaseError::NotFound("version not found".into()),
                other => other.into(),
            })?;
        if version.use_case_id != use_case.id {
            return Err(UseCaseError::NotFound("version not found".into()));
        }
        Ok(version)
    }

    pub fn current_version(&self, use_case: &UseCase) -> UseCaseResult<Option<UseCaseVersion>> {
        match use_case.current_version_id {
            Some(id) => Ok(Some(self.state.repo_read().get_use_case_version(id)?)),
            None => Ok(None),
        }
    }

    pub fn draft(&self, use_case: &UseCase) -> UseCaseResult<Option<UseCaseDraft>> {
        Ok(self.state.repo_read().get_use_case_draft(use_case.id)?)
    }

    pub fn next_code(&self, project_id: i32) -> UseCaseResult<String> {
        Ok(next_code(
            &self.state.repo_read().list_use_case_codes(project_id)?,
        ))
    }

    /// Create a use case and its first draft. No version exists until the
    /// draft is published.
    pub fn create(
        &self,
        actor: &User,
        project_id: i32,
        input: CreateUseCase,
    ) -> UseCaseResult<(UseCase, UseCaseDraft)> {
        let code = match input.reference_code.as_deref().map(str::trim) {
            Some(code) if !code.is_empty() => {
                if !CODE_RE.is_match(code) {
                    return Err(UseCaseError::BadRequest(
                        "reference_code should look like UC-001 or UC-NAV-001".into(),
                    ));
                }
                code.to_string()
            }
            _ => self.next_code(project_id)?,
        };
        let mut doc = match &input.document {
            Some(value) => parse_document(value)?,
            None => UseCaseDocument::new(input.title.as_deref().unwrap_or("").trim(), None),
        };
        if let Some(title) = input.title.as_deref().map(str::trim)
            && !title.is_empty()
        {
            doc.title = title.to_string();
        }
        if let Some(actor_name) = input.primary_actor.as_deref().map(str::trim) {
            doc.primary_actor = Some(actor_name.to_string()).filter(|a| !a.is_empty());
        }
        check(&doc, Stage::Draft)?;
        let new = NewUseCase {
            project_id,
            reference_code: code,
            created_by: Some(actor.id),
        };
        Ok(self
            .state
            .repo_write()
            .create_use_case(&new, &to_value(&doc)?)?)
    }

    pub fn delete(&self, use_case: &UseCase) -> UseCaseResult<()> {
        self.state.repo_write().delete_use_case(use_case.id)?;
        Ok(())
    }

    /// Ids that a version of this use case used and `base` (the version the
    /// draft started from) does not contain: they may not come back.
    fn retired_ids(
        &self,
        use_case: &UseCase,
        base: Option<&UseCaseDocument>,
    ) -> UseCaseResult<HashSet<String>> {
        let keep: HashSet<&str> = base
            .map(|d| d.ids().into_iter().collect())
            .unwrap_or_default();
        let mut retired = HashSet::new();
        for version in self.state.repo_read().list_use_case_versions(use_case.id)? {
            if let Ok(doc) = serde_json::from_value::<UseCaseDocument>(version.document) {
                retired.extend(
                    doc.ids()
                        .into_iter()
                        .filter(|id| !keep.contains(id))
                        .map(str::to_string),
                );
            }
        }
        Ok(retired)
    }

    /// Change the draft, or open one from the current version. `base_revision`
    /// must be the draft's current revision, or `None` when there is no draft.
    pub fn change_draft(
        &self,
        actor: &User,
        use_case: &UseCase,
        base_revision: Option<i32>,
        change: DraftChange,
    ) -> UseCaseResult<(UseCaseDraft, Vec<Effect>)> {
        let existing = self.draft(use_case)?;
        let current = self.current_version(use_case)?;
        if existing.as_ref().map(|d| d.revision) != base_revision {
            return Err(UseCaseError::Stale(existing));
        }
        let (old_doc, base_version_id) = match &existing {
            Some(draft) => (parse_document(&draft.document)?, draft.base_version_id),
            None => match &current {
                Some(version) => (parse_document(&version.document)?, Some(version.id)),
                None => {
                    return Err(UseCaseError::Conflict(
                        "this use case has neither a draft nor a version".into(),
                    ));
                }
            },
        };
        let (new_doc, effects) = match change {
            DraftChange::Replace(value) => (parse_document(&value)?, Vec::new()),
            DraftChange::Operations(ops) => {
                let mut doc = old_doc.clone();
                let effects = apply_operations(&mut doc, ops).map_err(UseCaseError::Operation)?;
                (doc, effects)
            }
        };
        check(&new_doc, Stage::Draft)?;

        let base_doc = match base_version_id {
            Some(id) => Some(parse_document(
                &self.state.repo_read().get_use_case_version(id)?.document,
            )?),
            None => None,
        };
        let retired = self.retired_ids(use_case, base_doc.as_ref())?;
        let old_ids: HashSet<&str> = old_doc.ids().into_iter().collect();
        let reused: Vec<DocumentError> = new_doc
            .ids()
            .into_iter()
            .filter(|id| !old_ids.contains(id) && retired.contains(*id))
            .map(|id| DocumentError {
                path: id.to_string(),
                message: "this id belonged to an element removed in an earlier version and cannot be reused".into(),
            })
            .collect();
        if !reused.is_empty() {
            return Err(UseCaseError::InvalidDocument(reused));
        }

        let saved = self.state.repo_write().save_use_case_draft(
            use_case.id,
            base_revision,
            base_version_id,
            &to_value(&new_doc)?,
            actor.id,
        )?;
        match saved {
            Some(draft) => Ok((draft, effects)),
            None => Err(UseCaseError::Stale(self.draft(use_case)?)),
        }
    }

    /// Discard the draft. A use case that was never published keeps its
    /// draft: delete the use case instead.
    pub fn discard_draft(&self, use_case: &UseCase) -> UseCaseResult<()> {
        if use_case.current_version_id.is_none() {
            return Err(UseCaseError::Conflict(
                "this use case has no published version; delete the use case instead".into(),
            ));
        }
        if !self.state.repo_write().delete_use_case_draft(use_case.id)? {
            return Err(UseCaseError::NotFound("there is no draft".into()));
        }
        Ok(())
    }

    /// Publish the draft at `base_revision` as the next version.
    pub fn publish(
        &self,
        actor: &User,
        use_case: &UseCase,
        base_revision: i32,
        change_note: Option<String>,
    ) -> UseCaseResult<UseCaseVersion> {
        let Some(draft) = self.draft(use_case)? else {
            return Err(UseCaseError::NotFound("there is no draft".into()));
        };
        if draft.revision != base_revision {
            return Err(UseCaseError::Stale(Some(draft)));
        }
        let doc = parse_document(&draft.document)?;
        check(&doc, Stage::Publish)?;
        let version = NewUseCaseVersion {
            use_case_id: use_case.id,
            document: draft.document.clone(),
            title: doc.title.trim().to_string(),
            primary_actor: doc.primary_actor.clone(),
            change_note: change_note
                .map(|n| n.trim().to_string())
                .filter(|n| !n.is_empty()),
            created_by: actor.id,
        };
        match self
            .state
            .repo_write()
            .publish_use_case_draft(base_revision, &version)?
        {
            Some(version) => Ok(version),
            None => Err(UseCaseError::Stale(self.draft(use_case)?)),
        }
    }

    pub fn set_approval(
        &self,
        actor: &User,
        version: &UseCaseVersion,
        state: &str,
    ) -> UseCaseResult<UseCaseVersion> {
        let state = state.trim();
        if state != "reviewed" && state != "approved" {
            return Err(UseCaseError::BadRequest(
                "state must be 'reviewed' or 'approved'".into(),
            ));
        }
        Ok(self
            .state
            .repo_write()
            .set_use_case_version_approval(version.id, state, actor.id)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_code_follows_the_highest_uc_number() {
        assert_eq!(next_code(&[]), "UC-001");
        assert_eq!(
            next_code(&["UC-002".into(), "UC-010".into(), "UC-NAV-099".into()]),
            "UC-011"
        );
    }
}

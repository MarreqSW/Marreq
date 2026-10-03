// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Verification methods (Test, Analysis, Inspection, …) of a project.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{NewVerificationMethod, User, VerificationMethod};
use crate::permissions::Permission;
use crate::repository::LookupRepository;
use crate::repository::errors::RepoError;
use crate::services::AuditLog;
use crate::services::project_service::ProjectBootstrap;

pub struct VerificationMethodService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> VerificationMethodService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve the verification methods of a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<VerificationMethod>, RepoError> {
        self.state
            .repo_read()
            .get_verification_methods_by_project(project_id)
    }

    /// Retrieve a single verification method by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<VerificationMethod, RepoError> {
        self.state.repo_read().get_verification_method_by_id(id)
    }

    fn require_manage(&self, actor: &User, project_id: i32) -> Result<(), RepoError> {
        crate::authorization::require_project_permission_for_service(
            &*self.state.repo_read(),
            actor,
            project_id,
            Permission::ManageProjectConfiguration,
        )
    }

    /// Create a verification method and log the action. Requires
    /// `ManageProjectConfiguration` on the target project.
    pub fn create(&self, actor: &User, payload: NewVerificationMethod) -> Result<i32, RepoError> {
        self.require_manage(actor, payload.project_id)?;
        let id = self.insert(&payload)?;
        self.audit_created(actor, id, &payload);
        Ok(id)
    }

    /// Seed a default verification method for a project that was just created.
    pub(crate) fn create_bootstrap(
        &self,
        bootstrap: &ProjectBootstrap,
        payload: NewVerificationMethod,
    ) -> Result<i32, RepoError> {
        bootstrap.check(payload.project_id)?;
        self.insert(&payload)
    }

    fn insert(&self, payload: &NewVerificationMethod) -> Result<i32, RepoError> {
        self.state
            .repo_write()
            .insert_new_verification_method(payload)
    }

    /// Update a verification method and log the change. Authorized against the
    /// stored project, which cannot be changed.
    pub fn update(
        &self,
        actor: &User,
        id: i32,
        mut payload: NewVerificationMethod,
    ) -> Result<VerificationMethod, RepoError> {
        let before = self.get_by_id(id)?;
        self.require_manage(actor, before.project_id)?;
        if payload.project_id != before.project_id {
            return Err(RepoError::CrossProjectViolation(
                "verification method project cannot be changed".into(),
            ));
        }

        payload.id = Some(id);
        if !self.state.repo_write().edit_verification_method(&payload)? {
            return Err(RepoError::NotFound);
        }

        let after = self.get_by_id(id)?;
        self.audit_updated(actor, &before, &after);
        Ok(after)
    }

    /// Delete a verification method and log the removal. Authorized against the
    /// stored project.
    pub fn delete(&self, actor: &User, id: i32) -> Result<VerificationMethod, RepoError> {
        let existing = self.get_by_id(id)?;
        self.require_manage(actor, existing.project_id)?;
        let deleted = self.state.repo_write().delete_verification_method(id)?;
        self.audit_deleted(actor, &deleted);
        Ok(deleted)
    }
}

impl AuditLog for VerificationMethodService<'_> {
    fn app_state(&self) -> &AppState<DieselCachedRepo> {
        self.state
    }
}

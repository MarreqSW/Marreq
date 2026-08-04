// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Service for project-scoped saved views.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{SavedView, SavedViewPayload};
use crate::repository::errors::RepoError;
use crate::repository::SavedViewRepository;

pub struct SavedViewService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> SavedViewService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    pub fn list_for_user(
        &self,
        project_id: i32,
        user_id: i32,
    ) -> Result<Vec<SavedView>, RepoError> {
        self.state
            .repo_read()
            .list_saved_views_for_user(project_id, user_id)
    }

    pub fn get_by_id(&self, id: i32) -> Result<SavedView, RepoError> {
        self.state.repo_read().get_saved_view_by_id(id)
    }

    pub fn create(
        &self,
        project_id: i32,
        owner_id: i32,
        payload: SavedViewPayload,
    ) -> Result<SavedView, RepoError> {
        self.state
            .repo_write()
            .create_saved_view(project_id, owner_id, &payload)
    }

    pub fn update(&self, id: i32, payload: SavedViewPayload) -> Result<SavedView, RepoError> {
        self.state.repo_write().update_saved_view(id, &payload)
    }

    pub fn delete(&self, id: i32) -> Result<(), RepoError> {
        self.state.repo_write().delete_saved_view(id)
    }

    pub fn lock(&self, id: i32) -> Result<(), RepoError> {
        self.state.repo_write().lock_saved_view(id)
    }
}

//! Service for immutable project baselines.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{Baseline, BaselineTraceability, NewBaseline, Requirement};
use crate::repository::errors::RepoError;
use crate::repository::BaselineRepository;

/// Baseline operations backed by the shared [`AppState`].
pub struct BaselineService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> BaselineService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Create an immutable baseline for the project (snapshot of current requirement versions and traceability).
    pub fn create_baseline(
        &self,
        project_id: i32,
        created_by: i32,
        payload: &NewBaseline,
    ) -> Result<Baseline, RepoError> {
        self.state
            .repo_write()
            .create_baseline(project_id, created_by, payload)
    }

    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<Baseline>, RepoError> {
        self.state.repo_read().list_baselines_by_project(project_id)
    }

    pub fn get_by_id(&self, baseline_id: i32) -> Result<Baseline, RepoError> {
        self.state.repo_read().get_baseline_by_id(baseline_id)
    }

    /// Requirements as at baseline time (from snapshot).
    pub fn get_requirements(&self, baseline_id: i32) -> Result<Vec<Requirement>, RepoError> {
        self.state
            .repo_read()
            .get_requirements_for_baseline(baseline_id)
    }

    pub fn get_traceability(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<BaselineTraceability>, RepoError> {
        self.state
            .repo_read()
            .get_baseline_traceability(baseline_id)
    }
}

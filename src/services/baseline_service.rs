// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Service for immutable project baselines.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{
    Baseline, BaselineTraceability, BaselineVerification, NewBaseline, Requirement,
};
use crate::repository::errors::RepoError;
use crate::repository::BaselineRepository;
use serde::Serialize;
use std::collections::HashSet;

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

    /// Verifications as at baseline time (from snapshot).
    pub fn get_verifications(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<BaselineVerification>, RepoError> {
        self.state
            .repo_read()
            .get_verifications_for_baseline(baseline_id)
    }

    /// Compare two baselines: requirements and traceability only in A, only in B.
    pub fn diff_baselines(
        &self,
        project_id: i32,
        baseline_a_id: i32,
        baseline_b_id: i32,
    ) -> Result<BaselineDiff, RepoError> {
        let bl_a = self.get_by_id(baseline_a_id)?;
        let bl_b = self.get_by_id(baseline_b_id)?;
        if bl_a.project_id != project_id || bl_b.project_id != project_id {
            return Err(RepoError::NotFound);
        }

        let reqs_a = self.get_requirements(baseline_a_id)?;
        let reqs_b = self.get_requirements(baseline_b_id)?;
        let ids_a: HashSet<i32> = reqs_a.iter().map(|r| r.id).collect();
        let ids_b: HashSet<i32> = reqs_b.iter().map(|r| r.id).collect();
        let requirements_only_in_a: Vec<i32> = ids_a.difference(&ids_b).copied().collect();
        let requirements_only_in_b: Vec<i32> = ids_b.difference(&ids_a).copied().collect();

        let trace_a = self.get_traceability(baseline_a_id)?;
        let trace_b = self.get_traceability(baseline_b_id)?;
        let set_a: HashSet<(i32, i32)> = trace_a
            .iter()
            .map(|t| (t.requirement_id, t.verification_id))
            .collect();
        let set_b: HashSet<(i32, i32)> = trace_b
            .iter()
            .map(|t| (t.requirement_id, t.verification_id))
            .collect();
        let trace_only_in_a: Vec<[i32; 2]> =
            set_a.difference(&set_b).map(|&(r, t)| [r, t]).collect();
        let trace_only_in_b: Vec<[i32; 2]> =
            set_b.difference(&set_a).map(|&(r, t)| [r, t]).collect();

        Ok(BaselineDiff {
            requirements_only_in_a,
            requirements_only_in_b,
            trace_only_in_a,
            trace_only_in_b,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BaselineDiff {
    pub requirements_only_in_a: Vec<i32>,
    pub requirements_only_in_b: Vec<i32>,
    pub trace_only_in_a: Vec<[i32; 2]>,
    pub trace_only_in_b: Vec<[i32; 2]>,
}

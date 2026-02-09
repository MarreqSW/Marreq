//! Service for computing diffs between two requirement versions (read-only, deterministic).

use crate::app::{AppState, DieselCachedRepo};
use crate::diff::{compute_requirement_diff, RequirementDiff};
use crate::models::Requirement;
use crate::repository::errors::RepoError;
use crate::repository::{BaselineRepository, LookupRepository, RequirementsRepository};

/// Service for requirement version diffs. Read-only; does not modify any data.
pub struct RequirementDiffService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> RequirementDiffService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    fn repo_read(&self) -> std::sync::RwLockReadGuard<'_, DieselCachedRepo> {
        self.state.repo.read().expect("repo lock poisoned")
    }

    /// Diff two versions of a requirement. Both version IDs must belong to `req_id`.
    /// Returns the structured diff (v1 = old, v2 = new) or NotFound if either version is missing or does not belong to the requirement.
    pub fn diff_versions(
        &self,
        req_id: i32,
        v1_id: i32,
        v2_id: i32,
    ) -> Result<RequirementDiff, RepoError> {
        let repo = self.repo_read();
        let v1 = repo.get_requirement_version_by_id(v1_id)?;
        let v2 = repo.get_requirement_version_by_id(v2_id)?;
        if v1.requirement_id != req_id || v2.requirement_id != req_id {
            return Err(RepoError::NotFound);
        }
        let verification_v1 = repo.get_verification_method_ids_for_version(v1_id)?;
        let verification_v2 = repo.get_verification_method_ids_for_version(v2_id)?;
        let mut diff = compute_requirement_diff(&v1, &v2, &verification_v1, &verification_v2);
        drop(repo);
        self.enrich_diff_with_labels(&mut diff);
        Ok(diff)
    }

    /// Diff the requirement as stored in the baseline vs the current version.
    /// Returns NotFound if baseline missing, requirement not in baseline, or requirement has no current version.
    pub fn diff_baseline_vs_current(
        &self,
        project_id: i32,
        baseline_id: i32,
        req_id: i32,
    ) -> Result<RequirementDiff, RepoError> {
        let repo = self.repo_read();
        let baseline = repo.get_baseline_by_id(baseline_id)?;
        if baseline.project_id != project_id {
            return Err(RepoError::NotFound);
        }
        let baseline_version_id = repo
            .get_baseline_requirement_version_id(baseline_id, req_id)?
            .ok_or(RepoError::NotFound)?;
        let requirement: Requirement = repo.get_requirement_by_id(req_id)?;
        let current_version_id = requirement.current_version_id.ok_or(RepoError::NotFound)?;
        let v1 = repo.get_requirement_version_by_id(baseline_version_id)?;
        let v2 = repo.get_requirement_version_by_id(current_version_id)?;
        let verification_v1 = repo.get_verification_method_ids_for_version(baseline_version_id)?;
        let verification_v2 = repo.get_verification_method_ids_for_version(current_version_id)?;
        let mut diff = compute_requirement_diff(&v1, &v2, &verification_v1, &verification_v2);
        drop(repo);
        self.enrich_diff_with_labels(&mut diff);
        Ok(diff)
    }

    /// Resolve metadata IDs to human-readable labels for display.
    fn enrich_diff_with_labels(&self, diff: &mut RequirementDiff) {
        let repo = self.repo_read();

        if let Some(id) = diff.metadata.status.unchanged {
            diff.metadata.status.unchanged_label =
                repo.get_requirement_status_by_id(id).ok().map(|s| s.title);
        }
        if let Some(id) = diff.metadata.status.old_id {
            diff.metadata.status.old_label =
                repo.get_requirement_status_by_id(id).ok().map(|s| s.title);
        }
        if let Some(id) = diff.metadata.status.new_id {
            diff.metadata.status.new_label =
                repo.get_requirement_status_by_id(id).ok().map(|s| s.title);
        }

        if let Some(id) = diff.metadata.category.unchanged {
            diff.metadata.category.unchanged_label =
                repo.get_category_by_id(id).ok().map(|c| c.title.clone());
        }
        if let Some(id) = diff.metadata.category.old_id {
            diff.metadata.category.old_label =
                repo.get_category_by_id(id).ok().map(|c| c.title.clone());
        }
        if let Some(id) = diff.metadata.category.new_id {
            diff.metadata.category.new_label =
                repo.get_category_by_id(id).ok().map(|c| c.title.clone());
        }

        if let Some(id) = diff.metadata.applicability.unchanged {
            diff.metadata.applicability.unchanged_label = repo
                .get_applicability_by_id(id)
                .ok()
                .map(|a| a.title.clone());
        }
        if let Some(id) = diff.metadata.applicability.old_id {
            diff.metadata.applicability.old_label = repo
                .get_applicability_by_id(id)
                .ok()
                .map(|a| a.title.clone());
        }
        if let Some(id) = diff.metadata.applicability.new_id {
            diff.metadata.applicability.new_label = repo
                .get_applicability_by_id(id)
                .ok()
                .map(|a| a.title.clone());
        }

        diff.metadata.verification.added_labels = Some(
            diff.metadata
                .verification
                .added_ids
                .iter()
                .filter_map(|&id| {
                    repo.get_verification_by_id(id)
                        .ok()
                        .map(|v| v.title.clone())
                })
                .collect(),
        );
        diff.metadata.verification.removed_labels = Some(
            diff.metadata
                .verification
                .removed_ids
                .iter()
                .filter_map(|&id| {
                    repo.get_verification_by_id(id)
                        .ok()
                        .map(|v| v.title.clone())
                })
                .collect(),
        );
        diff.metadata.verification.unchanged_labels = Some(
            diff.metadata
                .verification
                .unchanged_ids
                .iter()
                .filter_map(|&id| {
                    repo.get_verification_by_id(id)
                        .ok()
                        .map(|v| v.title.clone())
                })
                .collect(),
        );
    }
}

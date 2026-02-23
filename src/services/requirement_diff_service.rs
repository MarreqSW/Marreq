// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppState, DieselCachedRepo};
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::repository::CacheRepository;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::sync::{Arc, RwLock};

    fn epoch() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn state_with_repo(repo: DieselRepoMock) -> AppState<DieselCachedRepo> {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    #[test]
    fn service_new_constructs() {
        let mock = DieselRepoMock::default();
        let state = state_with_repo(mock);
        let _service = RequirementDiffService::new(&state);
    }

    #[test]
    fn diff_versions_returns_ok_when_both_versions_belong_to_requirement() {
        let mut mock = DieselRepoMock::default();
        mock.requirements.insert(
            1,
            Requirement {
                id: 1,
                current_version_id: Some(11),
                same_as_current: None,
                title: "R".into(),
                description: "D".into(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "R-1".into(),
                category_id: 1,
                parent_id: None,
                creation_date: epoch(),
                update_date: epoch(),
                deadline_date: None,
                applicability_id: 1,
                justification: None,
                project_id: 1,
                approval_state: "draft".into(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            },
        );
        let v10 = crate::models::RequirementVersion {
            id: 10,
            requirement_id: 1,
            title: "Old".into(),
            description: "D".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            category_id: 1,
            parent_id: None,
            applicability_id: 1,
            justification: None,
            deadline_date: None,
            created_at: epoch(),
            approval_state: "draft".into(),
            approved_by: None,
            approved_at: None,
        };
        let v11 = crate::models::RequirementVersion {
            id: 11,
            requirement_id: 1,
            title: "New".into(),
            description: "D".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            category_id: 1,
            parent_id: None,
            applicability_id: 1,
            justification: None,
            deadline_date: None,
            created_at: epoch(),
            approval_state: "draft".into(),
            approved_by: None,
            approved_at: None,
        };
        mock.requirement_versions.insert(10, v10);
        mock.requirement_versions.insert(11, v11);
        let state = state_with_repo(mock);
        let service = RequirementDiffService::new(&state);
        let diff = service.diff_versions(1, 10, 11).unwrap();
        assert_eq!(diff.text.title.removed, vec!["Old"]);
        assert_eq!(diff.text.title.added, vec!["New"]);
    }

    #[test]
    fn diff_versions_returns_not_found_when_version_belongs_to_different_requirement() {
        let mut mock = DieselRepoMock::default();
        mock.requirement_versions.insert(
            10,
            crate::models::RequirementVersion {
                id: 10,
                requirement_id: 1,
                title: "A".into(),
                description: "".into(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                category_id: 1,
                parent_id: None,
                applicability_id: 1,
                justification: None,
                deadline_date: None,
                created_at: epoch(),
                approval_state: "draft".into(),
                approved_by: None,
                approved_at: None,
            },
        );
        mock.requirement_versions.insert(
            20,
            crate::models::RequirementVersion {
                id: 20,
                requirement_id: 2,
                title: "B".into(),
                description: "".into(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                category_id: 1,
                parent_id: None,
                applicability_id: 1,
                justification: None,
                deadline_date: None,
                created_at: epoch(),
                approval_state: "draft".into(),
                approved_by: None,
                approved_at: None,
            },
        );
        let state = state_with_repo(mock);
        let service = RequirementDiffService::new(&state);
        let result = service.diff_versions(1, 10, 20);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::NotFound));
    }

    #[test]
    fn diff_baseline_vs_current_returns_not_found_when_baseline_wrong_project() {
        let mut mock = DieselRepoMock::default();
        mock.baselines.push(crate::models::Baseline {
            id: 1,
            project_id: 2,
            name: "v1".into(),
            description: None,
            created_at: epoch(),
            created_by: 1,
        });
        let state = state_with_repo(mock);
        let service = RequirementDiffService::new(&state);
        let result = service.diff_baseline_vs_current(1, 1, 1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::NotFound));
    }
}

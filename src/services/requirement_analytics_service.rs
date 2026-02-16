//! Service providing aggregated requirement metrics.
//!
//! Encapsulates the logic for computing per-status counts and coverage figures
//! so Rocket routes can stay focused on HTTP concerns.

use crate::app::{AppState, DieselCachedRepo};
use crate::repository::errors::RepoError;
use crate::repository::{LookupRepository, RequirementsRepository};
use crate::status_enums::RequirementStatusEnum;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer, Nullable, Text};
use serde::Serialize;
use std::collections::HashMap;

/// Aggregated requirement metrics for a project scope.
///
/// # Coverage Calculation
///
/// The `coverage_verified` and `coverage_percent` fields represent requirements coverage,
/// which is based on the `Accepted` status only. According to the canonical status definitions:
///
/// - **coverage_verified**: Count of requirements with status = "Accepted" (ID 3)
/// - **coverage_percent**: `(accepted / total) * 100`, rounded to nearest integer
///
/// Only "Accepted" requirements count toward coverage because they represent requirements
/// that have been formally approved and must be processed. Other statuses (Draft, Proposal,
/// Rejected, Cancelled, Finished) do not contribute to the coverage metric.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RequirementMetrics {
    pub total: i64,
    pub draft: i64,
    pub accepted: i64,
    pub rejected: i64,
    pub coverage_verified: i64,
    pub coverage_percent: i32,
}

/// Analytics helpers backed by the shared [`AppState`].
pub struct RequirementAnalyticsService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> RequirementAnalyticsService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Compute requirement metrics for the given project and optional filters.
    pub fn metrics(
        &self,
        project_id: i32,
        status_filter: Option<i32>,
        verification_filter: Option<i32>,
        category_filter: Option<i32>,
        applicability_filter: Option<i32>,
    ) -> Result<RequirementMetrics, RepoError> {
        // Try to use the optimized SQL path first.
        match self.metrics_via_sql(
            project_id,
            status_filter,
            verification_filter,
            category_filter,
            applicability_filter,
        ) {
            Ok(metrics) => Ok(metrics),
            Err(RepoError::Pool(_)) => {
                // Fall back to repository reads when no direct connection exists (e.g. tests).
                self.metrics_via_repository(
                    project_id,
                    status_filter,
                    verification_filter,
                    category_filter,
                    applicability_filter,
                )
            }
            Err(err) => Err(err),
        }
    }

    fn metrics_via_sql(
        &self,
        project_id: i32,
        status_filter: Option<i32>,
        verification_filter: Option<i32>,
        category_filter: Option<i32>,
        applicability_filter: Option<i32>,
    ) -> Result<RequirementMetrics, RepoError> {
        // Acquire a database connection from the underlying repository.
        let mut conn = {
            let repo_guard = self.state.repo_read();
            repo_guard.inner_repo().get_conn()?
        };

        #[derive(QueryableByName)]
        struct StatusAggregate {
            #[diesel(sql_type = Text)]
            status: String,
            #[diesel(sql_type = BigInt)]
            total: i64,
        }

        // Execute the grouped aggregation with optional filters expressed as bind parameters.
        // Verification filter uses requirement_version_verification_methods (current version).
        let query = diesel::sql_query(
            "SELECT LOWER(TRIM(rs.title)) AS status, COUNT(*)::BIGINT AS total
             FROM requirements r
             INNER JOIN requirement_versions rv ON r.current_version_id = rv.id
             INNER JOIN requirement_status rs ON rs.id = rv.status_id
             WHERE r.project_id = $1
               AND ($2 IS NULL OR rv.status_id = $2)
               AND ($3 IS NULL OR r.current_version_id IN (SELECT requirement_version_id FROM requirement_version_verification_methods WHERE verification_method_id = $3))
               AND ($4 IS NULL OR rv.category_id = $4)
               AND ($5 IS NULL OR rv.applicability_id = $5)
             GROUP BY status",
        );

        let aggregated: Vec<StatusAggregate> = query
            .bind::<Integer, _>(project_id)
            .bind::<Nullable<Integer>, _>(status_filter)
            .bind::<Nullable<Integer>, _>(verification_filter)
            .bind::<Nullable<Integer>, _>(category_filter)
            .bind::<Nullable<Integer>, _>(applicability_filter)
            .load(conn.as_mut())?;

        let counts = aggregated.into_iter().map(|row| (row.status, row.total));

        Ok(Self::build_metrics(counts))
    }

    fn metrics_via_repository(
        &self,
        project_id: i32,
        status_filter: Option<i32>,
        verification_filter: Option<i32>,
        category_filter: Option<i32>,
        applicability_filter: Option<i32>,
    ) -> Result<RequirementMetrics, RepoError> {
        // Gather requirements and statuses through the cached repository.
        let repo_guard = self.state.repo_read();
        let requirements = repo_guard.get_requirements_by_project(project_id)?;
        let statuses = repo_guard.get_requirement_status_all()?;
        let verification_requirement_ids = verification_filter
            .map(|vid| {
                repo_guard
                    .get_requirement_ids_by_verification_method(vid)
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        drop(repo_guard);

        let status_lookup: HashMap<i32, String> = statuses
            .into_iter()
            .map(|status| (status.id, status.title))
            .collect();

        let mut counts: HashMap<String, i64> = HashMap::new();

        // Apply the same filtering semantics as the SQL path.
        for requirement in requirements {
            if let Some(filter) = status_filter {
                if requirement.status_id != filter {
                    continue;
                }
            }
            if let Some(_filter) = verification_filter {
                if !verification_requirement_ids.contains(&requirement.id) {
                    continue;
                }
            }
            if let Some(filter) = category_filter {
                if requirement.category_id != filter {
                    continue;
                }
            }
            if let Some(filter) = applicability_filter {
                if requirement.applicability_id != filter {
                    continue;
                }
            }

            let title = status_lookup
                .get(&requirement.status_id)
                .map(|name| name.trim().to_string())
                .unwrap_or_else(|| format!("Unknown Status ({})", requirement.status_id));

            *counts.entry(title).or_insert(0) += 1;
        }

        Ok(Self::build_metrics(counts))
    }

    fn build_metrics<I>(counts: I) -> RequirementMetrics
    where
        I: IntoIterator<Item = (String, i64)>,
    {
        let mut metrics = RequirementMetrics::default();

        // Tally totals and the statuses we care about.
        for (status, count) in counts {
            metrics.total += count;

            // Use the enum to determine status type consistently
            if let Some(status_enum) = RequirementStatusEnum::from_title(&status) {
                match status_enum {
                    RequirementStatusEnum::Draft => metrics.draft += count,
                    RequirementStatusEnum::Accepted => metrics.accepted += count,
                    RequirementStatusEnum::Rejected => metrics.rejected += count,
                    _ => {}
                }
            }
        }

        // For coverage calculation, only Accepted requirements are considered verified
        metrics.coverage_verified = metrics.accepted;
        metrics.coverage_percent = if metrics.total > 0 {
            ((metrics.accepted as f64 / metrics.total as f64) * 100.0).round() as i32
        } else {
            0
        };

        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppState, DieselCachedRepo};
    use crate::models::{Requirement, RequirementStatus};
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::sync::{Arc, RwLock};

    fn state_with_repo(repo: DieselRepoMock) -> AppState<DieselCachedRepo> {
        AppState {
            repo: Arc::new(RwLock::new(DieselCachedRepo::new(repo, 0))),
        }
    }

    fn naive_datetime() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn make_requirement(
        id: i32,
        project_id: i32,
        status_id: i32,
        _verification_method_id: i32,
        category_id: i32,
    ) -> Requirement {
        Requirement {
            id,
            current_version_id: None,
            same_as_current: None,
            title: format!("Req {id}"),
            description: "desc".into(),
            status_id,
            author_id: 1,
            reviewer_id: 1,
            reference_code: format!("REF-{id}"),
            category_id,
            parent_id: None,
            creation_date: naive_datetime(),
            update_date: naive_datetime(),
            deadline_date: Some(naive_datetime()),
            applicability_id: 1,
            justification: None,
            project_id,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        }
    }

    fn status(id: i32, title: &str) -> RequirementStatus {
        RequirementStatus {
            id,
            title: title.into(),
            description: format!("{title} description"),
            tag: title.chars().take(3).collect(),
            project_id: 1,
            is_system: false,
            tag_color: None,
        }
    }

    #[test]
    fn metrics_from_repository_matches_counts() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(1, status(1, "Draft"));
        repo.requirement_statuses.insert(2, status(2, "Accepted"));
        repo.requirement_statuses.insert(3, status(3, "Rejected"));

        repo.requirements
            .insert(1, make_requirement(1, 1, 1, 1, 10));
        repo.requirements
            .insert(2, make_requirement(2, 1, 2, 1, 10));
        repo.requirements
            .insert(3, make_requirement(3, 1, 2, 1, 11));
        repo.requirements
            .insert(4, make_requirement(4, 1, 3, 2, 11));

        let state = state_with_repo(repo);
        let service = RequirementAnalyticsService::new(&state);

        let metrics = service
            .metrics(1, None, None, None, None)
            .expect("metrics should be computed");

        assert_eq!(metrics.total, 4);
        assert_eq!(metrics.draft, 1);
        assert_eq!(metrics.accepted, 2);
        assert_eq!(metrics.rejected, 1);
        assert_eq!(metrics.coverage_verified, 2);
        assert_eq!(metrics.coverage_percent, 50);
    }

    #[test]
    fn metrics_respect_filters() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(1, status(1, "Draft"));
        repo.requirement_statuses.insert(2, status(2, "Accepted"));

        repo.requirements
            .insert(1, make_requirement(1, 1, 1, 1, 10));
        repo.requirements
            .insert(2, make_requirement(2, 1, 2, 1, 11));
        repo.requirements
            .insert(3, make_requirement(3, 1, 2, 2, 10));
        repo.requirement_verification_methods.push((1, 1));
        repo.requirement_verification_methods.push((2, 5));
        repo.requirement_verification_methods.push((3, 2));

        let state = state_with_repo(repo);
        let service = RequirementAnalyticsService::new(&state);

        let status_filtered = service
            .metrics(1, Some(2), None, None, None)
            .expect("status filtered metrics");
        assert_eq!(status_filtered.total, 2);
        assert_eq!(status_filtered.accepted, 2);
        assert_eq!(status_filtered.coverage_percent, 100);

        let verification_filtered = service
            .metrics(1, None, Some(2), None, None)
            .expect("verification filtered metrics");
        assert_eq!(verification_filtered.total, 1);
        assert_eq!(verification_filtered.accepted, 1);

        let category_filtered = service
            .metrics(1, None, None, Some(10), None)
            .expect("category filtered metrics");
        assert_eq!(category_filtered.total, 2);
    }

    /// Verification filter uses the junction table: a requirement linked to multiple
    /// verification methods is counted when filtering by any of those methods.
    #[test]
    fn metrics_verification_filter_counts_requirement_with_multiple_verification_methods() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(1, status(1, "Draft"));
        repo.requirement_statuses.insert(2, status(2, "Accepted"));

        repo.requirements
            .insert(1, make_requirement(1, 1, 2, 1, 10)); // Accepted
        repo.requirements
            .insert(2, make_requirement(2, 1, 2, 1, 10)); // Accepted
                                                          // Req 1 has both verification 1 and 2; Req 2 has only verification 1
        repo.requirement_verification_methods.push((1, 1));
        repo.requirement_verification_methods.push((1, 2));
        repo.requirement_verification_methods.push((2, 1));

        let state = state_with_repo(repo);
        let service = RequirementAnalyticsService::new(&state);

        let by_verification_1 = service
            .metrics(1, None, Some(1), None, None)
            .expect("filter by verification 1");
        assert_eq!(by_verification_1.total, 2, "both reqs have verification 1");

        let by_verification_2 = service
            .metrics(1, None, Some(2), None, None)
            .expect("filter by verification 2");
        assert_eq!(by_verification_2.total, 1, "only req 1 has verification 2");
    }

    #[test]
    fn metrics_returns_zero_for_empty_project() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = RequirementAnalyticsService::new(&state);

        let metrics = service.metrics(999, None, None, None, None).unwrap();
        assert_eq!(metrics.total, 0);
        assert_eq!(metrics.draft, 0);
        assert_eq!(metrics.accepted, 0);
        assert_eq!(metrics.rejected, 0);
        assert_eq!(metrics.coverage_verified, 0);
        assert_eq!(metrics.coverage_percent, 0);
    }

    #[test]
    fn metrics_computes_coverage_percent_correctly() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(1, status(1, "Draft"));
        repo.requirement_statuses.insert(2, status(2, "Accepted"));

        // 3 accepted out of 10 total = 30%
        for i in 1..=3 {
            repo.requirements
                .insert(i, make_requirement(i, 1, 2, 1, 10)); // Accepted
        }
        for i in 4..=10 {
            repo.requirements
                .insert(i, make_requirement(i, 1, 1, 1, 10)); // Draft
        }

        let state = state_with_repo(repo);
        let service = RequirementAnalyticsService::new(&state);

        let metrics = service.metrics(1, None, None, None, None).unwrap();
        assert_eq!(metrics.total, 10);
        assert_eq!(metrics.accepted, 3);
        assert_eq!(metrics.coverage_verified, 3);
        assert_eq!(metrics.coverage_percent, 30);
    }

    #[test]
    fn metrics_handles_100_percent_coverage() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(2, status(2, "Accepted"));

        for i in 1..=5 {
            repo.requirements
                .insert(i, make_requirement(i, 1, 2, 1, 10)); // All Accepted
        }

        let state = state_with_repo(repo);
        let service = RequirementAnalyticsService::new(&state);

        let metrics = service.metrics(1, None, None, None, None).unwrap();
        assert_eq!(metrics.total, 5);
        assert_eq!(metrics.accepted, 5);
        assert_eq!(metrics.coverage_percent, 100);
    }

    #[test]
    fn metrics_handles_unknown_status() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(1, status(1, "Draft"));
        // Requirement with status_id that doesn't exist in statuses
        repo.requirements
            .insert(1, make_requirement(1, 1, 999, 1, 10));

        let state = state_with_repo(repo);
        let service = RequirementAnalyticsService::new(&state);

        let metrics = service.metrics(1, None, None, None, None).unwrap();
        assert_eq!(metrics.total, 1);
        // Unknown status should not be counted in draft/accepted/rejected
        assert_eq!(metrics.draft, 0);
        assert_eq!(metrics.accepted, 0);
        assert_eq!(metrics.rejected, 0);
    }

    #[test]
    fn metrics_applies_multiple_filters() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(2, status(2, "Accepted"));

        // Only this one matches all filters
        repo.requirements
            .insert(1, make_requirement(1, 1, 2, 5, 10)); // status=2, verification=5, category=10
        repo.requirements
            .insert(2, make_requirement(2, 1, 2, 5, 11)); // status=2, verification=5, category=11 (wrong)
        repo.requirements
            .insert(3, make_requirement(3, 1, 2, 6, 10)); // status=2, verification=6 (wrong), category=10
        repo.requirement_verification_methods.push((1, 5));
        repo.requirement_verification_methods.push((2, 5));
        repo.requirement_verification_methods.push((3, 6));

        let state = state_with_repo(repo);
        let service = RequirementAnalyticsService::new(&state);

        let metrics = service
            .metrics(1, Some(2), Some(5), Some(10), None)
            .unwrap();
        assert_eq!(metrics.total, 1);
        assert_eq!(metrics.accepted, 1);
    }

    #[test]
    fn metrics_applies_applicability_filter() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(2, status(2, "Accepted"));

        let mut req1 = make_requirement(1, 1, 2, 1, 10);
        req1.applicability_id = 20;
        let mut req2 = make_requirement(2, 1, 2, 1, 10);
        req2.applicability_id = 21;

        repo.requirements.insert(1, req1);
        repo.requirements.insert(2, req2);

        let state = state_with_repo(repo);
        let service = RequirementAnalyticsService::new(&state);

        let metrics = service.metrics(1, None, None, None, Some(20)).unwrap();
        assert_eq!(metrics.total, 1);
    }

    #[test]
    fn metrics_handles_different_projects() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(2, status(2, "Accepted"));

        repo.requirements
            .insert(1, make_requirement(1, 1, 2, 1, 10)); // Project 1
        repo.requirements
            .insert(2, make_requirement(2, 2, 2, 1, 10)); // Project 2

        let state = state_with_repo(repo);
        let service = RequirementAnalyticsService::new(&state);

        let metrics1 = service.metrics(1, None, None, None, None).unwrap();
        assert_eq!(metrics1.total, 1);

        let metrics2 = service.metrics(2, None, None, None, None).unwrap();
        assert_eq!(metrics2.total, 1);
    }
}

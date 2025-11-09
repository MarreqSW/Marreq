//! Service providing aggregated requirement metrics.
//!
//! Encapsulates the logic for computing per-status counts and coverage figures
//! so Rocket routes can stay focused on HTTP concerns.

use crate::app::{AppState, DieselCachedRepo};
use crate::repository::errors::RepoError;
use crate::repository::{LookupRepository, RequirementsRepository};
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer, Nullable, Text};
use serde::Serialize;
use std::collections::HashMap;

/// Aggregated requirement metrics for a project scope.
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
        let query = diesel::sql_query(
            "SELECT LOWER(TRIM(rs.req_st_title)) AS status, COUNT(*)::BIGINT AS total
             FROM requirements r
             INNER JOIN requirement_status rs ON rs.req_st_id = r.req_current_status
             WHERE r.project_id = $1
               AND ($2 IS NULL OR r.req_current_status = $2)
               AND ($3 IS NULL OR r.req_verification = $3)
               AND ($4 IS NULL OR r.req_category = $4)
               AND ($5 IS NULL OR r.req_applicability = $5)
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
        drop(repo_guard);

        let status_lookup: HashMap<i32, String> = statuses
            .into_iter()
            .map(|status| (status.req_st_id, status.req_st_title))
            .collect();

        let mut counts: HashMap<String, i64> = HashMap::new();

        // Apply the same filtering semantics as the SQL path.
        for requirement in requirements {
            if let Some(filter) = status_filter {
                if requirement.req_current_status != filter {
                    continue;
                }
            }
            if let Some(filter) = verification_filter {
                if requirement.req_verification != filter {
                    continue;
                }
            }
            if let Some(filter) = category_filter {
                if requirement.req_category != filter {
                    continue;
                }
            }
            if let Some(filter) = applicability_filter {
                if requirement.req_applicability != filter {
                    continue;
                }
            }

            let title = status_lookup
                .get(&requirement.req_current_status)
                .map(|name| name.trim().to_string())
                .unwrap_or_else(|| format!("Unknown Status ({})", requirement.req_current_status));

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
            let normalized = status.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "draft" => metrics.draft += count,
                "accepted" => metrics.accepted += count,
                "rejected" => metrics.rejected += count,
                _ => {}
            }
        }

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
        req_id: i32,
        project_id: i32,
        status_id: i32,
        verification_id: i32,
        category_id: i32,
    ) -> Requirement {
        Requirement {
            req_id,
            req_title: format!("Req {req_id}"),
            req_description: "desc".into(),
            req_verification: verification_id,
            req_current_status: status_id,
            req_author: 1,
            req_reviewer: 1,
            req_reference: format!("REF-{req_id}"),
            req_category: category_id,
            req_parent: 0,
            req_creation_date: naive_datetime(),
            req_update_date: naive_datetime(),
            req_deadline_date: naive_datetime(),
            req_applicability: 1,
            req_justification: None,
            project_id,
        }
    }

    fn status(id: i32, title: &str) -> RequirementStatus {
        RequirementStatus {
            req_st_id: id,
            req_st_title: title.into(),
            req_st_description: format!("{title} description"),
            req_st_short_name: title.chars().take(3).collect(),
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
}

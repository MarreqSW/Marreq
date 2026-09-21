// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::requirements::requirement_from_baseline_version;
use super::DieselRepo;
use crate::models::entities::*;
use crate::models::forms::*;
use crate::repository::errors::RepoError;
use crate::repository::BaselineRepository;
use crate::schema;
use diesel::expression_methods::NullableExpressionMethods;
use diesel::prelude::*;
use diesel::{Connection, JoinOnDsl, OptionalExtension, SelectableHelper};

impl BaselineRepository for DieselRepo {
    fn create_baseline(
        &mut self,
        project_id: i32,
        created_by: i32,
        payload: &crate::models::NewBaseline,
    ) -> Result<Baseline, RepoError> {
        use schema::baseline_requirements;
        use schema::baseline_traceability;
        use schema::baseline_verifications;
        use schema::baselines;
        use schema::matrix;
        use schema::requirement_versions;
        use schema::requirements;
        use schema::verifications;

        let mut conn = self.get_conn()?;
        conn.as_mut().transaction::<_, RepoError, _>(|conn| {
            let now = chrono::Utc::now().naive_utc();

            let (source_saved_view_id, source_view_definition) =
                if let Some(view_id) = payload.saved_view_id {
                    use schema::saved_views::dsl as sv;
                    let view: crate::models::SavedView = sv::saved_views
                        .filter(sv::id.eq(view_id))
                        .get_result(conn)
                        .map_err(|e| {
                            if e == diesel::result::Error::NotFound {
                                RepoError::NotFound
                            } else {
                                e.into()
                            }
                        })?;
                    if view.project_id != project_id {
                        return Err(RepoError::BadInput(
                            "saved view does not belong to this project".into(),
                        ));
                    }
                    (Some(view.id), Some(view.definition.clone()))
                } else {
                    (None, None)
                };

            let new_row = NewBaselineRow {
                project_id,
                name: payload.name.clone(),
                description: payload.description.clone(),
                created_at: now,
                created_by,
                source_saved_view_id,
                source_view_definition: source_view_definition.clone(),
                mcp_idempotency_identity: payload.mcp_idempotency_identity.clone(),
            };
            let baseline: Baseline = diesel::insert_into(baselines::table)
                .values(&new_row)
                .returning(Baseline::as_returning())
                .get_result(conn)?;
            let baseline_id = baseline.id;

            // Snapshot: all requirements in project with their current version (point-in-time)
            let rows: Vec<(RequirementContainer, RequirementVersion)> =
                requirements::table
                    .inner_join(requirement_versions::table.on(
                        requirements::current_version_id.eq(requirement_versions::id.nullable()),
                    ))
                    .filter(requirements::project_id.eq(project_id))
                    .select((
                        RequirementContainer::as_select(),
                        RequirementVersion::as_select(),
                    ))
                    .load(conn)?;

            let rows: Vec<(RequirementContainer, RequirementVersion)> =
                if let Some(def) = source_view_definition.as_ref() {
                    let applied = crate::saved_view_definition::filters_from_definition(def);
                    rows.into_iter()
                        .filter(|(_container, version)| {
                            if let Some(sid) = applied.status_id {
                                if version.status_id != sid {
                                    return false;
                                }
                            }
                            if let Some(cid) = applied.category_id {
                                if version.category_id != cid {
                                    return false;
                                }
                            }
                            if let Some(state) = applied.approval_state.as_deref() {
                                if !version.approval_state.eq_ignore_ascii_case(state) {
                                    return false;
                                }
                            }
                            if let Some(raw_q) = applied.q.as_deref() {
                                let needle = raw_q.trim().to_lowercase();
                                if !needle.is_empty() {
                                    let blob = format!("{} {}", version.title, version.description)
                                        .to_lowercase();
                                    if !blob.contains(&needle) {
                                        return false;
                                    }
                                }
                            }
                            true
                        })
                        .collect()
                } else {
                    rows
                };

            for (container, version) in rows {
                let br = NewBaselineRequirement {
                    baseline_id,
                    requirement_id: container.id,
                    version_id: version.id,
                };
                diesel::insert_into(baseline_requirements::table)
                    .values(&br)
                    .execute(conn)?;
            }

            // Snapshot: current traceability matrix (including suspect state at baseline time)
            let matrix_links: Vec<MatrixLink> = matrix::table
                .filter(matrix::project_id.eq(project_id))
                .load(conn)?;
            for link in matrix_links {
                let bt = NewBaselineTraceability {
                    baseline_id,
                    requirement_id: link.req_id,
                    verification_id: link.verification_id,
                    suspect: link.suspect,
                    suspect_at: link.suspect_at,
                    suspect_reason: link.suspect_reason.clone(),
                };
                diesel::insert_into(baseline_traceability::table)
                    .values(&bt)
                    .execute(conn)?;
            }

            // Snapshot: all verifications in project (point-in-time)
            let project_verifications: Vec<Verification> = verifications::table
                .filter(verifications::project_id.eq(project_id))
                .select(Verification::as_select())
                .load(conn)?;
            for v in project_verifications {
                let bv = NewBaselineVerification {
                    baseline_id,
                    verification_id: v.id,
                    name: v.name,
                    reference_code: v.reference_code,
                    description: v.description,
                    source: v.source,
                    status_id: v.status_id,
                    parent_id: v.parent_id,
                    project_id: v.project_id,
                    verification_method_id: v.verification_method_id,
                    author_id: v.author_id,
                    reviewer_id: v.reviewer_id,
                };
                diesel::insert_into(baseline_verifications::table)
                    .values(&bv)
                    .execute(conn)?;
            }

            if let Some(view_id) = source_saved_view_id {
                use schema::saved_views::dsl as sv;
                diesel::update(sv::saved_views.filter(sv::id.eq(view_id)))
                    .set((
                        sv::locked.eq(true),
                        sv::locked_at.eq(Some(now)),
                        sv::updated_at.eq(now),
                    ))
                    .execute(conn)?;
            }

            Ok(baseline)
        })
    }

    fn list_baselines_by_project(&self, project_id: i32) -> Result<Vec<Baseline>, RepoError> {
        use schema::baselines::dsl;
        let mut conn = self.get_conn()?;
        dsl::baselines
            .filter(dsl::project_id.eq(project_id))
            .order(dsl::created_at.desc())
            .select(Baseline::as_select())
            .load(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn get_baseline_by_id(&self, baseline_id: i32) -> Result<Baseline, RepoError> {
        use schema::baselines::dsl;
        let mut conn = self.get_conn()?;
        dsl::baselines
            .filter(dsl::id.eq(baseline_id))
            .select(Baseline::as_select())
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_requirements_for_baseline(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<Requirement>, RepoError> {
        use schema::baseline_requirements;
        use schema::requirement_versions;
        use schema::requirements;

        let mut conn = self.get_conn()?;
        let rows: Vec<(RequirementContainer, RequirementVersion)> = baseline_requirements::table
            .inner_join(
                requirement_versions::table
                    .on(baseline_requirements::version_id.eq(requirement_versions::id)),
            )
            .inner_join(
                requirements::table.on(baseline_requirements::requirement_id.eq(requirements::id)),
            )
            .filter(baseline_requirements::baseline_id.eq(baseline_id))
            .select((
                RequirementContainer::as_select(),
                RequirementVersion::as_select(),
            ))
            .load(conn.as_mut())?;
        Ok(rows
            .into_iter()
            .map(|(c, v)| requirement_from_baseline_version(&c, &v))
            .collect())
    }

    fn get_baseline_requirement_version_id(
        &self,
        baseline_id: i32,
        requirement_id: i32,
    ) -> Result<Option<i32>, RepoError> {
        use schema::baseline_requirements::dsl;
        let mut conn = self.get_conn()?;
        dsl::baseline_requirements
            .filter(dsl::baseline_id.eq(baseline_id))
            .filter(dsl::requirement_id.eq(requirement_id))
            .select(dsl::version_id)
            .get_result::<i32>(conn.as_mut())
            .optional()
            .map_err(RepoError::from)
    }

    fn get_baseline_traceability(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<BaselineTraceability>, RepoError> {
        use schema::baseline_traceability::dsl;
        let mut conn = self.get_conn()?;
        dsl::baseline_traceability
            .filter(dsl::baseline_id.eq(baseline_id))
            .order((dsl::requirement_id.asc(), dsl::verification_id.asc()))
            .load(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn get_verifications_for_baseline(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<BaselineVerification>, RepoError> {
        use schema::baseline_verifications::dsl;
        let mut conn = self.get_conn()?;
        dsl::baseline_verifications
            .filter(dsl::baseline_id.eq(baseline_id))
            .order(dsl::verification_id.asc())
            .load(conn.as_mut())
            .map_err(RepoError::from)
    }
}

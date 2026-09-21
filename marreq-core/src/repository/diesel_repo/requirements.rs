// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::{map_db_error, DieselRepo};
use crate::models::entities::*;
use crate::models::forms::*;
use crate::repository::errors::RepoError;
use crate::repository::{MatrixRepository, RequirementsRepository};
use crate::schema;
use diesel::expression_methods::NullableExpressionMethods;
use diesel::prelude::*;
use diesel::{Connection, JoinOnDsl, OptionalExtension, SelectableHelper};

pub(crate) fn requirement_from_baseline_version(
    container: &RequirementContainer,
    version: &RequirementVersion,
) -> Requirement {
    let same_as_current = container.current_version_id == Some(version.id);
    Requirement {
        id: container.id,
        current_version_id: Some(version.id),
        same_as_current: Some(same_as_current),
        title: version.title.clone(),
        description: version.description.clone(),
        status_id: version.status_id,
        author_id: version.author_id,
        reviewer_id: version.reviewer_id,
        reference_code: container.stable_code.clone(),
        category_id: version.category_id,
        parent_id: None, // populated from requirement_version_links by service/decorator layer
        creation_date: container.first_created_at,
        update_date: version.created_at,
        deadline_date: version.deadline_date,
        applicability_id: version.applicability_id,
        justification: version.justification.clone(),
        project_id: container.project_id,
        approval_state: version.approval_state.clone(),
        approved_by: version.approved_by,
        approved_at: version.approved_at,
        custom_fields: None,
    }
}

pub(crate) fn requirement_from_current(
    container: &RequirementContainer,
    version: &RequirementVersion,
) -> Requirement {
    Requirement {
        id: container.id,
        current_version_id: container.current_version_id,
        same_as_current: None,
        title: version.title.clone(),
        description: version.description.clone(),
        status_id: version.status_id,
        author_id: version.author_id,
        reviewer_id: version.reviewer_id,
        reference_code: container.stable_code.clone(),
        category_id: version.category_id,
        parent_id: None, // populated from requirement_version_links by service/decorator layer
        creation_date: container.first_created_at,
        update_date: version.created_at,
        deadline_date: version.deadline_date,
        applicability_id: version.applicability_id,
        justification: version.justification.clone(),
        project_id: container.project_id,
        approval_state: version.approval_state.clone(),
        approved_by: version.approved_by,
        approved_at: version.approved_at,
        custom_fields: None,
    }
}

impl RequirementsRepository for DieselRepo {
    fn get_requirement_by_id(&self, requirement_id: i32) -> Result<Requirement, RepoError> {
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        let (container, version): (RequirementContainer, RequirementVersion) = requirements::table
            .inner_join(
                requirement_versions::table
                    .on(requirements::current_version_id.eq(requirement_versions::id.nullable())),
            )
            .filter(requirements::id.eq(requirement_id))
            .select((
                RequirementContainer::as_select(),
                RequirementVersion::as_select(),
            ))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        Ok(requirement_from_current(&container, &version))
    }

    fn get_requirements_all(&self) -> Result<Vec<Requirement>, RepoError> {
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        let rows: Vec<(RequirementContainer, RequirementVersion)> = requirements::table
            .inner_join(
                requirement_versions::table
                    .on(requirements::current_version_id.eq(requirement_versions::id.nullable())),
            )
            .order(requirements::id)
            .select((
                RequirementContainer::as_select(),
                RequirementVersion::as_select(),
            ))
            .load(conn.as_mut())
            .map_err(RepoError::from)?;
        Ok(rows
            .into_iter()
            .map(|(c, v)| requirement_from_current(&c, &v))
            .collect())
    }

    fn get_requirements_by_project(
        &self,
        project_id_param: i32,
    ) -> Result<Vec<Requirement>, RepoError> {
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        let rows: Vec<(RequirementContainer, RequirementVersion)> = requirements::table
            .inner_join(
                requirement_versions::table
                    .on(requirements::current_version_id.eq(requirement_versions::id.nullable())),
            )
            .filter(requirements::project_id.eq(project_id_param))
            .order(requirements::id)
            .select((
                RequirementContainer::as_select(),
                RequirementVersion::as_select(),
            ))
            .load(conn.as_mut())
            .map_err(RepoError::from)?;
        Ok(rows
            .into_iter()
            .map(|(c, v)| requirement_from_current(&c, &v))
            .collect())
    }

    fn get_requirements_by_project_filtered_paginated(
        &self,
        project_id: i32,
        status_filter: Option<i32>,
        verification_filter: Option<i32>,
        category_filter: Option<i32>,
        applicability_filter: Option<i32>,
        custom_field_filters: Option<&[(i32, String)]>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Requirement>, RepoError> {
        use schema::custom_field_values::dsl as cfv_dsl;
        use schema::requirement_version_verification_methods::dsl as rvvm_dsl;
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        let mut query = requirements::table
            .inner_join(
                requirement_versions::table
                    .on(requirements::current_version_id.eq(requirement_versions::id.nullable())),
            )
            .filter(requirements::project_id.eq(project_id))
            .into_boxed();
        if let Some(s) = status_filter {
            query = query.filter(requirement_versions::status_id.eq(s));
        }
        if let Some(c) = category_filter {
            query = query.filter(requirement_versions::category_id.eq(c));
        }
        if let Some(a) = applicability_filter {
            query = query.filter(requirement_versions::applicability_id.eq(a));
        }
        if let Some(v) = verification_filter {
            query = query.filter(
                requirement_versions::id.eq_any(
                    rvvm_dsl::requirement_version_verification_methods
                        .filter(rvvm_dsl::verification_method_id.eq(v))
                        .select(rvvm_dsl::requirement_version_id),
                ),
            );
        }
        if let Some(filters) = custom_field_filters {
            for (field_id, value) in filters.iter() {
                let version_ids = cfv_dsl::custom_field_values
                    .filter(cfv_dsl::custom_field_definition_id.eq(field_id))
                    .filter(cfv_dsl::value.eq(value));
                query = query.filter(
                    requirement_versions::id
                        .eq_any(version_ids.select(cfv_dsl::requirement_version_id)),
                );
            }
        }
        let rows: Vec<(RequirementContainer, RequirementVersion)> = query
            .order(requirements::id)
            .limit(limit)
            .offset(offset)
            .select((
                RequirementContainer::as_select(),
                RequirementVersion::as_select(),
            ))
            .load(conn.as_mut())
            .map_err(RepoError::from)?;
        // Sort empty stable_code last (same as legacy)
        let mut result: Vec<Requirement> = rows
            .into_iter()
            .map(|(c, v)| requirement_from_current(&c, &v))
            .collect();
        result.sort_by(|a, b| {
            match (
                a.reference_code.trim().is_empty(),
                b.reference_code.trim().is_empty(),
            ) {
                (false, false) => a.reference_code.cmp(&b.reference_code),
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                (true, true) => a.id.cmp(&b.id),
            }
        });
        Ok(result)
    }

    fn insert_new_requirement(&mut self, new: &NewRequirement) -> Result<i32, RepoError> {
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        conn.as_mut().transaction::<i32, RepoError, _>(|conn| {
            let container = NewRequirementContainer {
                project_id: new.project_id,
                stable_code: new.reference_code.clone(),
                current_version_id: None,
                mcp_idempotency_identity: None,
            };
            let req_id: i32 = diesel::insert_into(requirements::table)
                .values(&container)
                .returning(requirements::id)
                .get_result(conn)?;
            let version = new.to_new_version(req_id);
            let (version_id, version_created_at): (i32, chrono::NaiveDateTime) =
                diesel::insert_into(requirement_versions::table)
                    .values(&version)
                    .returning((requirement_versions::id, requirement_versions::created_at))
                    .get_result(conn)?;
            diesel::update(requirements::table.filter(requirements::id.eq(req_id)))
                .set((
                    requirements::current_version_id.eq(version_id),
                    requirements::first_created_at.eq(version_created_at),
                ))
                .execute(conn)?;
            Ok(req_id)
        })
    }

    fn create_requirement_atomic(
        &mut self,
        new: &NewRequirement,
        verification_method_ids: &[i32],
        custom_fields: Option<&[CustomFieldValueInput]>,
        parent_links: &[NewRequirementVersionLink],
        mcp_idempotency_identity: Option<&str>,
    ) -> Result<i32, RepoError> {
        use schema::custom_field_values;
        use schema::requirement_version_links;
        use schema::requirement_version_verification_methods;
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        conn.as_mut().transaction::<i32, RepoError, _>(|conn| {
            let container = NewRequirementContainer {
                project_id: new.project_id,
                stable_code: new.reference_code.clone(),
                current_version_id: None,
                mcp_idempotency_identity: mcp_idempotency_identity.map(str::to_owned),
            };
            let req_id: i32 = diesel::insert_into(requirements::table)
                .values(&container)
                .returning(requirements::id)
                .get_result(conn)?;
            let version = new.to_new_version(req_id);
            let (version_id, version_created_at): (i32, chrono::NaiveDateTime) =
                diesel::insert_into(requirement_versions::table)
                    .values(&version)
                    .returning((requirement_versions::id, requirement_versions::created_at))
                    .get_result(conn)?;
            diesel::update(requirements::table.filter(requirements::id.eq(req_id)))
                .set((
                    requirements::current_version_id.eq(version_id),
                    requirements::first_created_at.eq(version_created_at),
                ))
                .execute(conn)?;
            for &verification_method_id in verification_method_ids {
                if verification_method_id <= 0 {
                    continue;
                }
                diesel::insert_into(requirement_version_verification_methods::table)
                    .values((
                        requirement_version_verification_methods::requirement_version_id
                            .eq(version_id),
                        requirement_version_verification_methods::verification_method_id
                            .eq(verification_method_id),
                    ))
                    .execute(conn)
                    .map_err(map_db_error)?;
            }
            if let Some(values) = custom_fields {
                for field in values {
                    if field.field_id <= 0 {
                        continue;
                    }
                    diesel::insert_into(custom_field_values::table)
                        .values((
                            custom_field_values::requirement_version_id.eq(version_id),
                            custom_field_values::custom_field_definition_id.eq(field.field_id),
                            custom_field_values::value.eq(field.value.as_deref()),
                        ))
                        .execute(conn)
                        .map_err(map_db_error)?;
                }
            }
            for link in parent_links {
                let mut new_link = link.clone();
                new_link.source_version_id = version_id;
                diesel::insert_into(requirement_version_links::table)
                    .values(&new_link)
                    .execute(conn)
                    .map_err(map_db_error)?;
            }
            Ok(req_id)
        })
    }

    fn get_verification_method_ids_for_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<i32>, RepoError> {
        use schema::requirement_version_verification_methods::dsl as rvvm_dsl;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        let current_version_id: Option<i32> = requirements::table
            .filter(requirements::id.eq(requirement_id))
            .select(requirements::current_version_id)
            .get_result::<Option<i32>>(conn.as_mut())
            .optional()
            .map_err(RepoError::from)?
            .flatten();
        let Some(vid) = current_version_id else {
            return Ok(vec![]);
        };
        rvvm_dsl::requirement_version_verification_methods
            .filter(rvvm_dsl::requirement_version_id.eq(vid))
            .select(rvvm_dsl::verification_method_id)
            .order(rvvm_dsl::verification_method_id)
            .load::<i32>(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn get_verification_method_ids_for_version(
        &self,
        version_id: i32,
    ) -> Result<Vec<i32>, RepoError> {
        use schema::requirement_version_verification_methods::dsl as rvvm_dsl;
        let mut conn = self.get_conn()?;
        rvvm_dsl::requirement_version_verification_methods
            .filter(rvvm_dsl::requirement_version_id.eq(version_id))
            .select(rvvm_dsl::verification_method_id)
            .order(rvvm_dsl::verification_method_id)
            .load::<i32>(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn get_requirement_ids_by_verification_method(
        &self,
        verification_method_id: i32,
    ) -> Result<Vec<i32>, RepoError> {
        use schema::requirement_version_verification_methods::dsl as rvvm_dsl;
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        requirements::table
            .inner_join(
                requirement_versions::table
                    .on(requirements::current_version_id.eq(requirement_versions::id.nullable())),
            )
            .inner_join(
                rvvm_dsl::requirement_version_verification_methods
                    .on(rvvm_dsl::requirement_version_id.eq(requirement_versions::id)),
            )
            .filter(rvvm_dsl::verification_method_id.eq(verification_method_id))
            .select(requirements::id)
            .load::<i32>(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn set_requirement_verification_methods(
        &mut self,
        requirement_id: i32,
        verification_method_ids: &[i32],
    ) -> Result<(), RepoError> {
        use schema::requirement_version_verification_methods;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        let current_version_id: Option<i32> = requirements::table
            .filter(requirements::id.eq(requirement_id))
            .select(requirements::current_version_id)
            .get_result::<Option<i32>>(conn.as_mut())
            .optional()
            .map_err(RepoError::from)?
            .flatten();
        let Some(vid) = current_version_id else {
            return Ok(());
        };
        diesel::delete(requirement_version_verification_methods::table)
            .filter(requirement_version_verification_methods::requirement_version_id.eq(vid))
            .execute(conn.as_mut())?;
        for &verification_method_id in verification_method_ids {
            if verification_method_id <= 0 {
                continue;
            }
            diesel::insert_into(requirement_version_verification_methods::table)
                .values((
                    requirement_version_verification_methods::requirement_version_id.eq(vid),
                    requirement_version_verification_methods::verification_method_id
                        .eq(verification_method_id),
                ))
                .execute(conn.as_mut())
                .map_err(map_db_error)?;
        }
        Ok(())
    }

    fn edit_requirement(&mut self, new: &NewRequirement) -> Result<bool, RepoError> {
        use schema::requirement_version_links::dsl as rvl;
        use schema::requirement_versions;
        use schema::requirements;
        let id_val = new
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let mut conn = self.get_conn()?;
        conn.as_mut().transaction::<bool, RepoError, _>(|conn| {
            let old_version_id: i32 = requirements::table
                .filter(requirements::id.eq(id_val))
                .select(requirements::current_version_id)
                .get_result::<Option<i32>>(conn)
                .optional()?
                .flatten()
                .ok_or(RepoError::NotFound)?;
            let version = new.to_new_version(id_val);
            let new_version_id: i32 = diesel::insert_into(requirement_versions::table)
                .values(&version)
                .returning(requirement_versions::id)
                .get_result(conn)?;
            let affected = diesel::update(requirements::table.filter(requirements::id.eq(id_val)))
                .set(requirements::current_version_id.eq(new_version_id))
                .execute(conn)?;
            // Keep hierarchy: version links attach to a specific requirement_version row. Without
            // repointing, parent/child edges would still reference the previous current_version_id
            // while `requirements.current_version_id` moved forward — list/detail enrichment would
            // see no parents and `list_links_by_target_version(current)` would miss children.
            diesel::update(
                rvl::requirement_version_links.filter(rvl::source_version_id.eq(old_version_id)),
            )
            .set(rvl::source_version_id.eq(new_version_id))
            .execute(conn)?;
            diesel::update(
                rvl::requirement_version_links.filter(rvl::target_version_id.eq(old_version_id)),
            )
            .set(rvl::target_version_id.eq(new_version_id))
            .execute(conn)?;
            Ok(affected > 0)
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn update_requirement_atomic(
        &mut self,
        requirement_id: i32,
        new: &NewRequirement,
        verification_method_ids: &[i32],
        custom_fields: Option<&[CustomFieldValueInput]>,
        parent_links: Option<&[NewRequirementVersionLink]>,
        suspect_reason: &str,
        actor_id: i32,
    ) -> Result<Requirement, RepoError> {
        use schema::custom_field_values;
        use schema::matrix;
        use schema::requirement_version_links;
        use schema::requirement_version_verification_methods;
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        conn.as_mut()
            .transaction::<Requirement, RepoError, _>(|conn| {
                let (container, old_version): (RequirementContainer, RequirementVersion) =
                    requirements::table
                        .inner_join(
                            requirement_versions::table.on(requirements::current_version_id
                                .eq(requirement_versions::id.nullable())),
                        )
                        .filter(requirements::id.eq(requirement_id))
                        .select((
                            RequirementContainer::as_select(),
                            RequirementVersion::as_select(),
                        ))
                        .get_result(conn)
                        .map_err(|e| {
                            if e == diesel::result::Error::NotFound {
                                RepoError::NotFound
                            } else {
                                e.into()
                            }
                        })?;
                let version = new.to_new_version(requirement_id);
                let new_version_id: i32 = diesel::insert_into(requirement_versions::table)
                    .values(&version)
                    .returning(requirement_versions::id)
                    .get_result(conn)?;
                let affected =
                    diesel::update(requirements::table.filter(requirements::id.eq(requirement_id)))
                        .set(requirements::current_version_id.eq(new_version_id))
                        .execute(conn)?;
                if affected == 0 {
                    return Err(RepoError::NotFound);
                }
                diesel::update(
                    requirement_version_links::table
                        .filter(requirement_version_links::source_version_id.eq(old_version.id)),
                )
                .set(requirement_version_links::source_version_id.eq(new_version_id))
                .execute(conn)?;
                diesel::update(
                    requirement_version_links::table
                        .filter(requirement_version_links::target_version_id.eq(old_version.id)),
                )
                .set(requirement_version_links::target_version_id.eq(new_version_id))
                .execute(conn)?;
                diesel::delete(
                    requirement_version_verification_methods::table.filter(
                        requirement_version_verification_methods::requirement_version_id
                            .eq(new_version_id),
                    ),
                )
                .execute(conn)?;
                for &verification_method_id in verification_method_ids {
                    if verification_method_id <= 0 {
                        continue;
                    }
                    diesel::insert_into(requirement_version_verification_methods::table)
                        .values((
                            requirement_version_verification_methods::requirement_version_id
                                .eq(new_version_id),
                            requirement_version_verification_methods::verification_method_id
                                .eq(verification_method_id),
                        ))
                        .execute(conn)
                        .map_err(map_db_error)?;
                }
                if let Some(values) = custom_fields {
                    diesel::delete(
                        custom_field_values::table
                            .filter(custom_field_values::requirement_version_id.eq(new_version_id)),
                    )
                    .execute(conn)?;
                    for field in values {
                        if field.field_id <= 0 {
                            continue;
                        }
                        diesel::insert_into(custom_field_values::table)
                            .values((
                                custom_field_values::requirement_version_id.eq(new_version_id),
                                custom_field_values::custom_field_definition_id.eq(field.field_id),
                                custom_field_values::value.eq(field.value.as_deref()),
                            ))
                            .execute(conn)
                            .map_err(map_db_error)?;
                    }
                }
                let now = chrono::Utc::now().naive_utc();
                diesel::update(matrix::table.filter(matrix::req_id.eq(requirement_id)))
                    .set((
                        matrix::suspect.eq(true),
                        matrix::suspect_at.eq(now),
                        matrix::suspect_reason.eq(suspect_reason),
                        matrix::cleared_by.eq(Option::<i32>::None),
                        matrix::cleared_at.eq(Option::<chrono::NaiveDateTime>::None),
                        matrix::triggering_version_id.eq(Some(new_version_id)),
                        matrix::triggering_user_id.eq(Some(actor_id)),
                    ))
                    .execute(conn)?;
                if let Some(links) = parent_links {
                    diesel::delete(
                        requirement_version_links::table.filter(
                            requirement_version_links::source_version_id.eq(new_version_id),
                        ),
                    )
                    .execute(conn)?;
                    for link in links {
                        let mut new_link = link.clone();
                        new_link.source_version_id = new_version_id;
                        diesel::insert_into(requirement_version_links::table)
                            .values(&new_link)
                            .execute(conn)
                            .map_err(map_db_error)?;
                    }
                }
                let new_version = requirement_versions::table
                    .filter(requirement_versions::id.eq(new_version_id))
                    .select(RequirementVersion::as_select())
                    .get_result(conn)?;
                Ok(requirement_from_current(
                    &RequirementContainer {
                        current_version_id: Some(new_version_id),
                        stable_code: new.reference_code.clone(),
                        ..container
                    },
                    &new_version,
                ))
            })
    }

    fn delete_requirement(&mut self, requirement_id: i32) -> Result<Requirement, RepoError> {
        let req = self.get_requirement_by_id(requirement_id)?;
        let mut conn = self.get_conn()?;
        diesel::delete(
            schema::requirements::table.filter(schema::requirements::id.eq(requirement_id)),
        )
        .execute(conn.as_mut())?;
        Ok(req)
    }

    fn update_requirement(&mut self, _requirement_id: i32) -> Result<(), RepoError> {
        // Versions are immutable; no update_date to touch. No-op for compatibility.
        Ok(())
    }

    fn list_requirement_versions(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<RequirementVersion>, RepoError> {
        use schema::requirement_versions::dsl;
        let mut conn = self.get_conn()?;
        dsl::requirement_versions
            .filter(dsl::requirement_id.eq(requirement_id))
            .order(dsl::created_at.desc())
            .select(RequirementVersion::as_select())
            .load(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn get_requirement_version_by_id(
        &self,
        version_id: i32,
    ) -> Result<RequirementVersion, RepoError> {
        use schema::requirement_versions::dsl;
        let mut conn = self.get_conn()?;
        dsl::requirement_versions
            .filter(dsl::id.eq(version_id))
            .select(RequirementVersion::as_select())
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn set_requirement_version_approval(
        &mut self,
        version_id: i32,
        new_state: &str,
        approved_by_user_id: i32,
    ) -> Result<RequirementVersion, RepoError> {
        use crate::status_enums::ApprovalState;
        use schema::requirement_versions::dsl;
        let mut conn = self.get_conn()?;
        let version: RequirementVersion = dsl::requirement_versions
            .filter(dsl::id.eq(version_id))
            .select(RequirementVersion::as_select())
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        let current = ApprovalState::from_db_string(&version.approval_state).ok_or_else(|| {
            RepoError::BadInput(format!(
                "invalid approval_state in DB: {}",
                version.approval_state
            ))
        })?;
        let target = ApprovalState::from_db_string(new_state)
            .ok_or_else(|| RepoError::BadInput(format!("invalid approval_state: {}", new_state)))?;
        if !current.can_transition_to(target) {
            return Err(RepoError::BadInput(format!(
                "invalid transition: {} -> {}",
                version.approval_state, new_state
            )));
        }
        // Idempotent: already in target state — return version unchanged
        if current == target {
            return Ok(version);
        }
        let now = chrono::Utc::now().naive_utc();
        let (approved_by, approved_at) = if target == ApprovalState::Approved {
            (Some(approved_by_user_id), Some(now))
        } else {
            (version.approved_by, version.approved_at)
        };
        let (reviewed_by, reviewed_at) = if target == ApprovalState::Reviewed {
            (Some(approved_by_user_id), Some(now))
        } else {
            (version.reviewed_by, version.reviewed_at)
        };
        diesel::update(dsl::requirement_versions.filter(dsl::id.eq(version_id)))
            .set((
                dsl::approval_state.eq(target.to_db_string()),
                dsl::approved_by.eq(approved_by),
                dsl::approved_at.eq(approved_at),
                dsl::reviewed_by.eq(reviewed_by),
                dsl::reviewed_at.eq(reviewed_at),
            ))
            .execute(conn.as_mut())?;
        drop(conn);
        let _ = self.mark_links_suspect_for_requirement(
            version.requirement_id,
            "Approval state changed",
            Some(version_id),
            Some(approved_by_user_id),
        )?;
        let mut conn = self.get_conn()?;
        dsl::requirement_versions
            .filter(dsl::id.eq(version_id))
            .select(RequirementVersion::as_select())
            .get_result(conn.as_mut())
            .map_err(RepoError::from)
    }
}

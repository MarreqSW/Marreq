// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Use cases, their versions and drafts (issue #398).

use super::{DieselRepo, map_db_error};
use crate::models::{
    BaselineUseCase, BaselineUseCaseLink, NewUseCase, NewUseCaseComment, NewUseCaseLink,
    NewUseCaseVersion, UseCase, UseCaseComment, UseCaseDraft, UseCaseLink, UseCaseVersion,
};
use crate::repository::UseCasesRepository;
use crate::repository::errors::RepoError;
use crate::schema;
use crate::status_enums::ApprovalState;
use diesel::prelude::*;

/// Record the current version of every published use case of the project,
/// and the use case links, in a new baseline (in its transaction).
pub(crate) fn snapshot_use_cases(
    conn: &mut PgConnection,
    baseline_id: i32,
    project_id: i32,
) -> QueryResult<()> {
    use diesel::sql_types::Integer;
    diesel::sql_query(
        "INSERT INTO baseline_use_cases (baseline_id, use_case_id, version_id)
         SELECT $1, id, current_version_id FROM use_cases
         WHERE project_id = $2 AND current_version_id IS NOT NULL",
    )
    .bind::<Integer, _>(baseline_id)
    .bind::<Integer, _>(project_id)
    .execute(conn)?;
    diesel::sql_query(
        "INSERT INTO baseline_use_case_links
           (baseline_id, use_case_id, element_id, requirement_id, verification_id, suspect)
         SELECT $1, l.use_case_id, l.element_id, l.requirement_id, l.verification_id, l.suspect
         FROM use_case_links l
         JOIN baseline_use_cases b ON b.use_case_id = l.use_case_id AND b.baseline_id = $1
         ORDER BY l.id",
    )
    .bind::<Integer, _>(baseline_id)
    .execute(conn)?;
    Ok(())
}

/// Mark the use case links to a requirement suspect, in the caller's
/// connection or transaction (the matrix rule, issue #400).
pub(crate) fn mark_use_case_links_suspect(
    conn: &mut PgConnection,
    requirement_id: i32,
    reason: &str,
    triggering_version_id: Option<i32>,
    triggering_user_id: Option<i32>,
) -> QueryResult<usize> {
    use schema::use_case_links::dsl;
    diesel::update(dsl::use_case_links.filter(dsl::requirement_id.eq(requirement_id)))
        .set((
            dsl::suspect.eq(true),
            dsl::suspect_at.eq(chrono::Utc::now().naive_utc()),
            dsl::suspect_reason.eq(reason),
            dsl::cleared_by.eq(Option::<i32>::None),
            dsl::cleared_at.eq(Option::<chrono::NaiveDateTime>::None),
            dsl::triggering_version_id.eq(triggering_version_id),
            dsl::triggering_user_id.eq(triggering_user_id),
        ))
        .execute(conn)
}

fn not_found(e: diesel::result::Error) -> RepoError {
    if e == diesel::result::Error::NotFound {
        RepoError::NotFound
    } else {
        e.into()
    }
}

impl UseCasesRepository for DieselRepo {
    fn list_use_cases_by_project(&self, project_id: i32) -> Result<Vec<UseCase>, RepoError> {
        use schema::use_cases::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::use_cases
            .filter(dsl::project_id.eq(project_id))
            .order(dsl::reference_code)
            .load(conn.as_mut())?)
    }

    fn get_use_case(&self, id: i32) -> Result<UseCase, RepoError> {
        use schema::use_cases::dsl;
        let mut conn = self.get_conn()?;
        dsl::use_cases
            .find(id)
            .first(conn.as_mut())
            .map_err(not_found)
    }

    fn list_use_case_codes(&self, project_id: i32) -> Result<Vec<String>, RepoError> {
        use schema::use_cases::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::use_cases
            .filter(dsl::project_id.eq(project_id))
            .select(dsl::reference_code)
            .load(conn.as_mut())?)
    }

    fn create_use_case(
        &mut self,
        new: &NewUseCase,
        draft_document: &serde_json::Value,
    ) -> Result<(UseCase, UseCaseDraft), RepoError> {
        let mut conn = self.get_conn()?;
        conn.as_mut().transaction::<_, RepoError, _>(|conn| {
            let use_case: UseCase = diesel::insert_into(schema::use_cases::table)
                .values(new)
                .get_result(conn)
                .map_err(map_db_error)?;
            use schema::use_case_drafts::dsl as d;
            let draft: UseCaseDraft = diesel::insert_into(d::use_case_drafts)
                .values((
                    d::use_case_id.eq(use_case.id),
                    d::document.eq(draft_document),
                    d::revision.eq(1),
                    d::updated_by.eq(new.created_by),
                ))
                .get_result(conn)?;
            Ok((use_case, draft))
        })
    }

    fn delete_use_case(&mut self, id: i32) -> Result<bool, RepoError> {
        use schema::use_cases::dsl;
        let mut conn = self.get_conn()?;
        let n = diesel::delete(dsl::use_cases.find(id)).execute(conn.as_mut())?;
        Ok(n > 0)
    }

    fn list_use_case_versions(&self, use_case_id: i32) -> Result<Vec<UseCaseVersion>, RepoError> {
        use schema::use_case_versions::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::use_case_versions
            .filter(dsl::use_case_id.eq(use_case_id))
            .order(dsl::version_number.desc())
            .load(conn.as_mut())?)
    }

    fn get_use_case_version(&self, version_id: i32) -> Result<UseCaseVersion, RepoError> {
        use schema::use_case_versions::dsl;
        let mut conn = self.get_conn()?;
        dsl::use_case_versions
            .find(version_id)
            .first(conn.as_mut())
            .map_err(not_found)
    }

    fn list_current_use_case_versions(
        &self,
        project_id: i32,
    ) -> Result<Vec<UseCaseVersion>, RepoError> {
        use schema::use_case_versions::dsl as v;
        use schema::use_cases::dsl as u;
        let mut conn = self.get_conn()?;
        let current = u::use_cases
            .filter(u::project_id.eq(project_id))
            .select(u::current_version_id)
            .filter(u::current_version_id.is_not_null());
        Ok(v::use_case_versions
            .filter(v::id.nullable().eq_any(current))
            .load(conn.as_mut())?)
    }

    fn set_use_case_version_approval(
        &mut self,
        version_id: i32,
        new_state: &str,
        user_id: i32,
    ) -> Result<UseCaseVersion, RepoError> {
        use schema::use_case_versions::dsl;
        let mut conn = self.get_conn()?;
        conn.as_mut().transaction::<_, RepoError, _>(|conn| {
            let version: UseCaseVersion = dsl::use_case_versions
                .find(version_id)
                .for_update()
                .first(conn)
                .map_err(not_found)?;
            let current = ApprovalState::from_db_string(&version.approval_state)
                .ok_or_else(|| RepoError::BadInput("invalid approval_state in DB".into()))?;
            let target = ApprovalState::from_db_string(new_state).ok_or_else(|| {
                RepoError::BadInput(format!("invalid approval_state: {new_state}"))
            })?;
            if !current.can_transition_to(target) {
                return Err(RepoError::BadInput(format!(
                    "invalid transition: {} -> {}",
                    version.approval_state, new_state
                )));
            }
            if current == target {
                return Ok(version);
            }
            let now = chrono::Utc::now().naive_utc();
            let updated = match target {
                ApprovalState::Reviewed => diesel::update(dsl::use_case_versions.find(version_id))
                    .set((
                        dsl::approval_state.eq(target.to_db_string()),
                        dsl::reviewed_by.eq(user_id),
                        dsl::reviewed_at.eq(now),
                    ))
                    .get_result(conn)?,
                ApprovalState::Approved => diesel::update(dsl::use_case_versions.find(version_id))
                    .set((
                        dsl::approval_state.eq(target.to_db_string()),
                        dsl::approved_by.eq(user_id),
                        dsl::approved_at.eq(now),
                    ))
                    .get_result(conn)?,
                ApprovalState::Draft => version,
            };
            use schema::use_cases::dsl as u;
            diesel::update(u::use_cases.find(updated.use_case_id))
                .set(u::updated_at.eq(now))
                .execute(conn)?;
            Ok(updated)
        })
    }

    fn get_use_case_draft(&self, use_case_id: i32) -> Result<Option<UseCaseDraft>, RepoError> {
        use schema::use_case_drafts::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::use_case_drafts
            .find(use_case_id)
            .first(conn.as_mut())
            .optional()?)
    }

    fn list_use_case_drafts_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<UseCaseDraft>, RepoError> {
        use schema::use_case_drafts::dsl as d;
        use schema::use_cases::dsl as u;
        let mut conn = self.get_conn()?;
        let ids = u::use_cases
            .filter(u::project_id.eq(project_id))
            .select(u::id);
        Ok(d::use_case_drafts
            .filter(d::use_case_id.eq_any(ids))
            .load(conn.as_mut())?)
    }

    fn save_use_case_draft(
        &mut self,
        use_case_id: i32,
        base_revision: Option<i32>,
        base_version_id: Option<i32>,
        document: &serde_json::Value,
        user_id: i32,
    ) -> Result<Option<UseCaseDraft>, RepoError> {
        use schema::use_case_drafts::dsl as d;
        use schema::use_cases::dsl as u;
        let mut conn = self.get_conn()?;
        conn.as_mut().transaction::<_, RepoError, _>(|conn| {
            let now = chrono::Utc::now().naive_utc();
            let saved: Option<UseCaseDraft> = match base_revision {
                None => diesel::insert_into(d::use_case_drafts)
                    .values((
                        d::use_case_id.eq(use_case_id),
                        d::base_version_id.eq(base_version_id),
                        d::document.eq(document),
                        d::revision.eq(1),
                        d::updated_by.eq(user_id),
                        d::updated_at.eq(now),
                    ))
                    .on_conflict_do_nothing()
                    .get_result(conn)
                    .optional()?,
                Some(revision) => diesel::update(
                    d::use_case_drafts
                        .filter(d::use_case_id.eq(use_case_id))
                        .filter(d::revision.eq(revision)),
                )
                .set((
                    d::document.eq(document),
                    d::revision.eq(revision + 1),
                    d::updated_by.eq(user_id),
                    d::updated_at.eq(now),
                ))
                .get_result(conn)
                .optional()?,
            };
            if saved.is_some() {
                diesel::update(u::use_cases.find(use_case_id))
                    .set(u::updated_at.eq(now))
                    .execute(conn)?;
            }
            Ok(saved)
        })
    }

    fn delete_use_case_draft(&mut self, use_case_id: i32) -> Result<bool, RepoError> {
        use schema::use_case_drafts::dsl;
        let mut conn = self.get_conn()?;
        let n = diesel::delete(dsl::use_case_drafts.find(use_case_id)).execute(conn.as_mut())?;
        Ok(n > 0)
    }

    fn publish_use_case_draft(
        &mut self,
        base_revision: i32,
        version: &NewUseCaseVersion,
    ) -> Result<Option<UseCaseVersion>, RepoError> {
        use schema::use_case_drafts::dsl as d;
        use schema::use_case_versions::dsl as v;
        use schema::use_cases::dsl as u;
        let mut conn = self.get_conn()?;
        conn.as_mut().transaction::<_, RepoError, _>(|conn| {
            // Lock the use case row: two publishes must not pick one number.
            u::use_cases
                .find(version.use_case_id)
                .for_update()
                .select(u::id)
                .first::<i32>(conn)
                .map_err(not_found)?;
            let deleted = diesel::delete(
                d::use_case_drafts
                    .filter(d::use_case_id.eq(version.use_case_id))
                    .filter(d::revision.eq(base_revision)),
            )
            .execute(conn)?;
            if deleted == 0 {
                return Ok(None);
            }
            let last: Option<i32> = v::use_case_versions
                .filter(v::use_case_id.eq(version.use_case_id))
                .select(diesel::dsl::max(v::version_number))
                .first(conn)?;
            let now = chrono::Utc::now().naive_utc();
            let created: UseCaseVersion = diesel::insert_into(v::use_case_versions)
                .values((
                    v::use_case_id.eq(version.use_case_id),
                    v::version_number.eq(last.unwrap_or(0) + 1),
                    v::document.eq(&version.document),
                    v::title.eq(&version.title),
                    v::primary_actor.eq(&version.primary_actor),
                    v::change_note.eq(&version.change_note),
                    v::created_by.eq(version.created_by),
                    v::created_at.eq(now),
                ))
                .get_result(conn)?;
            diesel::update(u::use_cases.find(version.use_case_id))
                .set((u::current_version_id.eq(created.id), u::updated_at.eq(now)))
                .execute(conn)?;
            Ok(Some(created))
        })
    }

    fn list_use_case_links(&self, use_case_id: i32) -> Result<Vec<UseCaseLink>, RepoError> {
        use schema::use_case_links::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::use_case_links
            .filter(dsl::use_case_id.eq(use_case_id))
            .order(dsl::id)
            .load(conn.as_mut())?)
    }

    fn list_use_case_links_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<UseCaseLink>, RepoError> {
        use schema::use_case_links::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::use_case_links
            .filter(dsl::project_id.eq(project_id))
            .order(dsl::id)
            .load(conn.as_mut())?)
    }

    fn list_use_case_links_for_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<UseCaseLink>, RepoError> {
        use schema::use_case_links::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::use_case_links
            .filter(dsl::requirement_id.eq(requirement_id))
            .order(dsl::id)
            .load(conn.as_mut())?)
    }

    fn list_use_case_links_for_verification(
        &self,
        verification_id: i32,
    ) -> Result<Vec<UseCaseLink>, RepoError> {
        use schema::use_case_links::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::use_case_links
            .filter(dsl::verification_id.eq(verification_id))
            .order(dsl::id)
            .load(conn.as_mut())?)
    }

    fn get_use_case_link(&self, id: i32) -> Result<UseCaseLink, RepoError> {
        use schema::use_case_links::dsl;
        let mut conn = self.get_conn()?;
        dsl::use_case_links
            .find(id)
            .first(conn.as_mut())
            .map_err(not_found)
    }

    fn create_use_case_link(&mut self, new: &NewUseCaseLink) -> Result<UseCaseLink, RepoError> {
        let mut conn = self.get_conn()?;
        diesel::insert_into(schema::use_case_links::table)
            .values(new)
            .get_result(conn.as_mut())
            .map_err(map_db_error)
    }

    fn delete_use_case_link(&mut self, id: i32) -> Result<bool, RepoError> {
        use schema::use_case_links::dsl;
        let mut conn = self.get_conn()?;
        Ok(diesel::delete(dsl::use_case_links.find(id)).execute(conn.as_mut())? > 0)
    }

    fn clear_use_case_link_suspect(
        &mut self,
        id: i32,
        user_id: i32,
        current_version_id: Option<i32>,
    ) -> Result<bool, RepoError> {
        use schema::use_case_links::dsl;
        let mut conn = self.get_conn()?;
        let n = diesel::update(dsl::use_case_links.find(id).filter(dsl::suspect.eq(true)))
            .set((
                dsl::suspect.eq(false),
                dsl::suspect_at.eq(Option::<chrono::NaiveDateTime>::None),
                dsl::suspect_reason.eq(Option::<String>::None),
                dsl::cleared_by.eq(user_id),
                dsl::cleared_at.eq(chrono::Utc::now().naive_utc()),
                dsl::linked_requirement_version_id.eq(current_version_id),
            ))
            .execute(conn.as_mut())?;
        Ok(n > 0)
    }

    fn delete_use_case_links_outside(
        &mut self,
        use_case_id: i32,
        keep: &[String],
    ) -> Result<Vec<UseCaseLink>, RepoError> {
        use schema::use_case_links::dsl;
        let mut conn = self.get_conn()?;
        Ok(diesel::delete(
            dsl::use_case_links
                .filter(dsl::use_case_id.eq(use_case_id))
                .filter(dsl::element_id.is_not_null())
                .filter(diesel::dsl::not(dsl::element_id.eq_any(keep))),
        )
        .get_results(conn.as_mut())?)
    }

    fn list_use_case_comments(
        &self,
        use_case_id: i32,
        version_id: Option<i32>,
        element_id: Option<&str>,
    ) -> Result<Vec<UseCaseComment>, RepoError> {
        use schema::use_case_comments::dsl;
        let mut conn = self.get_conn()?;
        let mut query = dsl::use_case_comments
            .filter(dsl::use_case_id.eq(use_case_id))
            .into_boxed();
        if let Some(vid) = version_id {
            query = query.filter(
                dsl::use_case_version_id
                    .eq(vid)
                    .or(dsl::use_case_version_id.is_null()),
            );
        }
        if let Some(element) = element_id {
            query = query.filter(dsl::element_id.eq(element));
        }
        Ok(query
            .order((dsl::created_at, dsl::id))
            .load(conn.as_mut())?)
    }

    fn create_use_case_comment(
        &mut self,
        new: &NewUseCaseComment,
    ) -> Result<UseCaseComment, RepoError> {
        let mut conn = self.get_conn()?;
        Ok(diesel::insert_into(schema::use_case_comments::table)
            .values(new)
            .get_result(conn.as_mut())?)
    }

    fn list_baseline_use_cases(&self, baseline_id: i32) -> Result<Vec<BaselineUseCase>, RepoError> {
        use schema::baseline_use_cases::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::baseline_use_cases
            .filter(dsl::baseline_id.eq(baseline_id))
            .order(dsl::use_case_id)
            .load(conn.as_mut())?)
    }

    fn list_baseline_use_case_links(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<BaselineUseCaseLink>, RepoError> {
        use schema::baseline_use_case_links::dsl;
        let mut conn = self.get_conn()?;
        Ok(dsl::baseline_use_case_links
            .filter(dsl::baseline_id.eq(baseline_id))
            .order(dsl::id)
            .load(conn.as_mut())?)
    }

    fn use_case_in_baseline(&self, use_case_id: i32) -> Result<bool, RepoError> {
        use schema::baseline_use_cases::dsl;
        let mut conn = self.get_conn()?;
        Ok(diesel::select(diesel::dsl::exists(
            dsl::baseline_use_cases.filter(dsl::use_case_id.eq(use_case_id)),
        ))
        .get_result(conn.as_mut())?)
    }
}

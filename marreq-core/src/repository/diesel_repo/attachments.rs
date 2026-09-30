// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::DieselRepo;
use crate::models::{Attachment, NewAttachment};
use crate::repository::errors::RepoError;
use crate::repository::{AttachmentsRepository, QuotaCheck, StorageUsage};
use crate::schema;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer};

#[derive(QueryableByName)]
struct Usage {
    #[diesel(sql_type = BigInt)]
    used_bytes: i64,
    #[diesel(sql_type = BigInt)]
    retained_bytes: i64,
}

/// Distinct files of a project; the second sum only counts files kept
/// solely by deleted rows (i.e. by baselines).
const USAGE_SQL: &str = "
SELECT
  COALESCE(SUM(size_bytes), 0)::BIGINT AS used_bytes,
  COALESCE(SUM(size_bytes) FILTER (WHERE NOT live), 0)::BIGINT AS retained_bytes
FROM (
  SELECT sha256, MAX(size_bytes) AS size_bytes, BOOL_OR(deleted_at IS NULL) AS live
  FROM attachments
  WHERE project_id = $1
  GROUP BY sha256
) files";

fn not_found(e: diesel::result::Error) -> RepoError {
    if e == diesel::result::Error::NotFound {
        RepoError::NotFound
    } else {
        e.into()
    }
}

fn usage(conn: &mut PgConnection, project_id: i32) -> Result<StorageUsage, RepoError> {
    let row: Usage = diesel::sql_query(USAGE_SQL)
        .bind::<Integer, _>(project_id)
        .get_result(conn)?;
    Ok(StorageUsage {
        used_bytes: row.used_bytes,
        retained_by_baselines_bytes: row.retained_bytes,
    })
}

impl AttachmentsRepository for DieselRepo {
    fn list_attachments(
        &self,
        project_id: i32,
        entity_type: &str,
        entity_id: i32,
    ) -> Result<Vec<Attachment>, RepoError> {
        use schema::attachments::dsl;
        let mut conn = self.get_conn()?;
        dsl::attachments
            .filter(dsl::project_id.eq(project_id))
            .filter(dsl::entity_type.eq(entity_type))
            .filter(dsl::entity_id.eq(entity_id))
            .filter(dsl::deleted_at.is_null())
            .order((dsl::created_at.asc(), dsl::id.asc()))
            .select(Attachment::as_select())
            .load(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn get_attachment(&self, id: i32) -> Result<Attachment, RepoError> {
        use schema::attachments::dsl;
        let mut conn = self.get_conn()?;
        dsl::attachments
            .filter(dsl::id.eq(id))
            .select(Attachment::as_select())
            .get_result(conn.as_mut())
            .map_err(not_found)
    }

    fn create_attachment_within_quota(
        &mut self,
        new: &NewAttachment,
        quota_bytes: i64,
    ) -> Result<QuotaCheck, RepoError> {
        use schema::attachments::dsl;
        let mut conn = self.get_conn()?;
        conn.as_mut().transaction::<_, RepoError, _>(|conn| {
            // Serialise uploads per project so two can't both pass the check.
            schema::projects::table
                .filter(schema::projects::id.eq(new.project_id))
                .select(schema::projects::id)
                .for_update()
                .get_result::<i32>(conn)
                .map_err(not_found)?;
            let already_stored = diesel::select(diesel::dsl::exists(
                dsl::attachments
                    .filter(dsl::project_id.eq(new.project_id))
                    .filter(dsl::sha256.eq(&new.sha256)),
            ))
            .get_result::<bool>(conn)?;
            if !already_stored {
                let used = usage(conn, new.project_id)?.used_bytes;
                if used + new.size_bytes > quota_bytes {
                    return Ok(QuotaCheck::Exceeded { used_bytes: used });
                }
            }
            let row = diesel::insert_into(dsl::attachments)
                .values(new)
                .returning(Attachment::as_returning())
                .get_result(conn)?;
            Ok(QuotaCheck::Created(row))
        })
    }

    fn soft_delete_attachment(&mut self, id: i32) -> Result<Attachment, RepoError> {
        use schema::attachments::dsl;
        let mut conn = self.get_conn()?;
        diesel::update(
            dsl::attachments
                .filter(dsl::id.eq(id))
                .filter(dsl::deleted_at.is_null()),
        )
        .set(dsl::deleted_at.eq(Some(chrono::Utc::now().naive_utc())))
        .returning(Attachment::as_returning())
        .get_result(conn.as_mut())
        .map_err(not_found)
    }

    fn soft_delete_attachments_for_entity(
        &mut self,
        project_id: i32,
        entity_type: &str,
        entity_id: i32,
    ) -> Result<Vec<Attachment>, RepoError> {
        use schema::attachments::dsl;
        let mut conn = self.get_conn()?;
        diesel::update(
            dsl::attachments
                .filter(dsl::project_id.eq(project_id))
                .filter(dsl::entity_type.eq(entity_type))
                .filter(dsl::entity_id.eq(entity_id))
                .filter(dsl::deleted_at.is_null()),
        )
        .set(dsl::deleted_at.eq(Some(chrono::Utc::now().naive_utc())))
        .returning(Attachment::as_returning())
        .get_results(conn.as_mut())
        .map_err(RepoError::from)
    }

    fn purge_attachment_if_unreferenced(&mut self, id: i32) -> Result<Option<String>, RepoError> {
        use schema::attachments::dsl;
        use schema::baseline_attachments::dsl as ba;
        let mut conn = self.get_conn()?;
        conn.as_mut().transaction::<_, RepoError, _>(|conn| {
            let row: Attachment = dsl::attachments
                .filter(dsl::id.eq(id))
                .select(Attachment::as_select())
                .for_update()
                .get_result(conn)
                .map_err(not_found)?;
            if row.deleted_at.is_none() {
                return Ok(None);
            }
            let kept = diesel::select(diesel::dsl::exists(
                ba::baseline_attachments.filter(ba::attachment_id.eq(id)),
            ))
            .get_result::<bool>(conn)?;
            if kept {
                return Ok(None);
            }
            diesel::delete(dsl::attachments.filter(dsl::id.eq(id))).execute(conn)?;
            let still_used = diesel::select(diesel::dsl::exists(
                dsl::attachments.filter(dsl::sha256.eq(&row.sha256)),
            ))
            .get_result::<bool>(conn)?;
            Ok((!still_used).then_some(row.sha256))
        })
    }

    fn attachment_blob_in_use(&self, sha256: &str) -> Result<bool, RepoError> {
        use schema::attachments::dsl;
        let mut conn = self.get_conn()?;
        diesel::select(diesel::dsl::exists(
            dsl::attachments.filter(dsl::sha256.eq(sha256)),
        ))
        .get_result(conn.as_mut())
        .map_err(RepoError::from)
    }

    fn project_storage_usage(&self, project_id: i32) -> Result<StorageUsage, RepoError> {
        let mut conn = self.get_conn()?;
        usage(conn.as_mut(), project_id)
    }

    fn get_project_storage_quota(&self, project_id: i32) -> Result<Option<i64>, RepoError> {
        use schema::project_storage_quotas::dsl;
        let mut conn = self.get_conn()?;
        dsl::project_storage_quotas
            .filter(dsl::project_id.eq(project_id))
            .select(dsl::quota_bytes)
            .get_result(conn.as_mut())
            .optional()
            .map_err(RepoError::from)
    }

    fn set_project_storage_quota(
        &mut self,
        project_id: i32,
        quota_bytes: Option<i64>,
        updated_by: i32,
    ) -> Result<(), RepoError> {
        use schema::project_storage_quotas::dsl;
        let mut conn = self.get_conn()?;
        match quota_bytes {
            Some(bytes) => {
                let now = chrono::Utc::now().naive_utc();
                diesel::insert_into(dsl::project_storage_quotas)
                    .values((
                        dsl::project_id.eq(project_id),
                        dsl::quota_bytes.eq(bytes),
                        dsl::updated_by.eq(Some(updated_by)),
                        dsl::updated_at.eq(now),
                    ))
                    .on_conflict(dsl::project_id)
                    .do_update()
                    .set((
                        dsl::quota_bytes.eq(bytes),
                        dsl::updated_by.eq(Some(updated_by)),
                        dsl::updated_at.eq(now),
                    ))
                    .execute(conn.as_mut())?;
            }
            None => {
                diesel::delete(dsl::project_storage_quotas.filter(dsl::project_id.eq(project_id)))
                    .execute(conn.as_mut())?;
            }
        }
        Ok(())
    }

    fn list_baseline_attachments(&self, baseline_id: i32) -> Result<Vec<Attachment>, RepoError> {
        use schema::attachments::dsl;
        use schema::baseline_attachments::dsl as ba;
        let mut conn = self.get_conn()?;
        dsl::attachments
            .inner_join(ba::baseline_attachments.on(ba::attachment_id.eq(dsl::id)))
            .filter(ba::baseline_id.eq(baseline_id))
            .order((dsl::entity_type.asc(), dsl::entity_id.asc(), dsl::id.asc()))
            .select(Attachment::as_select())
            .load(conn.as_mut())
            .map_err(RepoError::from)
    }
}

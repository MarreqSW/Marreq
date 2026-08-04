// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::{map_db_error, DieselRepo};
use crate::models::entities::*;
use crate::models::forms::*;
use crate::repository::errors::RepoError;
use crate::repository::MatrixRepository;
use crate::schema;
use diesel::prelude::*;
use diesel::OptionalExtension;

impl MatrixRepository for DieselRepo {
    fn get_matrix_by_project(&self, pid: i32) -> Result<Vec<MatrixLink>, RepoError> {
        use schema::matrix::dsl;
        let mut conn = self.get_conn()?;
        dsl::matrix
            .filter(dsl::project_id.eq(pid))
            .load::<MatrixLink>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn insert_new_matrix_item(&mut self, new: &NewMatrixLink) -> Result<(), RepoError> {
        let mut conn = self.get_conn()?;
        diesel::insert_into(schema::matrix::table)
            .values(new)
            .execute(conn.as_mut())
            .map_err(map_db_error)?;
        Ok(())
    }

    fn mark_links_suspect_for_requirement(
        &mut self,
        requirement_id: i32,
        reason: &str,
        triggering_version_id: Option<i32>,
        triggering_user_id: Option<i32>,
    ) -> Result<Vec<i32>, RepoError> {
        use schema::matrix::dsl;
        let now = chrono::Utc::now().naive_utc();
        let mut conn = self.get_conn()?;
        let updated: Vec<i32> = diesel::update(dsl::matrix.filter(dsl::req_id.eq(requirement_id)))
            .set((
                dsl::suspect.eq(true),
                dsl::suspect_at.eq(now),
                dsl::suspect_reason.eq(reason),
                dsl::cleared_by.eq(Option::<i32>::None),
                dsl::cleared_at.eq(Option::<chrono::NaiveDateTime>::None),
                dsl::triggering_version_id.eq(triggering_version_id),
                dsl::triggering_user_id.eq(triggering_user_id),
            ))
            .returning(dsl::project_id)
            .get_results(conn.as_mut())?;
        Ok(updated.into_iter().collect())
    }

    fn clear_suspect(
        &mut self,
        req_id: i32,
        verification_id: i32,
        cleared_by_user_id: i32,
    ) -> Result<(bool, Option<i32>), RepoError> {
        use schema::matrix::dsl;
        let now = chrono::Utc::now().naive_utc();
        let mut conn = self.get_conn()?;
        let project_id: Option<i32> = diesel::update(
            dsl::matrix
                .filter(dsl::req_id.eq(req_id))
                .filter(dsl::verification_id.eq(verification_id)),
        )
        .set((
            dsl::suspect.eq(false),
            dsl::suspect_at.eq(Option::<chrono::NaiveDateTime>::None),
            dsl::suspect_reason.eq(Option::<String>::None),
            dsl::cleared_by.eq(cleared_by_user_id),
            dsl::cleared_at.eq(now),
        ))
        .returning(dsl::project_id)
        .get_result(conn.as_mut())
        .optional()?;
        Ok((project_id.is_some(), project_id))
    }
}

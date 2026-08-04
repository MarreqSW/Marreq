// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::DieselRepo;
use crate::models::entities::*;
use crate::repository::errors::RepoError;
use crate::repository::RequirementCommentsRepository;
use crate::schema;
use diesel::expression_methods::BoolExpressionMethods;
use diesel::prelude::*;

impl RequirementCommentsRepository for DieselRepo {
    fn insert_requirement_comment(
        &mut self,
        new: &NewRequirementComment,
    ) -> Result<RequirementComment, RepoError> {
        let mut conn = self.get_conn()?;
        diesel::insert_into(schema::requirement_comments::table)
            .values(new)
            .returning(schema::requirement_comments::all_columns)
            .get_result(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn list_comments_by_requirement(
        &self,
        requirement_id: i32,
        version_id: Option<i32>,
    ) -> Result<Vec<RequirementComment>, RepoError> {
        use schema::requirement_comments::dsl;
        let mut conn = self.get_conn()?;
        let q = dsl::requirement_comments
            .filter(dsl::requirement_id.eq(requirement_id))
            .order(dsl::created_at.asc());
        let rows = match version_id {
            Some(vid) => q
                .filter(
                    dsl::requirement_version_id
                        .is_null()
                        .or(dsl::requirement_version_id.eq(vid)),
                )
                .load::<RequirementComment>(conn.as_mut()),
            None => q.load::<RequirementComment>(conn.as_mut()),
        };
        rows.map_err(RepoError::from)
    }
}

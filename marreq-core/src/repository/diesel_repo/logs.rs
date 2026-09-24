// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::DieselRepo;
use crate::models::entities::*;
use crate::models::forms::*;
use crate::repository::errors::RepoError;
use crate::repository::{LogListQuery, LogRepository};
use crate::schema;
use diesel::pg::Pg;
use diesel::prelude::*;

fn apply_log_filters<'a>(
    mut q: schema::logs::BoxedQuery<'a, Pg>,
    query: &LogListQuery,
) -> schema::logs::BoxedQuery<'a, Pg> {
    use schema::logs::dsl::*;
    if let Some(ref v) = query.entity_type {
        q = q.filter(entity_type.eq(v.clone()));
    }
    if let Some(id) = query.entity_id {
        q = q.filter(entity_id.eq(id));
    }
    if let Some(id) = query.user_id {
        q = q.filter(user_id.eq(id));
    }
    if let Some(ref v) = query.action_type {
        q = q.filter(action_type.eq(v.clone()));
    }
    if let Some(id) = query.project_id {
        q = q.filter(project_id.eq(id));
    }
    if let Some(since) = query.since {
        q = q.filter(created_at.ge(since));
    }
    if let Some(until) = query.until {
        q = q.filter(created_at.le(until));
    }
    q
}

impl LogRepository for DieselRepo {
    fn insert_log(&mut self, new: &NewLog) -> Result<(), RepoError> {
        let mut conn = self.get_conn()?;
        diesel::insert_into(schema::logs::table)
            .values(new)
            .execute(conn.as_mut())?;
        Ok(())
    }

    fn get_logs_recent(&self, limit: i64) -> Result<Vec<Log>, RepoError> {
        use schema::logs::dsl::*;
        let mut conn = self.get_conn()?;
        logs.order(created_at.desc())
            .limit(limit)
            .load::<Log>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_logs_by_entity(&self, etype: &str, eid: i32) -> Result<Vec<Log>, RepoError> {
        use schema::logs::dsl::*;
        let mut conn = self.get_conn()?;
        logs.filter(entity_type.eq(etype))
            .filter(entity_id.eq(eid))
            .order(created_at.desc())
            .load::<Log>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_logs_filtered(&self, query: &LogListQuery) -> Result<(Vec<Log>, i64), RepoError> {
        use schema::logs::dsl::*;
        let mut conn = self.get_conn()?;
        let total = apply_log_filters(logs.into_boxed(), query)
            .count()
            .get_result::<i64>(conn.as_mut())?;
        let rows = apply_log_filters(logs.into_boxed(), query)
            .order(created_at.desc())
            .limit(query.limit)
            .offset(query.offset)
            .load::<Log>(conn.as_mut())?;
        Ok((rows, total))
    }

    fn cleanup_logs(&mut self, days: i64) -> Result<usize, RepoError> {
        use schema::logs::dsl::*;
        let mut conn = self.get_conn()?;
        let cutoff = chrono::Utc::now().naive_utc() - chrono::Duration::days(days);
        let count = diesel::delete(logs.filter(created_at.lt(cutoff))).execute(conn.as_mut())?;
        Ok(count)
    }
}

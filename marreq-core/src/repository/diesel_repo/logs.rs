// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::DieselRepo;
use crate::models::entities::*;
use crate::models::forms::*;
use crate::repository::errors::RepoError;
use crate::repository::LogRepository;
use crate::schema;
use diesel::prelude::*;

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

    fn cleanup_logs(&mut self, days: i64) -> Result<usize, RepoError> {
        use schema::logs::dsl::*;
        let mut conn = self.get_conn()?;
        let cutoff = chrono::Utc::now().naive_utc() - chrono::Duration::days(days);
        let count = diesel::delete(logs.filter(created_at.lt(cutoff))).execute(conn.as_mut())?;
        Ok(count)
    }
}

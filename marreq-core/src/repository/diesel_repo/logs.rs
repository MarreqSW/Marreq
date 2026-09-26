// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::DieselRepo;
use crate::models::entities::*;
use crate::models::forms::*;
use crate::repository::errors::RepoError;
use crate::repository::{LogListQuery, LogRepository, LogStats};
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

#[derive(QueryableByName)]
struct DailyCount {
    #[diesel(sql_type = diesel::sql_types::Date)]
    day: chrono::NaiveDate,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct ActionCount {
    #[diesel(sql_type = diesel::sql_types::Text)]
    action_type: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct UserCount {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    user_id: i32,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

type BoxedRawQuery<'a> =
    diesel::query_builder::BoxedSqlQuery<'a, Pg, diesel::query_builder::SqlQuery>;

/// `SELECT … FROM logs WHERE <filters>` with the same filters as [`apply_log_filters`],
/// as numbered bind parameters. Returns the query and the next free placeholder
/// number. Used for the GROUP BY aggregates, which boxed DSL queries cannot express.
fn filtered_logs_sql<'a>(select: &str, query: &LogListQuery) -> (BoxedRawQuery<'a>, usize) {
    use diesel::sql_types::{Integer, Text, Timestamp};
    let mut n = 0usize;
    let mut next = || {
        n += 1;
        n
    };
    let mut q = diesel::sql_query(format!("{select} FROM logs WHERE TRUE")).into_boxed::<Pg>();
    if let Some(ref v) = query.entity_type {
        q = q
            .sql(format!(" AND entity_type = ${}", next()))
            .bind::<Text, _>(v.clone());
    }
    if let Some(id) = query.entity_id {
        q = q
            .sql(format!(" AND entity_id = ${}", next()))
            .bind::<Integer, _>(id);
    }
    if let Some(id) = query.user_id {
        q = q
            .sql(format!(" AND user_id = ${}", next()))
            .bind::<Integer, _>(id);
    }
    if let Some(ref v) = query.action_type {
        q = q
            .sql(format!(" AND action_type = ${}", next()))
            .bind::<Text, _>(v.clone());
    }
    if let Some(id) = query.project_id {
        q = q
            .sql(format!(" AND project_id = ${}", next()))
            .bind::<Integer, _>(id);
    }
    if let Some(since) = query.since {
        q = q
            .sql(format!(" AND created_at >= ${}", next()))
            .bind::<Timestamp, _>(since);
    }
    if let Some(until) = query.until {
        q = q
            .sql(format!(" AND created_at <= ${}", next()))
            .bind::<Timestamp, _>(until);
    }
    (q, next())
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

    fn get_log_stats(&self, query: &LogListQuery, top: i64) -> Result<LogStats, RepoError> {
        use diesel::sql_types::BigInt;
        use schema::logs::dsl::*;
        let mut conn = self.get_conn()?;

        let total = apply_log_filters(logs.into_boxed(), query)
            .count()
            .get_result::<i64>(conn.as_mut())?;

        // `created_at` is stored as naive UTC, so `::date` buckets by UTC day.
        let (distinct_users, _) =
            filtered_logs_sql("SELECT COUNT(DISTINCT user_id)::BIGINT AS count", query);
        let active_users = distinct_users.get_result::<Count>(conn.as_mut())?.count;

        let (daily, _) = filtered_logs_sql(
            "SELECT created_at::date AS day, COUNT(*)::BIGINT AS count",
            query,
        );
        let by_day = daily
            .sql(" GROUP BY day ORDER BY day")
            .load::<DailyCount>(conn.as_mut())?
            .into_iter()
            .map(|row| (row.day, row.count))
            .collect();

        let (actions, limit_param) =
            filtered_logs_sql("SELECT action_type, COUNT(*)::BIGINT AS count", query);
        let by_action = actions
            .sql(format!(
                " GROUP BY action_type ORDER BY count DESC, action_type LIMIT ${limit_param}"
            ))
            .bind::<BigInt, _>(top)
            .load::<ActionCount>(conn.as_mut())?
            .into_iter()
            .map(|row| (row.action_type, row.count))
            .collect();

        let (users, limit_param) =
            filtered_logs_sql("SELECT user_id, COUNT(*)::BIGINT AS count", query);
        let by_user = users
            .sql(format!(
                " GROUP BY user_id ORDER BY count DESC, user_id LIMIT ${limit_param}"
            ))
            .bind::<BigInt, _>(top)
            .load::<UserCount>(conn.as_mut())?
            .into_iter()
            .map(|row| (row.user_id, row.count))
            .collect();

        Ok(LogStats {
            total,
            active_users,
            by_day,
            by_action,
            by_user,
        })
    }
}

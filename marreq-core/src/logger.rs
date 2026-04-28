// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::models::{ActionType, EntityType, Log, NewLog};
use crate::schema::logs;
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::pg::PgConnection;
use diesel::prelude::*;

#[derive(Debug)]
pub enum LoggerError {
    Db(diesel::result::Error),
    Json(serde_json::Error),
    Other(String),
}

impl std::fmt::Display for LoggerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoggerError::Db(err) => write!(f, "database error: {}", err),
            LoggerError::Json(err) => write!(f, "serialization error: {}", err),
            LoggerError::Other(msg) => write!(f, "logger error: {}", msg),
        }
    }
}

impl std::error::Error for LoggerError {}

impl From<diesel::result::Error> for LoggerError {
    fn from(value: diesel::result::Error) -> Self {
        LoggerError::Db(value)
    }
}

impl From<serde_json::Error> for LoggerError {
    fn from(value: serde_json::Error) -> Self {
        LoggerError::Json(value)
    }
}

#[derive(Debug, Clone, Default)]
pub struct LogCtx {
    id: i32,
    ip_address: Option<String>,
    user_agent: Option<String>,
}

impl LogCtx {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            ip_address: None,
            user_agent: None,
        }
    }

    pub fn from_request(id: i32, request: &rocket::Request<'_>) -> Self {
        Self::new(id).with_request(request)
    }

    pub fn from_optional_request(id: i32, request: Option<&rocket::Request<'_>>) -> Self {
        request
            .map(|req| Self::new(id).with_request(req))
            .unwrap_or_else(|| Self::new(id))
    }

    pub fn with_request(mut self, request: &rocket::Request<'_>) -> Self {
        self.ip_address = request.remote().map(|addr| addr.ip().to_string());
        self.user_agent = request
            .headers()
            .get_one("User-Agent")
            .map(|s| s.to_string());
        self
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn ip_address(&self) -> Option<&str> {
        self.ip_address.as_deref()
    }

    pub fn user_agent(&self) -> Option<&str> {
        self.user_agent.as_deref()
    }
}

pub trait Loggable {
    fn entity_type() -> EntityType
    where
        Self: Sized;
    fn id(&self) -> i32;
    fn project_id(&self) -> Option<i32>;
    fn display_name(&self) -> String;
}

pub struct Logger;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogEntity {
    Project,
    Requirement,
    Test,
    Category,
    Applicability,
    User,
    MatrixLink,
    Verification,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogAction {
    Create(LogEntity),
    Update(LogEntity),
    Delete(LogEntity),
    Login,
    Logout,
    Export,
    Import,
    StatusChange,
}

impl Logger {
    pub fn created<T: serde::Serialize + Loggable>(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        created_id: i32,
        entity: &T,
    ) -> Result<(), LoggerError> {
        let payload = Some(Self::to_json_string(entity)?);
        Self::log_entity_action(
            conn,
            ctx,
            ActionType::Create,
            entity,
            Some(created_id),
            None,
            payload,
        )
    }

    pub fn updated<T: serde::Serialize + Loggable>(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        before: &T,
        after: &T,
    ) -> Result<(), LoggerError> {
        let oldv = Self::to_json_string(before)?;
        let newv = Self::to_json_string(after)?;
        let old_stripped = Self::strip_timestamps_for_compare(&oldv)?;
        let new_stripped = Self::strip_timestamps_for_compare(&newv)?;
        if old_stripped == new_stripped {
            return Ok(());
        }
        Self::log_entity_action(
            conn,
            ctx,
            ActionType::Update,
            after,
            None,
            Some(oldv),
            Some(newv),
        )
    }

    pub fn deleted<T: serde::Serialize + Loggable>(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        entity: &T,
    ) -> Result<(), LoggerError> {
        let oldv = Some(Self::to_json_string(entity)?);
        Self::log_entity_action(conn, ctx, ActionType::Delete, entity, None, oldv, None)
    }

    pub fn log_login(conn: &mut PgConnection, ctx: &LogCtx) -> Result<(), LoggerError> {
        Self::log_action(
            conn,
            ctx,
            ActionType::Login,
            Some(EntityType::User),
            Some(ctx.id()),
            None,
            None,
            None,
            Some("User logged in".to_string()),
        )
    }

    pub fn log_logout(conn: &mut PgConnection, ctx: &LogCtx) -> Result<(), LoggerError> {
        Self::log_action(
            conn,
            ctx,
            ActionType::Logout,
            Some(EntityType::User),
            Some(ctx.id()),
            None,
            None,
            None,
            Some("User logged out".to_string()),
        )
    }

    pub fn log_unauthorized(conn: &mut PgConnection, ctx: &LogCtx) -> Result<(), LoggerError> {
        let new_log = NewLog {
            user_id: ctx.id(),
            action_type: "ILLEGAL_ACCESS".to_string(),
            entity_type: "entity_type".to_string(),
            project_id: None,
            entity_id: None,
            old_values: None,
            new_values: None,
            description: None,
            ip_address: ctx.ip_address().map(str::to_owned),
            user_agent: ctx.user_agent().map(str::to_owned),
        };
        diesel::insert_into(logs::table)
            .values(&new_log)
            .execute(conn)
            .map(|_| ())
            .map_err(LoggerError::from)
    }

    pub fn log_export(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        description: Option<String>,
    ) -> Result<(), LoggerError> {
        Self::log_action(
            conn,
            ctx,
            ActionType::Export,
            None,
            None,
            None,
            None,
            description,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn log_custom(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        action_type: ActionType,
        entity_type: EntityType,
        entity_id: Option<i32>,
        project_id: Option<i32>,
        old_values: Option<String>,
        new_values: Option<String>,
        description: Option<String>,
    ) -> Result<(), LoggerError> {
        Self::log_action(
            conn,
            ctx,
            action_type,
            Some(entity_type),
            entity_id,
            project_id,
            old_values,
            new_values,
            description,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn log_action(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        action_type: ActionType,
        entity_type: Option<EntityType>, // still optional in input
        entity_id: Option<i32>,
        project_id: Option<i32>,
        old_values: Option<String>,
        new_values: Option<String>,
        description: Option<String>,
    ) -> Result<(), LoggerError> {
        let new_log = NewLog {
            user_id: ctx.id(),
            action_type: action_type.to_string(),
            // If None, default to empty string
            entity_type: entity_type.map(|et| et.to_string()).unwrap_or_default(),
            entity_id,
            project_id,
            old_values,
            new_values,
            description,
            ip_address: ctx.ip_address().map(str::to_owned),
            user_agent: ctx.user_agent().map(str::to_owned),
        };

        diesel::insert_into(logs::table)
            .values(&new_log)
            .execute(conn)
            .map(|_| ())
            .map_err(LoggerError::from)
    }

    fn log_entity_action<T: serde::Serialize + Loggable>(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        action_type: ActionType,
        entity: &T,
        entity_id_override: Option<i32>,
        old_values: Option<String>,
        new_values: Option<String>,
    ) -> Result<(), LoggerError> {
        let entity_type = T::entity_type();
        let entity_id = entity_id_override.or_else(|| Self::entity_id_for_logging(entity));
        let description = Some(Self::describe_entity_action(
            action_type,
            entity_type,
            entity,
        ));

        Self::log_action(
            conn,
            ctx,
            action_type,
            Some(entity_type),
            entity_id,
            entity.project_id(),
            old_values,
            new_values,
            description,
        )
    }

    fn entity_id_for_logging<T: Loggable>(entity: &T) -> Option<i32> {
        match entity.id() {
            0 => None,
            id => Some(id),
        }
    }

    fn describe_entity_action<T: Loggable>(
        action_type: ActionType,
        entity_type: EntityType,
        entity: &T,
    ) -> String {
        format!(
            "{} {} via API: {}",
            action_type.past_tense(),
            entity_type.human_name(),
            entity.display_name()
        )
    }

    pub fn get_logs_for_entity(
        conn: &mut PgConnection,
        entity_type_param: &str,
        entity_id_param: i32,
    ) -> Result<Vec<Log>, LoggerError> {
        use crate::schema::logs::dsl::*;

        let logs_list = logs
            .filter(entity_type.eq(entity_type_param))
            .filter(entity_id.eq(entity_id_param))
            .order(created_at.desc())
            .load::<Log>(conn)
            .map_err(LoggerError::from)?;

        Ok(logs_list)
    }

    pub fn get_recent_logs(conn: &mut PgConnection, limit: i64) -> Result<Vec<Log>, LoggerError> {
        use crate::schema::logs::dsl::*;

        let logs_list = logs
            .order(created_at.desc())
            .limit(limit)
            .load::<Log>(conn)
            .map_err(LoggerError::from)?;

        Ok(logs_list)
    }

    pub fn to_json_string<T: serde::Serialize>(value: &T) -> Result<String, LoggerError> {
        serde_json::to_string_pretty(value).map_err(LoggerError::from)
    }

    /// Keys removed from JSON before comparing old vs new so that "no real change" edits
    /// (e.g. save without changing any field) do not produce a log entry when only
    /// server-updated timestamps or version ids differ.
    const COMPARE_IGNORE_KEYS: &[&str] = &[
        "update_date",
        "updated_at",
        "current_version_id", // changes every edit (new version row); not content
    ];

    fn strip_timestamps_for_compare(json_str: &str) -> Result<serde_json::Value, LoggerError> {
        let mut v: serde_json::Value = serde_json::from_str(json_str).map_err(LoggerError::from)?;
        Self::strip_timestamps_from_value(&mut v);
        Ok(v)
    }

    fn strip_timestamps_from_value(v: &mut serde_json::Value) {
        if let Some(obj) = v.as_object_mut() {
            for key in Self::COMPARE_IGNORE_KEYS {
                obj.remove(*key);
            }
            for (_k, child) in obj.iter_mut() {
                Self::strip_timestamps_from_value(child);
            }
        }
        if let Some(arr) = v.as_array_mut() {
            for item in arr.iter_mut() {
                Self::strip_timestamps_from_value(item);
            }
        }
    }

    pub fn get_log_count(conn: &mut PgConnection, days: i64) -> Result<i64, LoggerError> {
        crate::logger::get_log_count(conn, Some(days))
    }

    pub fn cleanup_old_logs(conn: &mut PgConnection, days: i64) -> Result<usize, LoggerError> {
        use crate::schema::logs::dsl::*;

        let cutoff_datetime = calculate_cutoff(days)?;
        let deleted_count =
            diesel::delete(logs.filter(created_at.lt(cutoff_datetime))).execute(conn)?;

        Ok(deleted_count)
    }
}

pub fn get_log_count(conn: &mut PgConnection, days: Option<i64>) -> Result<i64, LoggerError> {
    use crate::schema::logs::dsl::*;

    let query = if let Some(days) = days {
        let cutoff_datetime = calculate_cutoff(days)?;
        logs.filter(created_at.ge(cutoff_datetime)).into_boxed()
    } else {
        logs.into_boxed()
    };

    let count = query.count().get_result(conn)?;
    Ok(count)
}

fn calculate_cutoff(days: i64) -> Result<NaiveDateTime, LoggerError> {
    Utc::now()
        .checked_sub_signed(Duration::days(days))
        .ok_or_else(|| LoggerError::Other("Invalid cutoff interval".into()))
        .map(|dt| dt.naive_utc())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[test]
    fn logger_error_display() {
        let err = LoggerError::Other("test message".into());
        assert!(err.to_string().contains("logger error"));
        assert!(err.to_string().contains("test message"));
    }

    #[test]
    fn logger_error_db_display() {
        let err = LoggerError::Db(diesel::result::Error::NotFound);
        assert!(err.to_string().contains("database error"));
    }

    #[test]
    fn logger_error_json_display() {
        let bad_json = serde_json::from_str::<serde_json::Value>("invalid");
        let err = bad_json.unwrap_err();
        let logger_err = LoggerError::Json(err);
        assert!(logger_err.to_string().contains("serialization error"));
    }

    #[test]
    fn logger_error_from_diesel() {
        let e = diesel::result::Error::NotFound;
        let le: LoggerError = e.into();
        assert!(matches!(le, LoggerError::Db(_)));
    }

    #[test]
    fn logger_error_from_serde_json() {
        let e = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        let le: LoggerError = e.into();
        assert!(matches!(le, LoggerError::Json(_)));
    }

    #[test]
    fn log_ctx_new_and_accessors() {
        let ctx = LogCtx::new(42);
        assert_eq!(ctx.id(), 42);
        assert_eq!(ctx.ip_address(), None);
        assert_eq!(ctx.user_agent(), None);
    }

    #[test]
    fn log_ctx_default() {
        let ctx = LogCtx::default();
        assert_eq!(ctx.id(), 0);
    }

    #[test]
    fn log_ctx_clone() {
        let ctx = LogCtx::new(1);
        let c2 = ctx.clone();
        assert_eq!(c2.id(), 1);
    }

    #[test]
    fn logger_to_json_string() {
        #[derive(Serialize)]
        struct Sample {
            id: i32,
            name: String,
        }
        let s = Sample {
            id: 1,
            name: "test".into(),
        };
        let json = Logger::to_json_string(&s).unwrap();
        assert!(json.contains("\"id\": 1"));
        assert!(json.contains("\"name\": \"test\""));
    }

    #[test]
    fn log_entity_variants() {
        assert_eq!(LogEntity::Project, LogEntity::Project);
        assert_ne!(LogEntity::Requirement, LogEntity::Test);
    }

    #[test]
    fn log_action_variants() {
        let _ = LogAction::Create(LogEntity::Requirement);
        let _ = LogAction::Login;
        let _ = LogAction::Export;
    }
}

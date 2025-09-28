use crate::models::{ActionType, EntityType, Log, NewLog};
use crate::schema::logs;
use chrono::{DateTime, Duration, Utc};
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
    user_id: i32,
    ip_address: Option<String>,
    user_agent: Option<String>,
}

impl LogCtx {
    pub fn new(user_id: i32) -> Self {
        Self {
            user_id,
            ip_address: None,
            user_agent: None,
        }
    }

    pub fn from_request(user_id: i32, request: &rocket::Request<'_>) -> Self {
        Self::new(user_id).with_request(request)
    }

    pub fn from_optional_request(user_id: i32, request: Option<&rocket::Request<'_>>) -> Self {
        let mut ctx = Self::new(user_id);
        if let Some(req) = request {
            ctx = ctx.with_request(req);
        }
        ctx
    }

    pub fn with_request(mut self, request: &rocket::Request<'_>) -> Self {
        self.ip_address = request.remote().map(|addr| addr.ip().to_string());
        self.user_agent = request
            .headers()
            .get_one("User-Agent")
            .map(|s| s.to_string());
        self
    }

    pub fn user_id(&self) -> i32 {
        self.user_id
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

impl Logger {
    pub fn created<T: serde::Serialize + Loggable>(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        created_id: i32,
        entity: &T,
    ) -> Result<(), LoggerError> {
        let payload = Some(Self::to_json_string(entity)?);
        Self::log_create(
            conn,
            ctx,
            T::entity_type(),
            created_id,
            entity.project_id(),
            payload,
            Some(format!(
                "Created {} via API: {}",
                std::any::type_name::<T>(),
                entity.display_name()
            )),
        )
    }

    pub fn updated<T: serde::Serialize + Loggable>(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        before: &T,
        after: &T,
    ) -> Result<(), LoggerError> {
        let oldv = Some(Self::to_json_string(before)?);
        let newv = Some(Self::to_json_string(after)?);
        Self::log_update(
            conn,
            ctx,
            T::entity_type(),
            after.id(),
            after.project_id(),
            oldv,
            newv,
            Some(format!(
                "Updated {} via API: {}",
                std::any::type_name::<T>(),
                after.display_name()
            )),
        )
    }

    pub fn deleted<T: serde::Serialize + Loggable>(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        entity: &T,
    ) -> Result<(), LoggerError> {
        let oldv = Some(Self::to_json_string(entity)?);
        Self::log_delete(
            conn,
            ctx,
            T::entity_type(),
            entity.id(),
            entity.project_id(),
            oldv,
            Some(format!(
                "Deleted {} via API: {}",
                std::any::type_name::<T>(),
                entity.display_name()
            )),
        )
    }

    fn log_action(
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
        let new_log = NewLog {
            user_id: ctx.user_id(),
            action_type: action_type.to_string(),
            entity_type: entity_type.to_string(),
            entity_id,
            project_id,
            old_values,
            new_values,
            description,
            ip_address: ctx.ip_address().map(|s| s.to_string()),
            user_agent: ctx.user_agent().map(|s| s.to_string()),
        };

        diesel::insert_into(logs::table)
            .values(&new_log)
            .execute(conn)
            .map(|_| ())
            .map_err(LoggerError::from)
    }

    pub(crate) fn log_create(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        entity_type: EntityType,
        entity_id: i32,
        project_id: Option<i32>,
        new_values: Option<String>,
        description: Option<String>,
    ) -> Result<(), LoggerError> {
        Self::log_action(
            conn,
            ctx,
            ActionType::Create,
            entity_type,
            Some(entity_id),
            project_id,
            None,
            new_values,
            description,
        )
    }

    pub(crate) fn log_update(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        entity_type: EntityType,
        entity_id: i32,
        project_id: Option<i32>,
        old_values: Option<String>,
        new_values: Option<String>,
        description: Option<String>,
    ) -> Result<(), LoggerError> {
        Self::log_action(
            conn,
            ctx,
            ActionType::Update,
            entity_type,
            Some(entity_id),
            project_id,
            old_values,
            new_values,
            description,
        )
    }

    pub(crate) fn log_delete(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        entity_type: EntityType,
        entity_id: i32,
        project_id: Option<i32>,
        old_values: Option<String>,
        description: Option<String>,
    ) -> Result<(), LoggerError> {
        Self::log_action(
            conn,
            ctx,
            ActionType::Delete,
            entity_type,
            Some(entity_id),
            project_id,
            old_values,
            None,
            description,
        )
    }

    pub fn log_login(conn: &mut PgConnection, ctx: &LogCtx) -> Result<(), LoggerError> {
        Self::log_action(
            conn,
            ctx,
            ActionType::Login,
            EntityType::User,
            Some(ctx.user_id()),
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
            EntityType::User,
            Some(ctx.user_id()),
            None,
            None,
            None,
            Some("User logged out".to_string()),
        )
    }

    pub fn log_export(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        entity_type: EntityType,
        entity_id: Option<i32>,
        project_id: Option<i32>,
        description: Option<String>,
    ) -> Result<(), LoggerError> {
        Self::log_action(
            conn,
            ctx,
            ActionType::Export,
            entity_type,
            entity_id,
            project_id,
            None,
            None,
            description,
        )
    }

    pub fn log_import(
        conn: &mut PgConnection,
        ctx: &LogCtx,
        entity_type: EntityType,
        project_id: Option<i32>,
        description: Option<String>,
    ) -> Result<(), LoggerError> {
        Self::log_action(
            conn,
            ctx,
            ActionType::Import,
            entity_type,
            None,
            project_id,
            None,
            None,
            description,
        )
    }

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
            entity_type,
            entity_id,
            project_id,
            old_values,
            new_values,
            description,
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

    pub fn get_logs_for_project(
        conn: &mut PgConnection,
        project_id_param: i32,
    ) -> Result<Vec<Log>, LoggerError> {
        use crate::schema::logs::dsl::*;

        let logs_list = logs
            .filter(project_id.eq(project_id_param))
            .order(created_at.desc())
            .load::<Log>(conn)
            .map_err(LoggerError::from)?;

        Ok(logs_list)
    }

    pub fn get_logs_for_user(
        conn: &mut PgConnection,
        user_id_param: i32,
    ) -> Result<Vec<Log>, LoggerError> {
        use crate::schema::logs::dsl::*;

        let logs_list = logs
            .filter(user_id.eq(user_id_param))
            .order(created_at.desc())
            .load::<Log>(conn)
            .map_err(LoggerError::from)?;

        return Ok(logs_list);
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

    pub fn get_log_count(conn: &mut PgConnection, days: i64) -> Result<i64, LoggerError> {
        get_log_count(conn, Some(days))
    }

    pub fn create_description(
        action_type: &ActionType,
        entity_type: &EntityType,
        _entity_id: Option<i32>,
    ) -> String {
        match (action_type, entity_type) {
            (ActionType::Create, EntityType::Requirement) => "Created new requirement".to_string(),
            (ActionType::Update, EntityType::Requirement) => "Updated requirement".to_string(),
            (ActionType::Delete, EntityType::Requirement) => "Deleted requirement".to_string(),
            (ActionType::Create, EntityType::Test) => "Created new test".to_string(),
            (ActionType::Update, EntityType::Test) => "Updated test".to_string(),
            (ActionType::Delete, EntityType::Test) => "Deleted test".to_string(),
            (ActionType::Create, EntityType::Category) => "Created new category".to_string(),
            (ActionType::Update, EntityType::Category) => "Updated category".to_string(),
            (ActionType::Delete, EntityType::Category) => "Deleted category".to_string(),
            (ActionType::Create, EntityType::Project) => "Created new project".to_string(),
            (ActionType::Update, EntityType::Project) => "Updated project".to_string(),
            (ActionType::Delete, EntityType::Project) => "Deleted project".to_string(),
            (ActionType::Create, EntityType::User) => "Created new user".to_string(),
            (ActionType::Update, EntityType::User) => "Updated user".to_string(),
            (ActionType::Delete, EntityType::User) => "Deleted user".to_string(),
            (ActionType::Create, EntityType::Applicability) => {
                "Created new applicability".to_string()
            }
            (ActionType::Update, EntityType::Applicability) => "Updated applicability".to_string(),
            (ActionType::Delete, EntityType::Applicability) => "Deleted applicability".to_string(),
            _ => format!("{:?} {:?}", action_type, entity_type),
        }
    }
}

pub fn cleanup_old_logs(conn: &mut PgConnection, days: i64) -> Result<usize, LoggerError> {
    use crate::schema::logs::dsl::*;

    let cutoff_timestamp = Utc::now() - Duration::days(days);
    let cutoff_datetime = DateTime::from_timestamp(cutoff_timestamp.timestamp(), 0)
        .ok_or_else(|| LoggerError::Other("Invalid timestamp".into()))?
        .naive_utc();

    let deleted_count =
        diesel::delete(logs.filter(created_at.lt(cutoff_datetime))).execute(conn)?;

    Ok(deleted_count)
}

pub fn get_log_count(conn: &mut PgConnection, days: Option<i64>) -> Result<i64, LoggerError> {
    use crate::schema::logs::dsl::*;

    let query = if let Some(days) = days {
        let cutoff_timestamp = Utc::now() - Duration::days(days);
        let cutoff_datetime = DateTime::from_timestamp(cutoff_timestamp.timestamp(), 0)
            .ok_or_else(|| LoggerError::Other("Invalid timestamp".into()))?
            .naive_utc();

        logs.filter(created_at.ge(cutoff_datetime)).into_boxed()
    } else {
        logs.into_boxed()
    };

    let count = query.count().get_result(conn)?;
    Ok(count)
}

use crate::models::{ActionType, EntityType, Log, NewLog};
use crate::schema::logs;
use diesel::prelude::*;
use diesel::PgConnection;
use rocket::request::Request;
use serde_json::Value;

pub struct Logger;

impl Logger {
    /// Log an action to the database
    pub fn log_action(
        conn: &mut PgConnection,
        user_id: i32,
        action_type: ActionType,
        entity_type: EntityType,
        entity_id: Option<i32>,
        project_id: Option<i32>,
        old_values: Option<Value>,
        new_values: Option<Value>,
        description: Option<String>,
        request: Option<&Request<'_>>,
    ) -> Result<(), diesel::result::Error> {
        let ip_address = request.and_then(|req| {
            req.headers()
                .get_one("X-Real-IP")
                .or_else(|| req.headers().get_one("X-Forwarded-For"))
                .or_else(|| req.remote().map(|addr| addr.ip().to_string().as_str()))
        });

        let user_agent = request
            .and_then(|req| req.headers().get_one("User-Agent"));

        let new_log = NewLog {
            user_id,
            action_type: action_type.to_string(),
            entity_type: entity_type.to_string(),
            entity_id,
            project_id,
            old_values,
            new_values,
            description,
            ip_address: ip_address.map(|s| s.to_string()),
            user_agent: user_agent.map(|s| s.to_string()),
        };

        diesel::insert_into(logs::table)
            .values(&new_log)
            .execute(conn)?;

        Ok(())
    }

    /// Log a creation action
    pub fn log_create(
        conn: &mut PgConnection,
        user_id: i32,
        entity_type: EntityType,
        entity_id: i32,
        project_id: Option<i32>,
        new_values: Value,
        description: Option<String>,
        request: Option<&Request<'_>>,
    ) -> Result<(), diesel::result::Error> {
        Self::log_action(
            conn,
            user_id,
            ActionType::Create,
            entity_type,
            Some(entity_id),
            project_id,
            None,
            Some(new_values),
            description,
            request,
        )
    }

    /// Log an update action
    pub fn log_update(
        conn: &mut PgConnection,
        user_id: i32,
        entity_type: EntityType,
        entity_id: i32,
        project_id: Option<i32>,
        old_values: Value,
        new_values: Value,
        description: Option<String>,
        request: Option<&Request<'_>>,
    ) -> Result<(), diesel::result::Error> {
        Self::log_action(
            conn,
            user_id,
            ActionType::Update,
            entity_type,
            Some(entity_id),
            project_id,
            Some(old_values),
            Some(new_values),
            description,
            request,
        )
    }

    /// Log a deletion action
    pub fn log_delete(
        conn: &mut PgConnection,
        user_id: i32,
        entity_type: EntityType,
        entity_id: i32,
        project_id: Option<i32>,
        old_values: Value,
        description: Option<String>,
        request: Option<&Request<'_>>,
    ) -> Result<(), diesel::result::Error> {
        Self::log_action(
            conn,
            user_id,
            ActionType::Delete,
            entity_type,
            Some(entity_id),
            project_id,
            Some(old_values),
            None,
            description,
            request,
        )
    }

    /// Log a login action
    pub fn log_login(
        conn: &mut PgConnection,
        user_id: i32,
        description: Option<String>,
        request: Option<&Request<'_>>,
    ) -> Result<(), diesel::result::Error> {
        Self::log_action(
            conn,
            user_id,
            ActionType::Login,
            EntityType::User,
            Some(user_id),
            None,
            None,
            None,
            description,
            request,
        )
    }

    /// Log a logout action
    pub fn log_logout(
        conn: &mut PgConnection,
        user_id: i32,
        description: Option<String>,
        request: Option<&Request<'_>>,
    ) -> Result<(), diesel::result::Error> {
        Self::log_action(
            conn,
            user_id,
            ActionType::Logout,
            EntityType::User,
            Some(user_id),
            None,
            None,
            None,
            description,
            request,
        )
    }

    /// Log an export action
    pub fn log_export(
        conn: &mut PgConnection,
        user_id: i32,
        entity_type: EntityType,
        project_id: Option<i32>,
        description: Option<String>,
        request: Option<&Request<'_>>,
    ) -> Result<(), diesel::result::Error> {
        Self::log_action(
            conn,
            user_id,
            ActionType::Export,
            entity_type,
            None,
            project_id,
            None,
            None,
            description,
            request,
        )
    }

    /// Log an import action
    pub fn log_import(
        conn: &mut PgConnection,
        user_id: i32,
        entity_type: EntityType,
        project_id: Option<i32>,
        description: Option<String>,
        request: Option<&Request<'_>>,
    ) -> Result<(), diesel::result::Error> {
        Self::log_action(
            conn,
            user_id,
            ActionType::Import,
            entity_type,
            None,
            project_id,
            None,
            None,
            description,
            request,
        )
    }

    /// Get logs for a specific entity
    pub fn get_logs_for_entity(
        conn: &mut PgConnection,
        entity_type: &str,
        entity_id: i32,
        limit: Option<i64>,
    ) -> Result<Vec<Log>, diesel::result::Error> {
        let mut query = logs::table
            .filter(logs::entity_type.eq(entity_type))
            .filter(logs::entity_id.eq(entity_id))
            .order(logs::created_at.desc())
            .into_boxed();

        if let Some(limit_val) = limit {
            query = query.limit(limit_val);
        }

        query.load::<Log>(conn)
    }

    /// Get logs for a specific project
    pub fn get_logs_for_project(
        conn: &mut PgConnection,
        project_id: i32,
        limit: Option<i64>,
    ) -> Result<Vec<Log>, diesel::result::Error> {
        let mut query = logs::table
            .filter(logs::project_id.eq(project_id))
            .order(logs::created_at.desc())
            .into_boxed();

        if let Some(limit_val) = limit {
            query = query.limit(limit_val);
        }

        query.load::<Log>(conn)
    }

    /// Get logs for a specific user
    pub fn get_logs_for_user(
        conn: &mut PgConnection,
        user_id: i32,
        limit: Option<i64>,
    ) -> Result<Vec<Log>, diesel::result::Error> {
        let mut query = logs::table
            .filter(logs::user_id.eq(user_id))
            .order(logs::created_at.desc())
            .into_boxed();

        if let Some(limit_val) = limit {
            query = query.limit(limit_val);
        }

        query.load::<Log>(conn)
    }

    /// Get recent logs with optional filtering
    pub fn get_recent_logs(
        conn: &mut PgConnection,
        limit: Option<i64>,
        action_type: Option<&str>,
        entity_type: Option<&str>,
    ) -> Result<Vec<Log>, diesel::result::Error> {
        let mut query = logs::table.order(logs::created_at.desc()).into_boxed();

        if let Some(limit_val) = limit {
            query = query.limit(limit_val);
        }

        if let Some(action) = action_type {
            query = query.filter(logs::action_type.eq(action));
        }

        if let Some(entity) = entity_type {
            query = query.filter(logs::entity_type.eq(entity));
        }

        query.load::<Log>(conn)
    }

    /// Convert a struct to JSON Value for logging
    pub fn to_json_value<T: serde::Serialize>(value: &T) -> Result<Value, serde_json::Error> {
        serde_json::to_value(value)
    }

    /// Create a description for common actions
    pub fn create_description(action_type: ActionType, entity_type: EntityType, entity_id: Option<i32>) -> String {
        match action_type {
            ActionType::Create => format!("Created new {}", entity_type.to_string().to_lowercase()),
            ActionType::Update => format!("Updated {}", entity_type.to_string().to_lowercase()),
            ActionType::Delete => format!("Deleted {}", entity_type.to_string().to_lowercase()),
            ActionType::Login => "User logged in".to_string(),
            ActionType::Logout => "User logged out".to_string(),
            ActionType::Export => format!("Exported {}", entity_type.to_string().to_lowercase()),
            ActionType::Import => format!("Imported {}", entity_type.to_string().to_lowercase()),
            ActionType::StatusChange => format!("Changed status of {}", entity_type.to_string().to_lowercase()),
        }
    }
} 
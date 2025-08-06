use diesel::prelude::*;
use diesel::pg::PgConnection;
use crate::schema::logs;
use crate::models::{Log, NewLog, ActionType, EntityType};
use chrono::{Utc, Duration, DateTime};


pub struct Logger;

impl Logger {
    pub fn log_action(
        conn: &mut PgConnection,
        user_id: i32,
        action_type: ActionType,
        entity_type: EntityType,
        entity_id: Option<i32>,
        project_id: Option<i32>,
        old_values: Option<String>,
        new_values: Option<String>,
        description: Option<String>,
        request: Option<&rocket::Request<'_>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ip_address = request.and_then(|req| {
            req.remote().map(|addr| addr.ip().to_string())
        });
        
        let user_agent = request.and_then(|req| {
            req.headers().get_one("User-Agent").map(|s| s.to_string())
        });

        let new_log = NewLog {
            user_id,
            action_type: action_type.to_string(),
            entity_type: entity_type.to_string(),
            entity_id,
            project_id,
            old_values,
            new_values,
            description,
            ip_address,
            user_agent,
        };

        diesel::insert_into(logs::table)
            .values(&new_log)
            .execute(conn)?;

        Ok(())
    }

    pub fn log_create(
        conn: &mut PgConnection,
        user_id: i32,
        entity_type: EntityType,
        entity_id: i32,
        project_id: Option<i32>,
        new_values: Option<String>,
        description: Option<String>,
        request: Option<&rocket::Request<'_>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::log_action(
            conn,
            user_id,
            ActionType::Create,
            entity_type,
            Some(entity_id),
            project_id,
            None,
            new_values,
            description,
            request,
        )
    }

    pub fn log_update(
        conn: &mut PgConnection,
        user_id: i32,
        entity_type: EntityType,
        entity_id: i32,
        project_id: Option<i32>,
        old_values: Option<String>,
        new_values: Option<String>,
        description: Option<String>,
        request: Option<&rocket::Request<'_>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::log_action(
            conn,
            user_id,
            ActionType::Update,
            entity_type,
            Some(entity_id),
            project_id,
            old_values,
            new_values,
            description,
            request,
        )
    }

    pub fn log_delete(
        conn: &mut PgConnection,
        user_id: i32,
        entity_type: EntityType,
        entity_id: i32,
        project_id: Option<i32>,
        old_values: Option<String>,
        description: Option<String>,
        request: Option<&rocket::Request<'_>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::log_action(
            conn,
            user_id,
            ActionType::Delete,
            entity_type,
            Some(entity_id),
            project_id,
            old_values,
            None,
            description,
            request,
        )
    }

    pub fn log_login(
        conn: &mut PgConnection,
        user_id: i32,
        request: Option<&rocket::Request<'_>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::log_action(
            conn,
            user_id,
            ActionType::Login,
            EntityType::User,
            Some(user_id),
            None,
            None,
            None,
            Some("User logged in".to_string()),
            request,
        )
    }

    pub fn log_logout(
        conn: &mut PgConnection,
        user_id: i32,
        request: Option<&rocket::Request<'_>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::log_action(
            conn,
            user_id,
            ActionType::Logout,
            EntityType::User,
            Some(user_id),
            None,
            None,
            None,
            Some("User logged out".to_string()),
            request,
        )
    }

    pub fn log_export(
        conn: &mut PgConnection,
        user_id: i32,
        entity_type: EntityType,
        entity_id: Option<i32>,
        project_id: Option<i32>,
        description: Option<String>,
        request: Option<&rocket::Request<'_>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::log_action(
            conn,
            user_id,
            ActionType::Export,
            entity_type,
            entity_id,
            project_id,
            None,
            None,
            description,
            request,
        )
    }

    pub fn log_import(
        conn: &mut PgConnection,
        user_id: i32,
        entity_type: EntityType,
        project_id: Option<i32>,
        description: Option<String>,
        request: Option<&rocket::Request<'_>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
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

    pub fn get_logs_for_entity(
        conn: &mut PgConnection,
        entity_type_param: &str,
        entity_id_param: i32,
    ) -> Result<Vec<Log>, Box<dyn std::error::Error>> {
        use crate::schema::logs::dsl::*;

        let logs_list = logs
            .filter(entity_type.eq(entity_type_param))
            .filter(entity_id.eq(entity_id_param))
            .order(created_at.desc())
            .load::<Log>(conn)?;

        Ok(logs_list)
    }

    pub fn get_logs_for_project(
        conn: &mut PgConnection,
        project_id_param: i32,
    ) -> Result<Vec<Log>, Box<dyn std::error::Error>> {
        use crate::schema::logs::dsl::*;

        let logs_list = logs
            .filter(project_id.eq(project_id_param))
            .order(created_at.desc())
            .load::<Log>(conn)?;

        Ok(logs_list)
    }

    pub fn get_logs_for_user(
        conn: &mut PgConnection,
        user_id_param: i32,
    ) -> Result<Vec<Log>, Box<dyn std::error::Error>> {
        use crate::schema::logs::dsl::*;

        let logs_list = logs
            .filter(user_id.eq(user_id_param))
            .order(created_at.desc())
            .load::<Log>(conn)?;

        return Ok(logs_list);
    }

    pub fn get_recent_logs(
        conn: &mut PgConnection,
        limit: i64,
    ) -> Result<Vec<Log>, Box<dyn std::error::Error>> {
        use crate::schema::logs::dsl::*;

        let logs_list = logs
            .order(created_at.desc())
            .limit(limit)
            .load::<Log>(conn)?;

        Ok(logs_list)
    }

    pub fn to_json_string<T: serde::Serialize>(value: &T) -> Result<String, Box<dyn std::error::Error>> {
        Ok(serde_json::to_string_pretty(value)?)
    }

    pub fn get_log_count(conn: &mut PgConnection, days: i64) -> Result<i64, Box<dyn std::error::Error>> {
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
            (ActionType::Create, EntityType::Applicability) => "Created new applicability".to_string(),
            (ActionType::Update, EntityType::Applicability) => "Updated applicability".to_string(),
            (ActionType::Delete, EntityType::Applicability) => "Deleted applicability".to_string(),
            _ => format!("{:?} {:?}", action_type, entity_type),
        }
    }
}

pub fn cleanup_old_logs(conn: &mut PgConnection, days: i64) -> Result<usize, Box<dyn std::error::Error>> {
    use crate::schema::logs::dsl::*;
    
    let cutoff_timestamp = Utc::now() - Duration::days(days);
    let cutoff_datetime = DateTime::from_timestamp(cutoff_timestamp.timestamp(), 0)
        .ok_or("Invalid timestamp")?
        .naive_utc();

    let deleted_count = diesel::delete(logs.filter(created_at.lt(cutoff_datetime)))
        .execute(conn)?;

    Ok(deleted_count)
}

pub fn get_log_count(conn: &mut PgConnection, days: Option<i64>) -> Result<i64, Box<dyn std::error::Error>> {
    use crate::schema::logs::dsl::*;
    
    let query = if let Some(days) = days {
        let cutoff_timestamp = Utc::now() - Duration::days(days);
        let cutoff_datetime = DateTime::from_timestamp(cutoff_timestamp.timestamp(), 0)
            .ok_or("Invalid timestamp")?
            .naive_utc();
        
        logs.filter(created_at.ge(cutoff_datetime)).into_boxed()
    } else {
        logs.into_boxed()
    };

    let count = query.count().get_result(conn)?;
    Ok(count)
} 
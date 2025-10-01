use super::helpers::*;
use super::prelude::*;

#[get("/logs")]
pub fn show_logs(admin: AdminOnly, state: &State<AppState>) -> Result<Template, Redirect> {
    let user = admin.into_inner();

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error in show_logs: {}", e);
        Redirect::to(uri!(crate::routes::html::admin::admin_dashboard))
    })?;
    let logs = Logger::get_recent_logs(connection, 1000).unwrap_or_default();

    // Enhance logs with user information
    let mut enhanced_logs = Vec::new();
    for log in logs {
        let username = state
            .repo_read()
            .get_user_by_id(log.user_id)
            .expect("Error reading table Users")
            .user_username;
        let mut log_json = serde_json::to_value(log).unwrap_or_default();
        if let Some(log_obj) = log_json.as_object_mut() {
            log_obj.insert("username".to_string(), serde_json::Value::String(username));
        }
        enhanced_logs.push(log_json);
    }

    let ctx = json!({
        "user": user,
        "logs": enhanced_logs,
        "title": "System Logs"
    });

    Ok(Template::render("logs", ctx))
}

#[get("/logs/<entity_type>/<entity_id>")]
pub fn show_entity_logs(
    admin: AdminOnly,
    entity_type: String,
    entity_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = admin.into_inner();

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error in show_entity_logs: {}", e);
        Redirect::to(uri!(show_logs))
    })?;
    let logs = Logger::get_logs_for_entity(connection, &entity_type, entity_id).unwrap_or_default();

    // Enhance logs with user information
    let mut enhanced_logs = Vec::new();
    for log in logs {
        let username = state
            .repo_read()
            .get_user_by_id(log.user_id)
            .expect("Error reading table Users")
            .user_username;
        let mut log_json = serde_json::to_value(log).unwrap_or_default();
        if let Some(log_obj) = log_json.as_object_mut() {
            log_obj.insert("username".to_string(), serde_json::Value::String(username));
        }
        enhanced_logs.push(log_json);
    }

    let ctx = json!({
        "user": user,
        "logs": enhanced_logs,
        "entity_type": entity_type,
        "entity_id": entity_id,
        "title": format!("Logs for {} {}", entity_type, entity_id)
    });

    Ok(Template::render("entity_logs", ctx))
}

#[get("/export_logs?<filename>")]
pub async fn export_logs(
    admin: AdminOnly,
    filename: Option<String>,
    state: &State<AppState>,
) -> Result<(ContentType, NamedFile), Redirect> {
    let user = admin.into_inner();

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error in export_logs: {}", e);
        Redirect::to(uri!(show_logs))
    })?;
    let logs = Logger::get_recent_logs(connection, 1000).unwrap_or_default();

    // Convert logs to JSON
    let logs_json = serde_json::to_string_pretty(&logs).unwrap_or_default();

    // Generate filename if not provided
    let filename = filename.unwrap_or_else(|| {
        let now = chrono::Utc::now();
        format!("reqman-logs_{}.json", now.format("%Y%m%d_%H%M%S"))
    });

    // Ensure filename has .json extension
    let filename = if filename.ends_with(".json") {
        filename
    } else {
        format!("{}.json", filename)
    };

    // Create exports directory if it doesn't exist
    let export_dir = "exports";
    if !std::path::Path::new(export_dir).exists() {
        std::fs::create_dir(export_dir).map_err(|_| Redirect::to(uri!(show_logs)))?;
    }

    let export_path = format!("{}/{}", export_dir, filename);

    // Write JSON to file
    std::fs::write(&export_path, logs_json).map_err(|_| Redirect::to(uri!(show_logs)))?;

    // Log the successful export
    let log_ctx = LogCtx::new(user.user_id);
    let _ = Logger::log_export(
        connection,
        &log_ctx,
        Some(format!("Exported logs to {}", filename)),
    );

    Ok((
        ContentType::JSON,
        NamedFile::open(export_path)
            .await
            .map_err(|_| Redirect::to(uri!(show_logs)))?,
    ))
}

#[get("/export_logs/<entity_type>/<entity_id>")]
pub fn export_entity_logs(
    _admin: AdminOnly,
    entity_type: String,
    entity_id: i32,
    state: &State<AppState>,
) -> Result<(rocket::http::ContentType, String), Redirect> {
    let mut connection = match get_db_connection(state) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(show_logs)));
        }
    };
    let logs = Logger::get_logs_for_entity(connection.as_mut(), &entity_type, entity_id)
        .unwrap_or_default();

    // Convert logs to JSON
    let logs_json = serde_json::to_string_pretty(&logs).unwrap_or_default();

    let content_type = rocket::http::ContentType::new("application", "json");
    Ok((content_type, logs_json))
}

#[post("/cleanup_logs")]
pub fn cleanup_logs(admin: AdminOnly, state: &State<AppState>) -> Result<Redirect, Redirect> {
    let user = admin.into_inner();

    let mut connection = match get_db_connection(state) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(show_logs)));
        }
    };

    // Clean up logs older than 90 days
    match Logger::cleanup_old_logs(connection.as_mut(), 90) {
        Ok(deleted_count) => {
            // Log the cleanup action
            let log_ctx = LogCtx::new(user.user_id);
            let _ = Logger::log_custom(
                connection.as_mut(),
                &log_ctx,
                crate::models::ActionType::StatusChange,
                crate::models::EntityType::User,
                None,
                None,
                None,
                None,
                Some(format!("Cleaned up {} old log entries", deleted_count)),
            );
        }
        Err(_) => {
            // Log the failed cleanup action
            let log_ctx = LogCtx::new(user.user_id);
            let _ = Logger::log_custom(
                connection.as_mut(),
                &log_ctx,
                crate::models::ActionType::StatusChange,
                crate::models::EntityType::User,
                None,
                None,
                None,
                None,
                Some("Failed to clean up old log entries".to_string()),
            );
        }
    }

    Ok(Redirect::to(uri!(show_logs)))
}

#[get("/log_analytics")]
pub fn log_analytics(admin: AdminOnly, state: &State<AppState>) -> Result<Template, Redirect> {
    let user = admin.into_inner();

    let mut connection = match get_db_connection(state) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(show_logs)));
        }
    };

    // Get basic statistics
    let last_7_days = Logger::get_log_count(connection.as_mut(), 7).unwrap_or(0);
    let last_30_days = Logger::get_log_count(connection.as_mut(), 30).unwrap_or(0);
    let last_90_days = Logger::get_log_count(connection.as_mut(), 90).unwrap_or(0);

    let ctx = json!({
        "user": user,
        "last_7_days": last_7_days,
        "last_30_days": last_30_days,
        "last_90_days": last_90_days,
        "title": "Log Analytics"
    });

    Ok(Template::render("log_analytics", ctx))
}

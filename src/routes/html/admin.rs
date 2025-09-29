use super::helpers::*;
use super::prelude::*;

#[get("/admin")]
pub fn admin_dashboard(admin: AdminOnly) -> Template {
    let user = admin.into_inner();

    let context = json!({
        "user": user,
        "title": "Admin Dashboard"
    });

    Template::render("admin/dashboard", context)
}

#[get("/admin/users")]
pub fn admin_users_page(admin: AdminOnly, state: &State<AppState>) -> Template {
    let user = admin.into_inner();

    let users = state.repo_read().get_users_all().unwrap_or_default();

    let context = json!({
        "user": user,
        "users": users,
        "title": "User Management"
    });

    Template::render("admin/users", context)
}

#[get("/admin/backup")]
pub fn admin_backup_page(admin: AdminOnly) -> Template {
    let user = admin.into_inner();

    let context = json!({
        "user": user,
        "title": "Database Backup"
    });

    Template::render("admin/backup", context)
}

#[post("/admin/backup/generate/<filename>")]
pub async fn generate_backup(
    admin: AdminOnly,
    filename: String,
    state: &State<AppState>,
) -> Result<(ContentType, NamedFile), Redirect> {
    let user = admin.into_inner();

    // Use the filename from the URL parameter
    let filename = if filename.ends_with(".sql") {
        filename
    } else {
        format!("{}.sql", filename)
    };

    // Create backup directory if it doesn't exist
    let backup_dir = "backups";
    if !std::path::Path::new(backup_dir).exists() {
        std::fs::create_dir(backup_dir).map_err(|_| Redirect::to(uri!(admin_backup_page)))?;
    }

    let backup_path = format!("{}/{}", backup_dir, filename);

    // Database configuration from Rocket.toml
    let _db_url = "postgres://rust:rust@127.0.0.1:5432/reqman";
    let password = "rust";
    let host = "127.0.0.1";
    let port = "5432";
    let username = "rust";
    let database = "reqman";

    // Set environment variable for password
    std::env::set_var("PGPASSWORD", password);

    // Execute pg_dump command with explicit table inclusion to ensure logs are included
    let output = std::process::Command::new("pg_dump")
        .args(&[
            "-h",
            host,
            "-p",
            port,
            "-U",
            username,
            "-d",
            database,
            "-f",
            &backup_path,
            "--no-password",
            "--verbose",       // Add verbose output for debugging
            "--no-owner",      // Don't include ownership information
            "--no-privileges", // Don't include privilege information
        ])
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                // Log the successful backup
                if let Ok(mut conn) = get_db_connection(state) {
                    let log_ctx = LogCtx::new(user.user_id);
                    let _ = Logger::log_custom(
                        &mut conn,
                        &log_ctx,
                        crate::models::ActionType::StatusChange,
                        crate::models::EntityType::User,
                        None,
                        None,
                        None,
                        None,
                        Some(format!("Database backup generated: {}", filename)),
                    );
                }

                // Return the backup file for download
                let file = NamedFile::open(&backup_path)
                    .await
                    .map_err(|_| Redirect::to(uri!(admin_backup_page)))?;

                let content_type = ContentType::new("application", "sql");
                Ok((content_type, file))
            } else {
                // Log the failed backup
                if let Ok(mut conn) = get_db_connection(state) {
                    let log_ctx = LogCtx::new(user.user_id);
                    let _ = Logger::log_custom(
                        &mut conn,
                        &log_ctx,
                        crate::models::ActionType::StatusChange,
                        crate::models::EntityType::User,
                        None,
                        None,
                        None,
                        None,
                        Some(format!(
                            "Database backup failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        )),
                    );
                }

                // If backup failed, redirect to backup page with error
                Err(Redirect::to(uri!(admin_backup_page)))
            }
        }
        Err(e) => {
            // Log the command failure
            if let Ok(mut conn) = get_db_connection(state) {
                let log_ctx = LogCtx::new(user.user_id);
                let _ = Logger::log_custom(
                    &mut conn,
                    &log_ctx,
                    crate::models::ActionType::StatusChange,
                    crate::models::EntityType::User,
                    None,
                    None,
                    None,
                    None,
                    Some(format!("Database backup command failed: {}", e)),
                );
            }

            // If command failed, redirect to backup page with error
            Err(Redirect::to(uri!(admin_backup_page)))
        }
    }
}

use super::helpers::get_db_connection;
use super::prelude::*;
use std::path::{Path, PathBuf};

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
    const HOST: &str = "127.0.0.1";
    const PORT: &str = "5432";
    const USER: &str = "rust";
    const PASSWORD: &str = "rust";
    const DB: &str = "reqman";
    const DIR: &str = "backups";

    let user_id = admin.into_inner().user_id;
    let filename = ensure_sql_extension(&filename);
    let backup_path = Path::new(DIR).join(&filename);

    ensure_dir(DIR).map_err(|_| admin_redirect())?;
    std::env::set_var("PGPASSWORD", PASSWORD);

    match run_pg_dump(HOST, PORT, USER, DB, &backup_path) {
        Err(e) => fail(
            state,
            user_id,
            format!("Database backup command failed: {e}"),
        ),
        // pg_dump started, but returned error
        Ok(output) if !output.status.success() => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            fail(
                state,
                user_id,
                format!("Database backup failed: {}", stderr.trim()),
            )
        }
        Ok(_) => {
            if let Ok(mut conn) = get_db_connection(state.inner()) {
                let ctx = LogCtx::new(user_id);
                let _ = Logger::log_export(
                    conn.as_mut(),
                    &ctx,
                    Some(format!("Database backup generated: {filename}")),
                );
            }

            let file = NamedFile::open(&backup_path)
                .await
                .map_err(|_| admin_redirect())?;
            Ok((ContentType::new("application", "sql"), file))
        }
    }
}

fn ensure_sql_extension(name: &str) -> String {
    if name.ends_with(".sql") {
        name.to_owned()
    } else {
        format!("{name}.sql")
    }
}

fn ensure_dir(dir: &str) -> std::io::Result<()> {
    if !Path::new(dir).exists() {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}

fn run_pg_dump(
    host: &str,
    port: &str,
    user: &str,
    database: &str,
    out_path: &PathBuf,
) -> std::io::Result<std::process::Output> {
    std::process::Command::new("pg_dump")
        .arg("-h")
        .arg(host)
        .arg("-p")
        .arg(port)
        .arg("-U")
        .arg(user)
        .arg("-d")
        .arg(database)
        .arg("-f")
        .arg(out_path)
        .arg("--no-password")
        .arg("--verbose")
        .arg("--no-owner")
        .arg("--no-privileges")
        .output()
}

fn admin_redirect() -> Redirect {
    Redirect::to(uri!(admin_backup_page))
}

fn fail<T>(state: &State<AppState>, user_id: i32, msg: impl Into<String>) -> Result<T, Redirect> {
    if let Ok(mut conn) = get_db_connection(state.inner()) {
        let ctx = LogCtx::new(user_id);
        let _ = Logger::log_export(conn.as_mut(), &ctx, Some(msg.into()));
    }
    Err(admin_redirect())
}

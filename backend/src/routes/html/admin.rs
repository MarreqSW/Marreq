// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::helpers::get_db_connection;
use super::prelude::*;
use std::path::{Path, PathBuf};

#[get("/admin")]
pub async fn admin_dashboard(admin: AdminOnly) -> Template {
    let user = admin.into_inner();

    let context = json!({
        "user": user,
        "page_title": "Admin Dashboard"
    });

    Template::render("admin/dashboard", context)
}

#[get("/admin/users")]
pub async fn admin_users_page(admin: AdminOnly, state: &State<AppState>) -> Template {
    let user = admin.into_inner();

    let users = state.repo_read().get_users_all().unwrap_or_default();

    let context = json!({
        "user": user,
        "users": users,
        "page_title": "User Management"
    });

    Template::render("admin/users", context)
}

#[get("/admin/backup")]
pub async fn admin_backup_page(admin: AdminOnly) -> Template {
    let user = admin.into_inner();

    let context = json!({
        "user": user,
        "page_title": "Database Backup"
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
    const DB: &str = "marreq";
    const DIR: &str = "backups";

    let id = admin.into_inner().id;
    let filename = ensure_sql_extension(&filename);
    let backup_path = Path::new(DIR).join(&filename);

    ensure_dir(DIR).map_err(|_| admin_redirect())?;
    std::env::set_var("PGPASSWORD", PASSWORD);

    let mut conn = get_db_connection(state.inner()).ok();
    if conn.is_none() {
        // If we can't even get a connection for logging, fail
        return Err(admin_redirect());
    }
    let ctx = LogCtx::new(id);

    // tiny helper to avoid repeating the logging boilerplate
    let mut log = |msg: String| {
        if let Some(c) = conn.as_mut() {
            let _ = Logger::log_export(c, &ctx, Some(msg));
        } else {
            eprintln!("Could not reach Database! Message: {}", msg);
        }
    };

    let output = match run_pg_dump(HOST, PORT, USER, DB, &backup_path) {
        Ok(o) => o,
        Err(e) => {
            log(format!("Database backup command failed: {e}"));
            return Err(admin_redirect());
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log(format!("Database backup failed: {}", stderr.trim()));
        return Err(admin_redirect());
    }

    log(format!("Database backup generated: {filename}"));

    let file = NamedFile::open(&backup_path)
        .await
        .map_err(|_| admin_redirect())?;

    Ok((ContentType::new("application", "sql"), file))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::repository::CacheRepository;
    use rocket::http::{Cookie, SameSite, Status};
    use rocket::local::asynchronous::Client;
    use rocket_dyn_templates::Template;
    use std::sync::{Arc, RwLock};

    const ADMIN_ID: i32 = 1;

    fn make_user(id: i32, username: &str, is_admin: bool) -> crate::models::User {
        let mut user = DieselRepoMock::make_user(id, username, "");
        user.is_admin = is_admin;
        user.name = format!("User {id}");
        user.email = format!("{username}@example.com");
        user
    }

    fn test_state() -> AppState<CacheRepository<DieselRepoMock>> {
        let users = vec![
            make_user(ADMIN_ID, "admin", true),
            make_user(2, "helper", false),
        ];

        let inner = DieselRepoMock::with_users(users);
        let repo = CacheRepository::new(inner, 60);

        AppState {
            repo: Arc::new(RwLock::new(repo)),
        }
    }

    async fn test_client() -> Client {
        let rocket = rocket::build()
            .manage(test_state())
            .attach(Template::fairing())
            .mount(
                "/",
                routes![
                    admin_dashboard,
                    admin_users_page,
                    admin_backup_page,
                    generate_backup
                ],
            );
        Client::tracked(rocket).await.expect("client")
    }

    fn admin_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(SameSite::Strict);
        cookie
    }

    #[rocket::async_test]
    async fn admin_dashboard_renders_dashboard_template() {
        let client = test_client().await;
        let response = client
            .get("/admin")
            .private_cookie(admin_cookie())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Admin Dashboard"));
        assert!(body.contains("Manage Users"));
    }

    #[rocket::async_test]
    async fn admin_users_page_lists_known_users() {
        let client = test_client().await;
        let response = client
            .get("/admin/users")
            .private_cookie(admin_cookie())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("User Management"));
        assert!(body.contains("helper"));
    }

    #[rocket::async_test]
    async fn admin_backup_page_renders_backup_template() {
        let client = test_client().await;
        let response = client
            .get("/admin/backup")
            .private_cookie(admin_cookie())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Database Backup"));
    }

    #[rocket::async_test]
    async fn generate_backup_redirects_on_failure_and_sets_env() {
        let _ = std::fs::remove_dir_all("backups");

        let client = test_client().await;
        let response = client
            .post("/admin/backup/generate/nightly")
            .private_cookie(admin_cookie())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/admin/backup")
        );
        assert_eq!(std::env::var("PGPASSWORD").ok().as_deref(), Some("rust"));
        assert!(std::path::Path::new("backups").exists());

        let _ = std::fs::remove_dir_all("backups");
        std::env::remove_var("PGPASSWORD");
    }

    #[test]
    fn ensure_sql_extension_adds_suffix_when_missing() {
        let name = "backup";
        let result = super::ensure_sql_extension(name);
        assert_eq!(result, "backup.sql");
    }

    #[test]
    fn ensure_sql_extension_keeps_existing_suffix() {
        let name = "archive.sql";
        let result = super::ensure_sql_extension(name);
        assert_eq!(result, "archive.sql");
    }

    #[test]
    fn ensure_dir_creates_directory_and_is_idempotent() {
        let unique = format!(
            "marreq-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );

        let path = std::env::temp_dir().join(unique);
        let dir_str = path.to_string_lossy().to_string();

        let _ = std::fs::remove_dir_all(&path);

        super::ensure_dir(&dir_str).expect("first creation should succeed");
        assert!(path.exists() && path.is_dir());

        super::ensure_dir(&dir_str).expect("second creation should succeed");

        let _ = std::fs::remove_dir_all(&path);
    }
}

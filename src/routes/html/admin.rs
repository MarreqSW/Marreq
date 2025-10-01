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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::repository::CacheRepository;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use rocket::http::{Cookie, Status};
    use rocket::local::asynchronous::Client;
    use rocket_dyn_templates::Template;
    use std::sync::{Arc, RwLock};

    const ADMIN_ID: i32 = 1;

    fn make_user(id: i32, username: &str, is_admin: bool) -> crate::models::User {
        let mut user = DieselRepoMock::make_user(id, username, "");
        user.is_admin = is_admin;
        user.user_name = format!("User {id}");
        user.user_email = format!("{username}@example.com");
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
                routes![admin_dashboard, admin_users_page, admin_backup_page],
            );
        Client::tracked(rocket).await.expect("client")
    }

    fn admin_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
        cookie.set_path("/");
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
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! `POST /api/admin/backup` — whole-database backup download for site administrators (issue #247).

use crate::api::prelude::*;
use crate::backup::{create_backup, remove_quietly, BackupConfig};
use crate::services::log_service::LogService;
use rocket::fs::NamedFile;
use rocket::http::{ContentType, Header};

/// Gzipped SQL dump streamed from disk as an attachment.
#[derive(Responder)]
#[response(status = 200)]
pub struct BackupDownload {
    file: NamedFile,
    content_type: ContentType,
    disposition: Header<'static>,
}

/// `POST /api/admin/backup` — run `pg_dump` and download the result as `.sql.gz`.
///
/// Administrators only; POST so the CSRF fairing applies. Disabled (410) in
/// deployment modes that do not allow whole-database backups. Every attempt is
/// recorded as an `EXPORT` audit entry.
#[post("/admin/backup")]
pub async fn create(
    admin: AdminOnly,
    state: &State<AppState>,
    config: BackupConfig,
) -> ApiResult<BackupDownload> {
    if !crate::deployment::current().allows_database_backup() {
        return Err(ApiError::Gone(
            "Database backup is not available in this deployment mode".into(),
        ));
    }
    let database_url = config
        .database_url
        .ok_or_else(|| ApiError::Internal("DATABASE_URL is not configured".into()))?;

    let audit = LogService::new(state.inner());
    let backup = match create_backup(&database_url, &config.pg_dump).await {
        Ok(backup) => backup,
        Err(err) => {
            let _ =
                audit.log_export_action(admin.id, Some(format!("Database backup failed: {err}")));
            return Err(ApiError::Internal(err.to_string()));
        }
    };

    // Unlink right after opening: the open handle keeps the data readable while
    // the response streams, and nothing is left behind in the temp directory.
    let opened = NamedFile::open(&backup.path).await;
    remove_quietly(&backup.path);
    let file = opened.map_err(|e| ApiError::Internal(format!("could not read backup: {e}")))?;
    let size = file.file().metadata().await.map(|m| m.len()).unwrap_or(0);

    audit
        .log_export_action(
            admin.id,
            Some(format!(
                "Database backup generated: {} ({size} bytes)",
                backup.filename
            )),
        )
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(BackupDownload {
        file,
        content_type: ContentType::new("application", "gzip"),
        disposition: Header::new(
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", backup.filename),
        ),
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::test_session_cookie_for;
    use crate::backup::tests::fake_pg_dump;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository, LogRepository};
    use rocket::http::Cookie;
    use rocket::local::asynchronous::Client;
    use std::path::PathBuf;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;
    const NON_ADMIN_ID: i32 = 2;

    fn scratch_dir() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("marreq-backup-api-test-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    async fn client(config: BackupConfig) -> Client {
        crate::deployment::install_test_server_mode();
        let mut repo = DieselRepoMock::default().with_admin_user();
        repo.users.insert(
            NON_ADMIN_ID,
            DieselRepoMock::make_user(NON_ADMIN_ID, "bob", "hash"),
        );
        let rocket = rocket::build()
            .manage::<TestState>(AppState {
                repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
            })
            .manage(config)
            .mount("/api", routes![create]);
        Client::tracked(rocket).await.unwrap()
    }

    fn cookie(client: &Client, user_id: i32) -> Cookie<'static> {
        test_session_cookie_for(client.rocket().state::<TestState>().unwrap(), user_id)
    }

    fn audit_descriptions(client: &Client) -> Vec<String> {
        let state = client.rocket().state::<TestState>().unwrap();
        let repo = state.repo.read().unwrap();
        repo.get_logs_recent(10)
            .unwrap()
            .into_iter()
            .filter_map(|l| l.description)
            .collect()
    }

    fn config_with(pg_dump: &std::path::Path) -> BackupConfig {
        BackupConfig {
            database_url: Some("postgres://rust:rust@db:5432/marreq".into()),
            pg_dump: pg_dump.display().to_string(),
        }
    }

    #[rocket::async_test]
    async fn requires_authentication() {
        let client = client(config_with(std::path::Path::new("/nonexistent"))).await;
        let response = client.post("/api/admin/backup").dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
    }

    #[rocket::async_test]
    async fn forbidden_for_non_admin() {
        let client = client(config_with(std::path::Path::new("/nonexistent"))).await;
        let response = client
            .post("/api/admin/backup")
            .private_cookie(cookie(&client, NON_ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn streams_gzip_attachment_and_audits() {
        let dir = scratch_dir();
        let script = fake_pg_dump(
            &dir,
            r#"for a in "$@"; do case "$a" in --file=*) printf 'dump' > "${a#--file=}";; esac; done"#,
        );
        let client = client(config_with(&script)).await;

        let response = client
            .post("/api/admin/backup")
            .private_cookie(cookie(&client, ADMIN_ID))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        assert_eq!(
            response.content_type(),
            Some(ContentType::new("application", "gzip"))
        );
        let disposition = response
            .headers()
            .get_one("Content-Disposition")
            .unwrap()
            .to_string();
        assert!(disposition.starts_with("attachment; filename=\"marreq-backup_"));
        assert!(disposition.ends_with(".sql.gz\""));
        assert_eq!(response.into_bytes().await.unwrap(), b"dump");

        let logs = audit_descriptions(&client);
        assert!(logs
            .iter()
            .any(|d| d.starts_with("Database backup generated: marreq-backup_")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[rocket::async_test]
    async fn dump_failure_returns_500_and_audits() {
        let dir = scratch_dir();
        let script = fake_pg_dump(&dir, "echo 'could not connect' >&2; exit 1");
        let client = client(config_with(&script)).await;

        let response = client
            .post("/api/admin/backup")
            .private_cookie(cookie(&client, ADMIN_ID))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::InternalServerError);
        let body: serde_json::Value = response.into_json().await.unwrap();
        assert!(body["message"]
            .as_str()
            .unwrap()
            .contains("could not connect"));
        assert!(audit_descriptions(&client)
            .iter()
            .any(|d| d.starts_with("Database backup failed")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! `POST /api/admin/backup` — whole-instance backup download for site
//! administrators: the database and the attachment files (issues #247, #341).

use crate::api::attachments::Storage;
use crate::api::prelude::*;
use crate::backup::{
    AttachmentUsage, BackupConfig, attachment_usage, create_backup, remove_quietly,
};
use crate::services::log_service::LogService;
use rocket::fs::NamedFile;
use rocket::http::{ContentType, Header};
use rocket::serde::Serialize;

/// The backup `.tar.gz`, streamed from disk as an attachment.
#[derive(Responder)]
#[response(status = 200)]
pub struct BackupDownload {
    file: NamedFile,
    content_type: ContentType,
    disposition: Header<'static>,
}

fn require_backup_allowed() -> ApiResult<()> {
    if crate::deployment::current().allows_database_backup() {
        Ok(())
    } else {
        Err(ApiError::Gone(
            "Database backup is not available in this deployment mode".into(),
        ))
    }
}

/// What a backup with attachment files would contain (for the Backup page).
#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct BackupInfo {
    /// Whether attachment storage is configured (otherwise backups hold the database only).
    pub attachments_available: bool,
    pub attachment_files: u64,
    pub attachment_bytes: u64,
}

/// `GET /api/admin/backup/info` — number and size of the attachment files a
/// backup would include. Administrators only.
#[get("/admin/backup/info")]
pub async fn info(_admin: AdminOnly, storage: Option<Storage>) -> ApiResult<Json<BackupInfo>> {
    require_backup_allowed()?;
    let Some(storage) = storage else {
        return Ok(Json(BackupInfo {
            attachments_available: false,
            attachment_files: 0,
            attachment_bytes: 0,
        }));
    };
    let dir = storage.0.config.dir.clone();
    let AttachmentUsage { files, bytes } =
        rocket::tokio::task::spawn_blocking(move || attachment_usage(&dir))
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .map_err(|e| ApiError::Internal(format!("could not read attachment storage: {e}")))?;
    Ok(Json(BackupInfo {
        attachments_available: true,
        attachment_files: files,
        attachment_bytes: bytes,
    }))
}

/// `POST /api/admin/backup?attachments=<bool>` — run `pg_dump` and download a
/// `.tar.gz` with the SQL dump and, unless `attachments=false`, the attachment
/// files (left out when attachment storage is not configured).
///
/// Administrators only; POST so the CSRF fairing applies. Disabled (410) in
/// deployment modes that do not allow whole-database backups. Every attempt is
/// recorded as an `EXPORT` audit entry.
#[post("/admin/backup?<attachments>")]
pub async fn create(
    admin: AdminOnly,
    state: &State<AppState>,
    config: BackupConfig,
    storage: Option<Storage>,
    attachments: Option<bool>,
) -> ApiResult<BackupDownload> {
    require_backup_allowed()?;
    let database_url = config
        .database_url
        .ok_or_else(|| ApiError::Internal("DATABASE_URL is not configured".into()))?;

    let audit = LogService::new(state.inner());
    let attachments_dir = attachments
        .unwrap_or(true)
        .then(|| storage.map(|s| s.0.config.dir.clone()))
        .flatten();
    let backup = match create_backup(&database_url, &config.pg_dump, attachments_dir).await {
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
            Some(if backup.includes_attachments {
                format!(
                    "Database backup generated: {} ({size} bytes, {} attachment files)",
                    backup.filename, backup.attachments.files
                )
            } else {
                format!(
                    "Database backup generated: {} ({size} bytes, database only)",
                    backup.filename
                )
            }),
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
    use crate::backup::tests::{archive_entries, fake_pg_dump};
    use crate::repository::{CacheRepository, LogRepository, diesel_repo_mock::DieselRepoMock};
    use crate::storage::{AttachmentStorage, AttachmentsConfig};
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
        client_with_storage(config, None).await
    }

    async fn client_with_storage(config: BackupConfig, attachments: Option<PathBuf>) -> Client {
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
            .mount("/api", routes![create, info]);
        let rocket = match attachments {
            Some(dir) => rocket.manage(Arc::new(AttachmentStorage::local(AttachmentsConfig {
                dir,
                ..AttachmentsConfig::default()
            }))),
            None => rocket,
        };
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

    const DUMP_SCRIPT: &str =
        r#"for a in "$@"; do case "$a" in --file=*) printf 'dump' > "${a#--file=}";; esac; done"#;

    /// An attachment store holding one 5-byte file.
    fn store(dir: &std::path::Path) -> PathBuf {
        let store = dir.join("attachments");
        std::fs::create_dir_all(store.join("ab/cd")).unwrap();
        std::fs::write(store.join("ab/cd/abcd1"), b"bytes").unwrap();
        store
    }

    /// POST the backup, check the download headers and return its entries.
    async fn download(client: &Client, uri: &str) -> Vec<(String, Vec<u8>)> {
        let response = client
            .post(uri.to_string())
            .private_cookie(cookie(client, ADMIN_ID))
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
        assert!(disposition.ends_with(".tar.gz\""));
        let bytes = response.into_bytes().await.unwrap();
        let path = scratch_dir().join("download.tar.gz");
        std::fs::write(&path, bytes).unwrap();
        let entries = archive_entries(&path);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
        entries
    }

    #[rocket::async_test]
    async fn streams_archive_with_attachments_and_audits() {
        let dir = scratch_dir();
        let script = fake_pg_dump(&dir, DUMP_SCRIPT);
        let client = client_with_storage(config_with(&script), Some(store(&dir))).await;

        let entries = download(&client, "/api/admin/backup").await;
        assert!(entries[0].0.ends_with("/database.sql"));
        assert_eq!(entries[0].1, b"dump");
        assert!(entries[1].0.ends_with("/attachments/ab/cd/abcd1"));
        assert_eq!(entries[1].1, b"bytes");

        let logs = audit_descriptions(&client);
        assert!(logs.iter().any(
            |d| d.starts_with("Database backup generated: marreq-backup_")
                && d.ends_with("1 attachment files)")
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[rocket::async_test]
    async fn attachments_can_be_left_out() {
        let dir = scratch_dir();
        let script = fake_pg_dump(&dir, DUMP_SCRIPT);
        let client = client_with_storage(config_with(&script), Some(store(&dir))).await;

        let entries = download(&client, "/api/admin/backup?attachments=false").await;
        assert_eq!(entries.len(), 2, "database.sql and manifest.json only");
        assert!(
            audit_descriptions(&client)
                .iter()
                .any(|d| d.ends_with("database only)"))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[rocket::async_test]
    async fn without_attachment_storage_the_backup_is_database_only() {
        let dir = scratch_dir();
        let script = fake_pg_dump(&dir, DUMP_SCRIPT);
        let client = client(config_with(&script)).await;
        let entries = download(&client, "/api/admin/backup").await;
        assert_eq!(entries.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[rocket::async_test]
    async fn info_reports_attachment_size_to_admins_only() {
        let dir = scratch_dir();
        let client = client_with_storage(
            config_with(std::path::Path::new("/nonexistent")),
            Some(store(&dir)),
        )
        .await;

        let denied = client
            .get("/api/admin/backup/info")
            .private_cookie(cookie(&client, NON_ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(denied.status(), Status::Forbidden);

        let response = client
            .get("/api/admin/backup/info")
            .private_cookie(cookie(&client, ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: serde_json::Value = response.into_json().await.unwrap();
        assert_eq!(
            body,
            serde_json::json!({
                "attachments_available": true,
                "attachment_files": 1,
                "attachment_bytes": 5,
            })
        );
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
        assert!(
            body["message"]
                .as_str()
                .unwrap()
                .contains("could not connect")
        );
        assert!(
            audit_descriptions(&client)
                .iter()
                .any(|d| d.starts_with("Database backup failed"))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

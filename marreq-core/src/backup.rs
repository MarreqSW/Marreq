// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Whole-database backups via `pg_dump` (issue #247).
//!
//! The dump is written as gzipped plain SQL to a private temp file; the caller
//! streams it to the client and removes it. The database password is passed to
//! the child process only (never the server's own environment or argv).

use std::path::{Path, PathBuf};
use std::process::Stdio;

/// Environment variable overriding the `pg_dump` binary (defaults to `pg_dump` on `PATH`).
pub const PG_DUMP_ENV: &str = "MARREQ_PG_DUMP";

const STDERR_TAIL_CHARS: usize = 500;

/// A finished dump waiting to be sent to the client.
#[derive(Debug)]
pub struct BackupFile {
    pub path: PathBuf,
    pub filename: String,
}

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("DATABASE_URL is not a valid PostgreSQL URL")]
    InvalidDatabaseUrl,
    #[error("pg_dump is not available on the server: {0}")]
    Unavailable(String),
    #[error("could not create a temporary backup file: {0}")]
    TempFile(String),
    #[error("pg_dump failed: {0}")]
    DumpFailed(String),
}

/// Where to dump from and which `pg_dump` to run.
///
/// Read from the environment by default; tests (or embedders) can put one in
/// Rocket managed state to override it.
#[derive(Debug, Clone)]
pub struct BackupConfig {
    pub database_url: Option<String>,
    pub pg_dump: String,
}

impl BackupConfig {
    /// `DATABASE_URL` and [`PG_DUMP_ENV`] (default `pg_dump` on `PATH`).
    pub fn from_env() -> Self {
        let non_empty = |key: &str| std::env::var(key).ok().filter(|v| !v.trim().is_empty());
        Self {
            database_url: non_empty("DATABASE_URL"),
            pg_dump: non_empty(PG_DUMP_ENV).unwrap_or_else(|| "pg_dump".into()),
        }
    }
}

/// Request guard: a [`BackupConfig`] from Rocket managed state when one is
/// registered (tests, embedders), otherwise from the environment. Avoids an
/// `Option<&State<_>>` parameter, which Rocket's sentinels reject when unmanaged.
#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for BackupConfig {
    type Error = std::convert::Infallible;

    async fn from_request(
        req: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let config = req
            .rocket()
            .state::<BackupConfig>()
            .cloned()
            .unwrap_or_else(BackupConfig::from_env);
        rocket::request::Outcome::Success(config)
    }
}

/// Download name for a backup taken at `now` (UTC), e.g. `marreq-backup_20260926_101500.sql.gz`.
pub fn backup_filename(now: chrono::DateTime<chrono::Utc>) -> String {
    format!("marreq-backup_{}.sql.gz", now.format("%Y%m%d_%H%M%S"))
}

/// Split a connection URL into a password-free URL and the decoded password.
fn split_password(database_url: &str) -> Result<(String, Option<String>), BackupError> {
    let mut url = url::Url::parse(database_url).map_err(|_| BackupError::InvalidDatabaseUrl)?;
    if !matches!(url.scheme(), "postgres" | "postgresql") {
        return Err(BackupError::InvalidDatabaseUrl);
    }
    let password = url
        .password()
        .map(|p| urlencoding::decode(p).map(|d| d.into_owned()))
        .transpose()
        .map_err(|_| BackupError::InvalidDatabaseUrl)?;
    url.set_password(None)
        .map_err(|_| BackupError::InvalidDatabaseUrl)?;
    Ok((url.to_string(), password))
}

fn create_private_temp_file() -> Result<PathBuf, BackupError> {
    use rand::Rng;
    use std::fs::OpenOptions;

    let suffix: u64 = rand::thread_rng().gen();
    let path = std::env::temp_dir().join(format!("marreq-backup-{suffix:016x}.sql.gz"));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(&path)
        .map_err(|e| BackupError::TempFile(e.to_string()))?;
    Ok(path)
}

fn stderr_summary(stderr: &[u8], password: Option<&str>) -> String {
    let mut text = String::from_utf8_lossy(stderr).trim().to_string();
    if let Some(pw) = password.filter(|p| !p.is_empty()) {
        text = text.replace(pw, "***");
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() > STDERR_TAIL_CHARS {
        text = chars[chars.len() - STDERR_TAIL_CHARS..].iter().collect();
    }
    if text.is_empty() {
        "no error output".into()
    } else {
        text
    }
}

/// Dump the whole database behind `database_url` to a gzipped plain-SQL temp file.
pub async fn create_backup(database_url: &str, pg_dump: &str) -> Result<BackupFile, BackupError> {
    let (dbname, password) = split_password(database_url)?;
    let path = create_private_temp_file()?;

    let mut command = tokio::process::Command::new(pg_dump);
    command
        .arg(format!("--dbname={dbname}"))
        .arg("--no-owner")
        .arg("--no-privileges")
        .arg("--compress=6")
        .arg(format!("--file={}", path.display()))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    match &password {
        Some(pw) => command.env("PGPASSWORD", pw),
        None => command.env_remove("PGPASSWORD"),
    };

    let output = match command.output().await {
        Ok(output) => output,
        Err(e) => {
            remove_quietly(&path);
            return Err(BackupError::Unavailable(e.to_string()));
        }
    };
    if !output.status.success() {
        remove_quietly(&path);
        return Err(BackupError::DumpFailed(stderr_summary(
            &output.stderr,
            password.as_deref(),
        )));
    }

    Ok(BackupFile {
        path,
        filename: backup_filename(chrono::Utc::now()),
    })
}

/// Best-effort removal of a temp file (used on error paths and after opening).
pub fn remove_quietly(path: &Path) {
    let _ = std::fs::remove_file(path);
}

#[cfg(all(test, unix))]
pub(crate) mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// Write an executable fake `pg_dump` script and return its path.
    pub fn fake_pg_dump(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("fake_pg_dump.sh");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "marreq-backup-test-{name}-{}",
            rand::random::<u32>()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn split_password_removes_and_decodes_password() {
        let (url, pw) = split_password("postgres://rust:p%40ss@db:5432/marreq").unwrap();
        assert_eq!(url, "postgres://rust@db:5432/marreq");
        assert_eq!(pw.as_deref(), Some("p@ss"));

        let (url, pw) = split_password("postgresql://db/marreq").unwrap();
        assert_eq!(url, "postgresql://db/marreq");
        assert!(pw.is_none());

        assert!(split_password("mysql://u:p@db/x").is_err());
        assert!(split_password("not a url").is_err());
    }

    #[test]
    fn filename_uses_utc_timestamp() {
        let at = chrono::DateTime::parse_from_rfc3339("2026-09-26T10:15:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        assert_eq!(backup_filename(at), "marreq-backup_20260926_101500.sql.gz");
    }

    #[test]
    fn stderr_summary_redacts_password_and_truncates() {
        let long = format!("{}secret", "x".repeat(1000));
        let summary = stderr_summary(long.as_bytes(), Some("secret"));
        assert!(summary.ends_with("***"));
        assert!(summary.chars().count() <= STDERR_TAIL_CHARS);
        assert_eq!(stderr_summary(b"", None), "no error output");
    }

    #[tokio::test]
    async fn successful_dump_writes_file_and_passes_password_to_child_only() {
        let dir = scratch_dir("ok");
        // Record argv and PGPASSWORD, then write the "dump" to the --file target.
        let script = fake_pg_dump(
            &dir,
            &format!(
                r#"out=""; for a in "$@"; do case "$a" in --file=*) out="${{a#--file=}}";; esac; done
echo "$@" > {args}
printf '%s' "$PGPASSWORD" > {pw}
printf 'dump-bytes' > "$out""#,
                args = dir.join("args").display(),
                pw = dir.join("pw").display(),
            ),
        );

        let backup = create_backup(
            "postgres://rust:s3cret@db:5432/marreq",
            script.to_str().unwrap(),
        )
        .await
        .unwrap();

        assert!(backup.filename.starts_with("marreq-backup_"));
        assert!(backup.filename.ends_with(".sql.gz"));
        assert_eq!(std::fs::read(&backup.path).unwrap(), b"dump-bytes");
        let mode = std::fs::metadata(&backup.path)
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "backup temp file must be private");

        let args = std::fs::read_to_string(dir.join("args")).unwrap();
        assert!(args.contains("--dbname=postgres://rust@db:5432/marreq"));
        assert!(args.contains("--compress=6"));
        assert!(!args.contains("s3cret"), "password must not appear in argv");
        assert_eq!(std::fs::read_to_string(dir.join("pw")).unwrap(), "s3cret");

        remove_quietly(&backup.path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn failed_dump_reports_stderr_and_cleans_up() {
        let dir = scratch_dir("fail");
        let script = fake_pg_dump(
            &dir,
            "echo 'connection to server failed: password s3cret rejected' >&2; exit 1",
        );

        let err = create_backup("postgres://rust:s3cret@db/marreq", script.to_str().unwrap())
            .await
            .unwrap_err();
        match err {
            BackupError::DumpFailed(msg) => {
                assert!(msg.contains("connection to server failed"));
                assert!(!msg.contains("s3cret"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn missing_binary_is_unavailable() {
        let err = create_backup("postgres://db/marreq", "/nonexistent/pg_dump")
            .await
            .unwrap_err();
        assert!(matches!(err, BackupError::Unavailable(_)));
    }
}

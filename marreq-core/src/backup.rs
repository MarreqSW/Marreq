// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Whole-instance backups (issues #247, #341).
//!
//! `pg_dump` writes plain SQL to a private temp file; that dump and, unless
//! left out, every stored attachment file are then packed into one private
//! `.tar.gz`, which the caller streams to the client and removes:
//!
//! ```text
//! marreq-backup_<ts>/database.sql          plain SQL, restore with psql
//! marreq-backup_<ts>/attachments/ab/cd/…   the attachment store, as on disk
//! marreq-backup_<ts>/manifest.json         what the archive contains
//! ```
//!
//! The database password is passed to the child process only (never the
//! server's own environment or argv).

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// `format` value of `manifest.json`.
pub const MANIFEST_FORMAT: &str = "marreq.backup.v1";

/// Environment variable overriding the `pg_dump` binary (defaults to `pg_dump` on `PATH`).
pub const PG_DUMP_ENV: &str = "MARREQ_PG_DUMP";

const STDERR_TAIL_CHARS: usize = 500;

/// A finished backup archive waiting to be sent to the client.
#[derive(Debug)]
pub struct BackupFile {
    pub path: PathBuf,
    pub filename: String,
    pub includes_attachments: bool,
    pub attachments: AttachmentUsage,
}

/// Attachment files found in the store (`tmp/` excluded).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct AttachmentUsage {
    pub files: u64,
    pub bytes: u64,
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
    #[error("could not write the backup archive: {0}")]
    Archive(String),
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

/// Name of a backup taken at `now` (UTC) without extension, e.g. `marreq-backup_20260926_101500`.
/// It is also the archive's top-level directory.
pub fn backup_stem(now: chrono::DateTime<chrono::Utc>) -> String {
    format!("marreq-backup_{}", now.format("%Y%m%d_%H%M%S"))
}

/// Download name for a backup taken at `now` (UTC), e.g. `marreq-backup_20260926_101500.tar.gz`.
pub fn backup_filename(now: chrono::DateTime<chrono::Utc>) -> String {
    format!("{}.tar.gz", backup_stem(now))
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

fn create_private_temp_file(extension: &str) -> Result<PathBuf, BackupError> {
    use std::fs::OpenOptions;

    let suffix: u64 = rand::random();
    let path = std::env::temp_dir().join(format!("marreq-backup-{suffix:016x}.{extension}"));
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

/// Back up the database behind `database_url` and, when `attachments_dir` is
/// given, the attachment files under it, into one private `.tar.gz`.
pub async fn create_backup(
    database_url: &str,
    pg_dump: &str,
    attachments_dir: Option<PathBuf>,
) -> Result<BackupFile, BackupError> {
    let created_at = chrono::Utc::now();
    let sql = dump_database(database_url, pg_dump).await?;
    let archive = match create_private_temp_file("tar.gz") {
        Ok(path) => path,
        Err(e) => {
            remove_quietly(&sql);
            return Err(e);
        }
    };

    let stem = backup_stem(created_at);
    let includes_attachments = attachments_dir.is_some();
    let (sql_path, archive_path) = (sql.clone(), archive.clone());
    let written = tokio::task::spawn_blocking(move || {
        write_archive(
            &archive_path,
            &sql_path,
            &stem,
            attachments_dir.as_deref(),
            created_at,
        )
    })
    .await;
    remove_quietly(&sql);
    let attachments = match written {
        Ok(Ok(usage)) => usage,
        Ok(Err(e)) => {
            remove_quietly(&archive);
            return Err(BackupError::Archive(e.to_string()));
        }
        Err(e) => {
            remove_quietly(&archive);
            return Err(BackupError::Archive(e.to_string()));
        }
    };

    Ok(BackupFile {
        path: archive,
        filename: backup_filename(created_at),
        includes_attachments,
        attachments,
    })
}

/// Dump the whole database behind `database_url` as plain SQL to a private temp file.
async fn dump_database(database_url: &str, pg_dump: &str) -> Result<PathBuf, BackupError> {
    let (dbname, password) = split_password(database_url)?;
    let path = create_private_temp_file("sql")?;

    let mut command = tokio::process::Command::new(pg_dump);
    command
        .arg(format!("--dbname={dbname}"))
        .arg("--no-owner")
        .arg("--no-privileges")
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
    Ok(path)
}

/// Pack the dump, the attachment files and a manifest into the (already
/// created, private) file at `out`.
fn write_archive(
    out: &Path,
    sql: &Path,
    stem: &str,
    attachments_dir: Option<&Path>,
    created_at: chrono::DateTime<chrono::Utc>,
) -> io::Result<AttachmentUsage> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(out)?;
    let gz = flate2::write::GzEncoder::new(BufWriter::new(file), flate2::Compression::new(6));
    let mut tar = tar::Builder::new(gz);
    tar.follow_symlinks(false);

    tar.append_path_with_name(sql, format!("{stem}/database.sql"))?;

    let mut usage = AttachmentUsage::default();
    if let Some(dir) = attachments_dir {
        let prefix = PathBuf::from(format!("{stem}/attachments"));
        walk_attachment_store(dir, &mut |path, relative| {
            // A file deleted since it was listed is simply not in the backup.
            let mut file = match File::open(path) {
                Ok(file) => file,
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(e) => return Err(e),
            };
            let size = file.metadata()?.len();
            tar.append_file(prefix.join(relative), &mut file)?;
            usage.files += 1;
            usage.bytes += size;
            Ok(())
        })?;
    }

    let manifest = serde_json::json!({
        "format": MANIFEST_FORMAT,
        "created_at": created_at.to_rfc3339(),
        "marreq_version": env!("CARGO_PKG_VERSION"),
        "database": "database.sql: plain SQL from pg_dump (--no-owner --no-privileges); restore into an empty database with psql",
        "includes_attachments": attachments_dir.is_some(),
        "attachment_files": usage.files,
        "attachment_bytes": usage.bytes,
        "attachments": "attachments/: the contents of MARREQ_ATTACHMENTS_DIR (ab/cd/<sha256>); copy back into the attachments volume. Taken after the database dump: files uploaded meanwhile may be extra, files deleted meanwhile are left out.",
    });
    let body = serde_json::to_vec_pretty(&manifest).map_err(io::Error::other)?;
    let mut header = tar::Header::new_gnu();
    header.set_size(body.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(created_at.timestamp().max(0) as u64);
    header.set_cksum();
    tar.append_data(
        &mut header,
        format!("{stem}/manifest.json"),
        body.as_slice(),
    )?;

    let gz = tar.into_inner()?;
    let mut writer = gz.finish()?;
    writer.flush()?;
    writer
        .into_inner()
        .map_err(|e| e.into_error())?
        .sync_all()?;
    Ok(usage)
}

/// Visit every regular file of the attachment store with its path relative to
/// `dir`. Skips the `tmp/` upload area and symbolic links; a missing store is
/// an empty one.
fn walk_attachment_store(
    dir: &Path,
    visit: &mut dyn FnMut(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    fn walk(
        root: &Path,
        dir: &Path,
        visit: &mut dyn FnMut(&Path, &Path) -> io::Result<()>,
    ) -> io::Result<()> {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        let mut entries: Vec<_> = entries.collect::<io::Result<_>>()?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let kind = entry.file_type()?;
            if dir == root && entry.file_name() == "tmp" {
                continue;
            }
            if kind.is_dir() {
                walk(root, &path, visit)?;
            } else if kind.is_file() {
                let relative = path.strip_prefix(root).map_err(io::Error::other)?;
                visit(&path, relative)?;
            }
        }
        Ok(())
    }
    walk(dir, dir, visit)
}

/// Number and total size of the stored attachment files (blocking: walks the store).
pub fn attachment_usage(dir: &Path) -> io::Result<AttachmentUsage> {
    let mut usage = AttachmentUsage::default();
    walk_attachment_store(dir, &mut |path, _| {
        match std::fs::metadata(path) {
            Ok(meta) => {
                usage.files += 1;
                usage.bytes += meta.len();
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        Ok(())
    })?;
    Ok(usage)
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
        assert_eq!(backup_filename(at), "marreq-backup_20260926_101500.tar.gz");
        assert_eq!(backup_stem(at), "marreq-backup_20260926_101500");
    }

    #[test]
    fn stderr_summary_redacts_password_and_truncates() {
        let long = format!("{}secret", "x".repeat(1000));
        let summary = stderr_summary(long.as_bytes(), Some("secret"));
        assert!(summary.ends_with("***"));
        assert!(summary.chars().count() <= STDERR_TAIL_CHARS);
        assert_eq!(stderr_summary(b"", None), "no error output");
    }

    /// Every entry of a `.tar.gz`: (path, contents).
    pub fn archive_entries(path: &Path) -> Vec<(String, Vec<u8>)> {
        use std::io::Read;
        let file = File::open(path).unwrap();
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
        archive
            .entries()
            .unwrap()
            .map(|entry| {
                let mut entry = entry.unwrap();
                let name = entry.path().unwrap().display().to_string();
                let mut body = Vec::new();
                entry.read_to_end(&mut body).unwrap();
                (name, body)
            })
            .collect()
    }

    /// An attachment store with two blobs and a leftover upload in `tmp/`.
    fn attachment_store(dir: &Path) -> PathBuf {
        let store = dir.join("attachments");
        std::fs::create_dir_all(store.join("ab/cd")).unwrap();
        std::fs::create_dir_all(store.join("ef/01")).unwrap();
        std::fs::create_dir_all(store.join("tmp")).unwrap();
        std::fs::write(store.join("ab/cd/abcd1"), b"first file").unwrap();
        std::fs::write(store.join("ef/01/ef012"), b"second").unwrap();
        std::fs::write(store.join("tmp/upload-1"), b"half uploaded").unwrap();
        store
    }

    /// A fake pg_dump that records argv, PGPASSWORD and its --file target.
    fn recording_pg_dump(dir: &Path) -> PathBuf {
        fake_pg_dump(
            dir,
            &format!(
                r#"out=""; for a in "$@"; do case "$a" in --file=*) out="${{a#--file=}}";; esac; done
echo "$@" > {args}
printf '%s' "$PGPASSWORD" > {pw}
printf '%s' "$out" > {target}
printf 'dump-bytes' > "$out""#,
                args = dir.join("args").display(),
                pw = dir.join("pw").display(),
                target = dir.join("target").display(),
            ),
        )
    }

    #[tokio::test]
    async fn backup_packs_dump_attachments_and_manifest() {
        let dir = scratch_dir("ok");
        let store = attachment_store(&dir);
        let script = recording_pg_dump(&dir);

        let backup = create_backup(
            "postgres://rust:s3cret@db:5432/marreq",
            script.to_str().unwrap(),
            Some(store),
        )
        .await
        .unwrap();

        assert!(backup.filename.starts_with("marreq-backup_"));
        assert!(backup.filename.ends_with(".tar.gz"));
        assert!(backup.includes_attachments);
        assert_eq!(
            backup.attachments,
            AttachmentUsage {
                files: 2,
                bytes: 16
            }
        );
        let mode = std::fs::metadata(&backup.path)
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "backup temp file must be private");

        let stem = backup.filename.trim_end_matches(".tar.gz");
        let entries = archive_entries(&backup.path);
        let names: Vec<&str> = entries.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            vec![
                format!("{stem}/database.sql"),
                format!("{stem}/attachments/ab/cd/abcd1"),
                format!("{stem}/attachments/ef/01/ef012"),
                format!("{stem}/manifest.json"),
            ]
        );
        assert_eq!(entries[0].1, b"dump-bytes");
        assert_eq!(entries[1].1, b"first file");
        let manifest: serde_json::Value = serde_json::from_slice(&entries[3].1).unwrap();
        assert_eq!(manifest["format"], MANIFEST_FORMAT);
        assert_eq!(manifest["includes_attachments"], true);
        assert_eq!(manifest["attachment_files"], 2);
        assert_eq!(manifest["attachment_bytes"], 16);

        let args = std::fs::read_to_string(dir.join("args")).unwrap();
        assert!(args.contains("--dbname=postgres://rust@db:5432/marreq"));
        assert!(
            !args.contains("--compress"),
            "the archive is compressed as a whole"
        );
        assert!(!args.contains("s3cret"), "password must not appear in argv");
        assert_eq!(std::fs::read_to_string(dir.join("pw")).unwrap(), "s3cret");
        let dump_target = std::fs::read_to_string(dir.join("target")).unwrap();
        assert!(
            !Path::new(&dump_target).exists(),
            "the intermediate SQL file is removed"
        );

        remove_quietly(&backup.path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn database_only_backup_has_no_attachments() {
        let dir = scratch_dir("dbonly");
        let script = recording_pg_dump(&dir);

        let backup = create_backup("postgres://db/marreq", script.to_str().unwrap(), None)
            .await
            .unwrap();

        assert!(!backup.includes_attachments);
        let entries = archive_entries(&backup.path);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].0.ends_with("/database.sql"));
        let manifest: serde_json::Value = serde_json::from_slice(&entries[1].1).unwrap();
        assert_eq!(manifest["includes_attachments"], false);
        assert_eq!(manifest["attachment_files"], 0);

        remove_quietly(&backup.path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn attachment_usage_counts_stored_files_only() {
        let dir = scratch_dir("usage");
        let store = attachment_store(&dir);
        assert_eq!(
            attachment_usage(&store).unwrap(),
            AttachmentUsage {
                files: 2,
                bytes: 16
            }
        );
        assert_eq!(
            attachment_usage(&dir.join("missing")).unwrap(),
            AttachmentUsage::default()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn failed_dump_reports_stderr_and_cleans_up() {
        let dir = scratch_dir("fail");
        let script = fake_pg_dump(
            &dir,
            "echo 'connection to server failed: password s3cret rejected' >&2; exit 1",
        );

        let err = create_backup(
            "postgres://rust:s3cret@db/marreq",
            script.to_str().unwrap(),
            None,
        )
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
        let err = create_backup("postgres://db/marreq", "/nonexistent/pg_dump", None)
            .await
            .unwrap_err();
        assert!(matches!(err, BackupError::Unavailable(_)));
    }
}

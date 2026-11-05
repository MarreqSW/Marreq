//! Integrity evidence around the existing marreq.backup.v1 archive, not a second backup format.
use anyhow::{Context, Result, ensure};
use diesel::{PgConnection, RunQueryDsl, prelude::*};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::os::unix::fs::OpenOptionsExt;
use std::{collections::BTreeMap, fs::File, io::Read, path::Path};

#[derive(Debug, Serialize, Deserialize, QueryableByName, PartialEq, Eq)]
pub struct Attachment {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub sha256: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub size_bytes: i64,
}
#[derive(Serialize, Deserialize)]
pub struct Evidence {
    pub format: String,
    pub database: String,
    pub history: Vec<String>,
    pub archive_sha256: String,
    pub attachments: Vec<Attachment>,
    pub source_sha: String,
    pub application_version: String,
    pub writes_quiesced: bool,
}
#[derive(QueryableByName)]
struct Identity {
    #[diesel(sql_type = diesel::sql_types::Text)]
    value: String,
}
fn identity(conn: &mut PgConnection) -> Result<String> {
    // Database OID + cluster address are recovery evidence, never credentials.
    let row: Identity = diesel::sql_query("SELECT current_database() || ':' || oid::text || ':' || coalesce(inet_server_addr()::text, 'local') || ':' || coalesce(inet_server_port()::text, 'local') AS value FROM pg_database WHERE datname=current_database()").get_result(conn)?;
    Ok(row.value)
}
pub fn attachments(conn: &mut PgConnection, history: &[String]) -> Result<Vec<Attachment>> {
    if !history.iter().any(|v| v == "20260930000001") {
        return Ok(vec![]);
    }
    // Include soft deleted files too: baselines can still refer to them.
    Ok(diesel::sql_query(
        "SELECT DISTINCT sha256::text, size_bytes FROM attachments ORDER BY sha256, size_bytes",
    )
    .load(conn)?)
}
fn digest(mut input: impl Read) -> Result<String> {
    let mut hash = Sha256::new();
    let mut buf = [0; 65536];
    loop {
        let len = input.read(&mut buf)?;
        if len == 0 {
            break;
        }
        hash.update(&buf[..len]);
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn record(bundle: &Path, conn: &mut PgConnection) -> Result<()> {
    let history = crate::history(conn)?;
    crate::validate_prefix(&history, &crate::expected())?;
    let evidence = Evidence {
        format: "marreq.recovery.v1".into(),
        database: identity(conn)?,
        attachments: attachments(conn, &history)?,
        history,
        archive_sha256: digest(File::open(bundle.join("backup.tar.gz"))?)?,
        source_sha: option_env!("MARREQ_GIT_SHA")
            .unwrap_or("development")
            .into(),
        application_version: option_env!("MARREQ_VERSION")
            .filter(|value| !value.is_empty())
            .unwrap_or(env!("CARGO_PKG_VERSION"))
            .into(),
        writes_quiesced: true,
    };
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(bundle.join("recovery.json"))?;
    serde_json::to_writer_pretty(file, &evidence)?;
    verify(bundle, conn, true)
}

/// Verify the entire archive by streaming. No extraction, symlinks, traversal or unbounded blobs.
pub fn verify(bundle: &Path, conn: &mut PgConnection, source: bool) -> Result<()> {
    let evidence: Evidence = serde_json::from_reader(
        File::open(bundle.join("recovery.json")).context("recovery evidence missing")?,
    )?;
    ensure!(
        evidence.format == "marreq.recovery.v1" && evidence.writes_quiesced,
        "invalid or non-quiesced recovery evidence"
    );
    let archive = bundle.join("backup.tar.gz");
    ensure!(
        digest(File::open(&archive).context("backup.tar.gz missing or unreadable")?)?
            == evidence.archive_sha256,
        "backup checksum mismatch"
    );
    ensure!(
        crate::history(conn)? == evidence.history,
        "backup migration history differs from database"
    );
    ensure!(
        attachments(conn, &evidence.history)? == evidence.attachments,
        "backup attachment references differ from database"
    );
    if source {
        ensure!(
            identity(conn)? == evidence.database,
            "backup belongs to a different database"
        );
    }
    let gz = flate2::read::GzDecoder::new(File::open(archive)?);
    let mut tar = tar::Archive::new(gz);
    let mut files = BTreeMap::new();
    let mut root = None;
    let mut manifest = None;
    let mut sql = false;
    let mut count = 0u64;
    let mut bytes = 0u64;
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let parts: Vec<_> = path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        ensure!(
            parts.len() >= 2 && parts.iter().all(|s| s != ".." && s != "." && s != "/"),
            "unsafe archive path"
        );
        ensure!(
            entry.header().entry_type().is_file(),
            "archive contains non-file entry"
        );
        if let Some(r) = &root {
            ensure!(r == &parts[0], "multiple archive roots");
        } else {
            root = Some(parts[0].clone());
        }
        let relative = parts[1..].join("/");
        ensure!(!files.contains_key(&relative), "duplicate archive entry");
        let size = entry.size();
        let hash = if relative == "manifest.json" {
            ensure!(size <= 65536, "oversized backup manifest");
            let mut body = String::new();
            entry.read_to_string(&mut body)?;
            manifest = Some(serde_json::from_str::<serde_json::Value>(&body)?);
            String::new()
        } else {
            if relative == "database.sql" {
                ensure!(size > 0, "empty database dump");
                sql = true;
            } else {
                ensure!(
                    parts.len() == 5 && parts[1] == "attachments",
                    "unexpected backup file"
                );
                let name = &parts[4];
                ensure!(
                    name.len() == 64
                        && name
                            .bytes()
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                    "invalid blob name"
                );
                ensure!(
                    parts[2] == name[..2] && parts[3] == name[2..4],
                    "invalid blob path"
                );
                count += 1;
                bytes += size;
            }
            digest(&mut entry)?
        };
        if relative.starts_with("attachments/") {
            ensure!(parts[4] == hash, "attachment content hash mismatch");
        }
        files.insert(relative, (size, hash));
    }
    // Drain the gzip footer too; detects truncated/corrupt compressed streams.
    let mut gz = tar.into_inner();
    std::io::copy(&mut gz, &mut std::io::sink())?;
    let manifest = manifest.context("backup manifest missing")?;
    ensure!(
        sql && manifest["format"] == "marreq.backup.v1" && manifest["includes_attachments"] == true,
        "full database and attachment archive required"
    );
    ensure!(
        manifest["attachment_files"].as_u64() == Some(count)
            && manifest["attachment_bytes"].as_u64() == Some(bytes),
        "backup manifest totals mismatch"
    );
    for blob in &evidence.attachments {
        let name = &blob.sha256;
        ensure!(
            name.len() == 64
                && name
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "invalid database attachment hash"
        );
        let path = format!("attachments/{}/{}/{name}", &name[..2], &name[2..4]);
        ensure!(
            files
                .get(&path)
                .map(|(size, hash)| (*size as i64, hash.as_str()))
                == Some((blob.size_bytes, name.as_str())),
            "referenced attachment missing or corrupt: {name}"
        );
    }
    println!("Backup integrity, history and attachment references verified");
    Ok(())
}

/// Record a rehearsal only against a different, restored database and its restored blob store.
pub fn verify_restored(bundle: &Path, conn: &mut PgConnection) -> Result<()> {
    verify(bundle, conn, false)?;
    let evidence: Evidence = serde_json::from_reader(File::open(bundle.join("recovery.json"))?)?;
    ensure!(
        identity(conn)? != evidence.database,
        "restore rehearsal must use an isolated database"
    );
    let dir = std::env::var("MARREQ_ATTACHMENTS_DIR")
        .context("restored attachment directory required")?;
    for blob in &evidence.attachments {
        let name = &blob.sha256;
        let file = File::open(
            Path::new(&dir)
                .join(&name[..2])
                .join(&name[2..4])
                .join(name),
        )?;
        ensure!(
            file.metadata()?.len() as i64 == blob.size_bytes && digest(file)? == *name,
            "restored attachment missing or corrupt"
        );
    }
    let invalid: Identity = diesel::sql_query("SELECT count(*)::text AS value FROM pg_constraint WHERE NOT convalidated AND connamespace='public'::regnamespace").get_result(conn)?;
    ensure!(
        invalid.value == "0",
        "restored database has unvalidated constraints; investigate before proceeding"
    );
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(bundle.join("restore-verified.json"))?;
    serde_json::to_writer_pretty(
        file,
        &serde_json::json!({"archive_sha256": evidence.archive_sha256, "restored_database": identity(conn)?}),
    )?;
    println!("Isolated restore verified; application smoke tests remain a deployment gate");
    Ok(())
}

pub fn require_restore(bundle: &Path) -> Result<()> {
    let evidence: Evidence = serde_json::from_reader(File::open(bundle.join("recovery.json"))?)?;
    let receipt: serde_json::Value = serde_json::from_reader(
        File::open(bundle.join("restore-verified.json"))
            .context("isolated restore rehearsal receipt missing")?,
    )?;
    ensure!(
        receipt["archive_sha256"].as_str() == Some(&evidence.archive_sha256)
            && receipt["restored_database"]
                .as_str()
                .is_some_and(|s| s != evidence.database),
        "restore evidence does not match this backup"
    );
    Ok(())
}

//! Offline release backup using the existing application archive implementation.
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;
#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Recovery backup failed: {error}");
        std::process::exit(1);
    }
}
async fn run() -> Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    ensure!(
        std::env::var("MARREQ_WRITES_QUIESCED").as_deref() == Ok("true"),
        "stop every database/attachment writer and set MARREQ_WRITES_QUIESCED=true"
    );
    let bundle = PathBuf::from(
        std::env::var("MARREQ_RECOVERY_BUNDLE")
            .context("MARREQ_RECOVERY_BUNDLE required (new directory)")?,
    );
    let dir = PathBuf::from(
        std::env::var("MARREQ_ATTACHMENTS_DIR").context("MARREQ_ATTACHMENTS_DIR required")?,
    );
    ensure!(dir.is_dir(), "attachment store is missing");
    std::fs::DirBuilder::new().mode(0o700).create(&bundle)?;
    let mut conn = marreq_migrations::connect()?;
    marreq_migrations::lock(&mut conn, std::time::Duration::from_secs(60))?;
    let config = marreq_core::backup::BackupConfig::from_env();
    let backup = marreq_core::backup::create_backup(
        config
            .database_url
            .as_deref()
            .context("DATABASE_URL required")?,
        &config.pg_dump,
        Some(dir),
    )
    .await?;
    // copy (rather than rename) also supports separately mounted backup volumes.
    let copied = std::fs::copy(&backup.path, bundle.join("backup.tar.gz"));
    marreq_core::backup::remove_quietly(&backup.path);
    copied?;
    marreq_migrations::recovery::record(&bundle, &mut conn)?;
    println!("Recovery bundle created; perform an isolated restore rehearsal before proceeding");
    Ok(())
}

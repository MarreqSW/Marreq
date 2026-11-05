//! Release migrations: one session owns both the advisory lock and Diesel execution.
use anyhow::{Context, Result, bail, ensure};
use diesel::{Connection, PgConnection, RunQueryDsl, connection::SimpleConnection, prelude::*};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::time::{Duration, Instant};

pub mod recovery;
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../marreq-core/migrations");
// Stable across images, hosts and application variants; scoped by PostgreSQL database.
pub const LOCK: i64 = 0x4d4152524551;

#[derive(QueryableByName)]
struct Version {
    #[diesel(sql_type = diesel::sql_types::Text)]
    version: String,
}
#[derive(QueryableByName)]
struct Flag {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    value: bool,
}

pub fn connect() -> Result<PgConnection> {
    let url = std::env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    // Do not propagate libpq errors: they can contain credentials/connection strings.
    PgConnection::establish(&url).map_err(|_| {
        anyhow::anyhow!("database connection failed (check configuration and connectivity)")
    })
}

pub fn expected() -> Vec<String> {
    use diesel::migration::MigrationSource;
    let mut versions: Vec<_> =
        <EmbeddedMigrations as MigrationSource<diesel::pg::Pg>>::migrations(&MIGRATIONS)
            .expect("embedded migrations are valid")
            .iter()
            .map(|m| m.name().version().to_string())
            .collect();
    versions.sort();
    versions
}

/// Read-only: unlike Diesel's applied_migrations(), this never creates the history table.
pub fn history(conn: &mut PgConnection) -> Result<Vec<String>> {
    let exists: Flag = diesel::sql_query(
        "SELECT to_regclass('public.__diesel_schema_migrations') IS NOT NULL AS value",
    )
    .get_result(conn)?;
    if !exists.value {
        let populated: Flag = diesel::sql_query("SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE') AS value").get_result(conn)?;
        ensure!(
            !populated.value,
            "migration history missing on a nonempty database; reconcile manually"
        );
        return Ok(vec![]);
    }
    let applied: Vec<String> = diesel::sql_query(
        "SELECT version::text FROM public.__diesel_schema_migrations ORDER BY version",
    )
    .load::<Version>(conn)?
    .into_iter()
    .map(|v| v.version)
    .collect();
    if applied.is_empty() {
        let populated: Flag = diesel::sql_query("SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE' AND table_name <> '__diesel_schema_migrations') AS value").get_result(conn)?;
        ensure!(
            !populated.value,
            "empty migration history on an existing schema; reconcile manually"
        );
    }
    Ok(applied)
}

pub fn validate_prefix(applied: &[String], known: &[String]) -> Result<()> {
    ensure!(
        applied.len() <= known.len() && known[..applied.len()] == *applied,
        "migration history is unknown, out of order, or has gaps; no schema changes applied"
    );
    Ok(())
}

/// Exact history by default. A reviewed overlap may pin an explicit upper version;
/// the release pipeline must prove that expansion against retained old/new images.
pub fn check(conn: &mut PgConnection) -> Result<()> {
    let applied = history(conn)?;
    let known = expected();
    ensure!(
        applied.len() >= known.len() && applied[..known.len()] == known,
        "required schema is missing or inconsistent; run the explicit migration job"
    );
    if applied.len() > known.len() {
        let through = std::env::var("MARREQ_SCHEMA_COMPATIBLE_THROUGH").unwrap_or_default();
        ensure!(
            applied.last() == Some(&through),
            "schema is newer than this binary; reviewed compatibility pin required"
        );
    }
    // Minimal read-only projections detect missing core structures as well as history drift.
    // Authenticated application smoke remains necessary for full semantic validation.
    conn.batch_execute("SELECT id, name FROM public.projects LIMIT 0; SELECT id, username, password_hash FROM public.users LIMIT 0; SELECT id, sha256, size_bytes FROM public.attachments LIMIT 0")?;
    Ok(())
}

pub fn lock(conn: &mut PgConnection, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    loop {
        let got: Flag = diesel::sql_query("SELECT pg_try_advisory_lock($1) AS value")
            .bind::<diesel::sql_types::BigInt, _>(LOCK)
            .get_result(conn)?;
        if got.value {
            return Ok(());
        }
        if start.elapsed() >= timeout {
            bail!("migration lock timeout; another migration job is active");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

pub fn apply(conn: &mut PgConnection, development: bool) -> Result<()> {
    apply_plan(conn, development, MIGRATIONS, expected())?;
    check(conn)?;
    if !development {
        // Historical files are immutable, including their bootstrap user. Block release
        // progression until an operator explicitly rotates/disables that public credential.
        let bootstrap: Flag = diesel::sql_query("SELECT EXISTS(SELECT 1 FROM public.users WHERE password_hash=$1) AS value")
            .bind::<diesel::sql_types::Text, _>("$argon2id$v=19$m=19456,t=2,p=1$3o6cC/67ksnBxHCCF9rGHA$oWCATKyiKRCdDgWucvrMHinlWvzZNhqoUUvnpyCgOW0")
            .get_result(conn)?;
        ensure!(
            !bootstrap.value,
            "legacy bootstrap password is enabled; schema is prepared but release is blocked: rotate/disable the bootstrap account, then rerun apply"
        );
    }
    Ok(())
}

fn apply_plan<S: diesel::migration::MigrationSource<diesel::pg::Pg>>(
    conn: &mut PgConnection,
    development: bool,
    source: S,
    known: Vec<String>,
) -> Result<()> {
    let seconds = std::env::var("MARREQ_MIGRATION_LOCK_SECONDS")
        .unwrap_or_else(|_| "60".into())
        .parse::<u64>()
        .context("invalid lock timeout")?;
    ensure!(seconds <= 3600, "lock timeout must be <= 3600 seconds");
    lock(conn, Duration::from_secs(seconds))?;
    // Connection drop releases the session lock on success, error, SIGTERM or SIGKILL.
    conn.batch_execute(
        "SET search_path TO public; SET lock_timeout='5s'; SET statement_timeout='15min'",
    )?;
    let applied = history(conn)?;
    validate_prefix(&applied, &known)?;
    if applied == known {
        println!("Schema already prepared");
        return Ok(());
    }
    if !development {
        ensure!(
            std::env::var("MARREQ_WRITES_QUIESCED").as_deref() == Ok("true"),
            "stop all writers and set MARREQ_WRITES_QUIESCED=true"
        );
        let bundle = std::env::var("MARREQ_RECOVERY_BUNDLE")
            .context("MARREQ_RECOVERY_BUNDLE is required for production migration")?;
        recovery::verify(std::path::Path::new(&bundle), conn, true)?;
        recovery::require_restore(std::path::Path::new(&bundle))?;
    }
    let permitted: Flag = diesel::sql_query("SELECT has_schema_privilege(current_user, 'public', 'CREATE') AND NOT EXISTS (SELECT 1 FROM pg_class WHERE relnamespace='public'::regnamespace AND relkind IN ('r','p','S') AND NOT pg_has_role(relowner, 'USAGE')) AS value").get_result(conn)?;
    ensure!(
        permitted.value,
        "migration role requires schema CREATE and ownership of existing tables/sequences"
    );
    // Diesel supplies per-migration transactions and version bookkeeping; do not duplicate it.
    conn.run_pending_migrations(source).map_err(|_| {
        anyhow::anyhow!(
            "migration failed; inspect PostgreSQL logs, reconcile nontransactional DDL before retry"
        )
    })?;
    ensure!(history(conn)? == known, "postflight history mismatch");
    println!("Migrations and schema postflight passed");
    Ok(())
}

pub fn startup_or_exit() {
    if let Err(error) = connect().and_then(|mut conn| check(&mut conn)) {
        eprintln!("Schema preflight failed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests;

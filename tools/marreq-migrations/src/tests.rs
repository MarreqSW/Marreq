use super::*;
use diesel_migrations::FileBasedMigrations;
use std::{fs, path::PathBuf, process::Command};

struct Database {
    url: String,
    name: String,
    admin: PgConnection,
}
impl Database {
    fn new() -> Self {
        let url = std::env::var("MARREQ_MIGRATION_TEST_URL")
            .expect("use scripts/test-migrations.sh; disposable DB only");
        let mut admin = PgConnection::establish(&url).unwrap();
        let name = format!(
            "migration_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        admin
            .batch_execute(&format!("CREATE DATABASE {name}"))
            .unwrap();
        Self {
            url: format!("{}/{name}", url.rsplit_once('/').unwrap().0),
            name,
            admin,
        }
    }
    fn conn(&self) -> PgConnection {
        PgConnection::establish(&self.url).unwrap()
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        self.admin
            .batch_execute(&format!("DROP DATABASE {} WITH (FORCE)", self.name))
            .unwrap();
    }
}
struct Fixtures(PathBuf);
impl Fixtures {
    fn new(sql: &[&str]) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "migration-fixtures-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        for (i, sql) in sql.iter().enumerate() {
            let path = dir.join(format!("{:014}_fixture", i + 1));
            fs::create_dir(&path).unwrap();
            fs::write(path.join("up.sql"), sql).unwrap();
            fs::write(path.join("down.sql"), "SELECT 1;").unwrap();
        }
        Self(dir)
    }
    fn source(&self) -> FileBasedMigrations {
        FileBasedMigrations::from_path(&self.0).unwrap()
    }
    fn run(&self, conn: &mut PgConnection) -> Result<()> {
        use diesel::migration::MigrationSource;
        let source = self.source();
        let mut known: Vec<_> =
            <FileBasedMigrations as MigrationSource<diesel::pg::Pg>>::migrations(&source)
                .unwrap()
                .iter()
                .map(|m| m.name().version().to_string())
                .collect();
        known.sort();
        apply_plan(conn, true, source, known)
    }
}
impl Drop for Fixtures {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn history_prefix_rejects_gaps_and_unknowns() {
    let known = vec!["1".into(), "2".into()];
    assert!(validate_prefix(&[], &known).is_ok());
    assert!(validate_prefix(&["2".into()], &known).is_err());
    assert!(validate_prefix(&["1".into(), "3".into()], &known).is_err());
}

#[test]
#[ignore = "requires disposable PostgreSQL; scripts/test-migrations.sh"]
fn fresh_repeated_and_invalid_history() {
    let db = Database::new();
    assert!(check(&mut db.conn()).is_err());
    apply(&mut db.conn(), true).unwrap();
    check(&mut db.conn()).unwrap();
    apply(&mut db.conn(), true).unwrap();
    let mut conn = db.conn();
    conn.batch_execute("DELETE FROM __diesel_schema_migrations WHERE version=(SELECT min(version) FROM __diesel_schema_migrations)").unwrap();
    assert!(apply(&mut conn, true).is_err());
    assert!(check(&mut conn).is_err());
    conn.batch_execute("DROP TABLE __diesel_schema_migrations")
        .unwrap();
    assert!(history(&mut conn).is_err());
}

#[test]
#[ignore = "requires disposable PostgreSQL"]
fn concurrent_jobs_and_timeout() {
    let db = Database::new();
    let fixture = Fixtures::new(&[
        "SELECT pg_sleep(1); CREATE TABLE sample (id int);",
        "ALTER TABLE sample ADD COLUMN optional text;",
    ]);
    let path = fixture.0.clone();
    let url = db.url.clone();
    let first = std::thread::spawn(move || {
        let mut conn = PgConnection::establish(&url).unwrap();
        apply_plan(
            &mut conn,
            true,
            FileBasedMigrations::from_path(path).unwrap(),
            vec!["00000000000001".into(), "00000000000002".into()],
        )
        .unwrap();
    });
    // Observe a lock owned by the actual first migration session, not a timing guess.
    let mut observer = db.conn();
    for i in 0..100 {
        let held: Flag = diesel::sql_query("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND database=(SELECT oid FROM pg_database WHERE datname=current_database())) AS value").get_result(&mut observer).unwrap();
        if held.value {
            break;
        }
        assert!(i < 99);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(lock(&mut db.conn(), Duration::from_millis(20)).is_err());
    fixture.run(&mut db.conn()).unwrap();
    first.join().unwrap();
    assert_eq!(history(&mut db.conn()).unwrap().len(), 2);
}

#[test]
#[ignore = "requires disposable PostgreSQL"]
fn failed_transaction_retry_and_permissions() {
    let db = Database::new();
    let fixture = Fixtures::new(&[
        "CREATE TABLE sample (id int);",
        "CREATE TABLE failed(id int); SELECT 1/0;",
    ]);
    assert!(fixture.run(&mut db.conn()).is_err());
    assert_eq!(history(&mut db.conn()).unwrap().len(), 1);
    assert!(db.conn().batch_execute("SELECT * FROM failed").is_err());
    // Test-only unreleased fixture repair: released migration files must never be edited.
    fs::write(
        fixture.0.join("00000000000002_fixture/up.sql"),
        "ALTER TABLE sample ADD COLUMN optional text;",
    )
    .unwrap();
    fixture.run(&mut db.conn()).unwrap();
    let mut conn = db.conn();
    conn.batch_execute("CREATE ROLE migration_no_ddl; SET ROLE migration_no_ddl")
        .unwrap();
    assert!(fixture.run(&mut conn).is_err());
    conn.batch_execute("RESET ROLE; DROP ROLE migration_no_ddl")
        .unwrap();
}

#[test]
#[ignore = "child process helper"]
fn interrupted_child() {
    let Ok(url) = std::env::var("MARREQ_INTERRUPTION_URL") else {
        return;
    };
    let path = std::env::var("MARREQ_INTERRUPTION_FIXTURE").unwrap();
    let mut conn = PgConnection::establish(&url).unwrap();
    apply_plan(
        &mut conn,
        true,
        FileBasedMigrations::from_path(path).unwrap(),
        vec!["00000000000001".into()],
    )
    .unwrap();
}

#[test]
#[ignore = "requires disposable PostgreSQL"]
fn interruption_releases_lock_and_rolls_back() {
    let db = Database::new();
    let fixture = Fixtures::new(&["CREATE TABLE interrupted(id int); SELECT pg_sleep(3);"]);
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "tests::interrupted_child"])
        .env("MARREQ_INTERRUPTION_URL", &db.url)
        .env("MARREQ_INTERRUPTION_FIXTURE", &fixture.0)
        .spawn()
        .unwrap();
    let mut observer = db.conn();
    let mut running = false;
    for _ in 0..200 {
        let active: Flag = diesel::sql_query("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND query LIKE 'CREATE TABLE interrupted%' AND wait_event='PgSleep') AS value").get_result(&mut observer).unwrap();
        if active.value {
            running = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(running);
    // PostgreSQL releases the dead session once the bounded statement finishes.
    lock(&mut db.conn(), Duration::from_secs(5)).unwrap();
    assert!(history(&mut db.conn()).unwrap().is_empty());
    fs::write(
        fixture.0.join("00000000000001_fixture/up.sql"),
        "CREATE TABLE interrupted(id int);",
    )
    .unwrap();
    fixture.run(&mut db.conn()).unwrap();
}

#[test]
#[ignore = "requires disposable PostgreSQL"]
fn expansion_supports_old_queries_and_rejects_unreviewed_new_history() {
    let db = Database::new();
    apply(&mut db.conn(), true).unwrap();
    let mut conn = db.conn();
    conn.batch_execute("ALTER TABLE projects ADD COLUMN optional_release_field text; SELECT id, name FROM projects; INSERT INTO __diesel_schema_migrations(version) VALUES ('20990101000001')").unwrap();
    // Schema additions preserve old SQL, but startup still needs a reviewed overlap pin.
    assert!(check(&mut conn).is_err());
    assert!(apply(&mut db.conn(), true).is_err());
    conn.batch_execute("DELETE FROM __diesel_schema_migrations WHERE version='20990101000001'")
        .unwrap();
    check(&mut conn).unwrap();
}

#[test]
#[ignore = "requires disposable PostgreSQL"]
fn missing_core_structure_is_rejected_despite_complete_history() {
    let db = Database::new();
    apply(&mut db.conn(), true).unwrap();
    let mut conn = db.conn();
    conn.batch_execute("ALTER TABLE attachments RENAME COLUMN sha256 TO missing_hash")
        .unwrap();
    assert!(check(&mut conn).is_err());
}

use anyhow::{Result, bail};
fn run() -> Result<()> {
    dotenvy::dotenv().ok();
    let mut conn = marreq_migrations::connect()?;
    match std::env::args().nth(1).as_deref() {
        Some("check") => marreq_migrations::check(&mut conn),
        Some("apply") => marreq_migrations::apply(&mut conn, false),
        Some("dev-init") => marreq_migrations::apply(&mut conn, true),
        Some("verify-restored") => {
            let path = std::env::var("MARREQ_RECOVERY_BUNDLE")?;
            marreq_migrations::recovery::verify_restored(std::path::Path::new(&path), &mut conn)
        }
        Some("verify-backup") => {
            let path = std::env::var("MARREQ_RECOVERY_BUNDLE")?;
            marreq_migrations::recovery::verify(std::path::Path::new(&path), &mut conn, false)
        }
        _ => bail!("usage: marreq-migrations check|apply|dev-init|verify-backup"),
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("Migration operation failed: {error}");
        std::process::exit(1);
    }
}

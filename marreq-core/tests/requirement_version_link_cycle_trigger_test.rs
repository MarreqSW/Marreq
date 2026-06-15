#[test]
fn cycle_trigger_uses_check_violation_for_client_errors() {
    let migration = include_str!(
        "../migrations/2026-06-15-000001_prevent_requirement_version_link_cycles/up.sql"
    );

    assert!(
        migration.contains("ERRCODE = '23514'"),
        "cycle trigger errors must use PostgreSQL check-violation SQLSTATE so the API maps them to a client error"
    );
    assert!(
        migration.contains("CONSTRAINT = 'requirement_version_links_no_cycles'"),
        "cycle trigger should expose a stable constraint name for cycle failures"
    );
    assert!(
        migration.contains("CONSTRAINT = 'requirement_version_links_no_self_link'"),
        "self-link trigger should expose a stable constraint name for self-link failures"
    );
    assert!(
        !migration.contains("[requirement_version_links_cycle]"),
        "cycle trigger must not raise a generic PostgreSQL exception token that maps to HTTP 500"
    );
}

#[test]
fn cycle_trigger_checks_target_ancestors_before_accepting_link() {
    let migration = include_str!(
        "../migrations/2026-06-15-000001_prevent_requirement_version_link_cycles/up.sql"
    );

    assert!(
        migration.contains("WHERE rvl.source_version_id = NEW.target_version_id"),
        "cycle detection must start at the proposed target version"
    );
    assert!(
        migration.contains("WHERE version_id = NEW.source_version_id"),
        "cycle detection must reject when the proposed source is already reachable from the target"
    );
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Regression coverage for project requirement listing enrichment.
//!
//! The production `build_requirement_list_rows` implementation should keep enrichment query counts
//! bounded by loading related verification methods, custom fields, and parent links in batches.
//! This test is intentionally database-free so it can guard the query shape without requiring a
//! seeded PostgreSQL instance.

#[test]
fn project_requirement_listing_uses_bounded_bulk_enrichment_queries() {
    let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/api/requirements.rs");
    let source = std::fs::read_to_string(&source_path)
        .expect("requirements API source should be readable from the crate manifest directory");

    let production_start = source
        .find("#[cfg(not(any(test, feature = \"test-helpers\")))]\nfn build_requirement_list_rows")
        .expect("production build_requirement_list_rows implementation should exist");
    let test_helper_start = source[production_start..]
        .find("\n#[cfg(any(test, feature = \"test-helpers\"))]")
        .map(|offset| production_start + offset)
        .expect("test-helper build_requirement_list_rows implementation should follow production implementation");
    let production_impl = &source[production_start..test_helper_start];

    assert!(
        production_impl.contains("rvvm::table")
            && production_impl.contains("verification_rows: Vec<(i32, i32)>")
            && production_impl.contains("verification_method_ids_by_requirement"),
        "project requirement listing must bulk-load verification method ids instead of querying once per requirement"
    );
    assert!(
        production_impl.contains("cfv::table")
            && production_impl.contains("cfd::table")
            && production_impl.contains("custom_fields_by_version"),
        "project requirement listing must bulk-load custom field display values by current version"
    );
    assert!(
        production_impl.contains("rvl::table")
            && production_impl.contains("source_version_id.eq_any(&version_ids)")
            && production_impl.contains("parent_ids_by_source_version"),
        "project requirement listing must bulk-load parent links by current version"
    );
    assert!(
        production_impl.matches(".load(conn.as_mut())").count() <= 4,
        "enrichment should use a bounded number of Diesel loads independent of requirement count"
    );
    assert!(
        !production_impl.contains("RequirementService::new"),
        "production enrichment should not delegate to per-requirement service calls"
    );
}

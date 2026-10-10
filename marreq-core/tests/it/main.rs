//! All integration tests in one binary: each former `tests/*.rs` file is a
//! module here, so the crate and its dependencies are linked once instead of
//! once per file. Filter with `cargo test --test it <module>`.

#[macro_use]
extern crate rocket;

mod api_applicability_integration_test;
mod api_attachments_test;
mod api_authentication_test;
mod api_cache_integration_test;
mod api_categories_integration_test;
mod api_comment_mentions_test;
mod api_constraint_violation_test;
mod api_dsm_test;
mod api_error_consistency_test;
mod api_exports_integration_test;
mod api_home_test;
mod api_imports_integration_test;
mod api_matrix_endpoint_test;
mod api_matrix_integration_test;
mod api_mcp_phase2_test;
mod api_project_archive_test;
mod api_project_bundle_files_test;
mod api_project_bundle_test;
mod api_project_edit_test;
mod api_project_scoping_test;
mod api_reqifz_test;
mod api_requirements_integration_test;
mod api_semantic_search_test;
mod api_status_integration_test;
mod api_tests_integration_test;
mod api_users_integration_test;
mod api_validation_test;
mod api_workflow_integration_test;
mod cors_test;
mod csrf_integration_test;
mod requirement_list_bulk_query_regression;
mod requirement_version_link_cycle_trigger_test;

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

pub mod applicability;
pub mod baselines;
pub mod cache;
pub mod categories;
pub mod comments;
pub mod custom_fields;
pub mod error;
pub mod matrix;
pub mod mcp;
pub mod prelude;
pub mod requirement_diff;
pub mod requirements;
pub mod semantic_search;
pub mod status;
pub mod test_cases;
pub mod test_status;
pub mod traceability;
pub mod users;

use rocket::Route;

pub fn routes() -> Vec<Route> {
    routes![
        baselines::list,
        baselines::get,
        baselines::create,
        baselines::get_requirements,
        baselines::get_traceability,
        baselines::diff_baselines,
        requirements::list,
        requirements::list_by_project,
        requirements::get,
        requirements::get_by_project,
        requirements::list_versions,
        requirements::list_versions_by_project,
        requirements::get_version,
        requirements::get_version_by_project,
        requirements::get_impacted_tests,
        requirements::set_version_approval,
        requirements::set_version_approval_by_project,
        requirements::create,
        requirements::create_by_project,
        requirements::delete,
        requirements::patch_requirement,
        requirements::patch_by_project,
        comments::list,
        comments::create,
        requirement_diff::diff_versions,
        requirement_diff::diff_versions_by_project,
        requirement_diff::diff_baseline_vs_current,
        test_cases::list,
        test_cases::get,
        test_cases::create,
        test_cases::delete,
        test_cases::update_field,
        categories::list,
        categories::get,
        categories::create,
        categories::update,
        categories::delete,
        applicability::list,
        applicability::get,
        applicability::create,
        applicability::update,
        applicability::delete,
        custom_fields::list_by_project,
        custom_fields::get,
        custom_fields::create,
        custom_fields::update,
        custom_fields::delete,
        status::list_requirement_statuses,
        status::get_requirement_status,
        status::create_requirement_status,
        status::update_requirement_status,
        status::delete_requirement_status,
        test_status::list_test_statuses,
        test_status::get_test_status,
        test_status::create_test_status,
        test_status::update_test_status,
        test_status::delete_test_status,
        users::list,
        users::get,
        users::create,
        users::delete,
        matrix::list,
        matrix::list_by_project,
        traceability::trace_up,
        traceability::trace_down,
        traceability::coverage_report,
        traceability::clear_suspect,
        cache::stats,
        cache::clear,
        cache::cleanup,
        cache::performance,
        cache::recommendations,
        cache::reset_counters,
        cache::health,
        // Semantic search endpoints
        semantic_search::semantic_search,
        semantic_search::ask,
        semantic_search::reindex,
        semantic_search::index_status,
        semantic_search::search_status,
        mcp::audit,
    ]
}

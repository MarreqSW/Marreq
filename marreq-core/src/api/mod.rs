// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

pub mod activity;
pub mod applicability;
pub mod auth;
pub mod baselines;
pub mod cache;
pub mod catalog;
pub mod categories;
pub mod comments;
pub mod custom_fields;
pub mod dashboard;
pub mod error;
pub mod exports;
pub mod groups;
pub mod guards;
pub mod idempotency;
pub mod imports;
pub mod logs;
pub mod matrix;
pub mod mcp;
pub mod members;
pub mod meta;
pub mod notifications;
pub mod oauth;
pub mod prelude;
pub mod projects;
pub mod projects_session;
pub mod requirement_diff;
pub mod requirement_version_links;
pub mod requirements;
pub mod saved_views;
pub mod semantic_search;
pub mod status;
pub mod traceability;
pub mod users;
pub mod verification_diff;
pub mod verification_methods;
pub mod verification_status;
pub mod verifications;

use rocket::Route;

/// Auditable access-policy declaration for every mounted REST route.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoutePolicy {
    Public,
    Authenticated,
    ProjectRead,
    ProjectWrite,
    ProjectApproval,
    ProjectManagement,
    Administrator,
    Internal,
}

/// Pair a policy with a set of Rocket routes (for deployment-specific mounts).
pub fn classify(policy: RoutePolicy, routes: Vec<Route>) -> Vec<(RoutePolicy, Route)> {
    routes.into_iter().map(|route| (policy, route)).collect()
}

macro_rules! policy_routes {
    ($all:ident, $policy:ident; $($route:path),+ $(,)?) => {
        $all.extend(routes![$($route),+].into_iter().map(|route| (RoutePolicy::$policy, route)));
    };
}

/// Return routes paired with their declared policy. Adding a mounted route
/// requires choosing a policy here, so authorization coverage cannot silently
/// drift from route registration.
pub fn routes_with_policies() -> Vec<(RoutePolicy, Route)> {
    let mut r = Vec::new();
    policy_routes!(r, Public;
        meta::api_root,
        meta::build_info,
        meta::deployment_info,
        meta::health,
        auth::auth_csrf,
        auth::auth_providers,
        auth::auth_external_start,
        auth::auth_external_callback,
        auth::auth_login,
        cache::health,
    );
    policy_routes!(r, Authenticated;
        auth::auth_identities,
        auth::auth_external_link_start,
        auth::auth_identity_delete,
        auth::auth_logout,
        auth::auth_change_password,
        auth::auth_me,
        dashboard::dashboard_json,
        projects_session::list_for_session,
        projects_session::project_from_path,
        catalog::get,
        notifications::list,
        notifications::unread_count,
        notifications::mark_read,
        notifications::mark_all_read,
        notifications::get_preferences,
        notifications::set_preference,
        notifications::delete_preference,
    );
    policy_routes!(r, ProjectRead;
        baselines::list,
        baselines::get,
        baselines::get_requirements,
        baselines::get_traceability,
        baselines::get_verifications,
        baselines::diff_baselines,
        requirements::list,
        requirements::list_by_project,
        requirements::get,
        requirements::get_by_project,
        activity::requirement_activity_by_project,
        requirements::list_versions,
        requirements::list_versions_by_project,
        requirements::get_version,
        requirements::get_version_by_project,
        requirements::get_impacted_tests,
        comments::list,
        comments::list_by_project,
        requirement_diff::diff_versions,
        requirement_diff::diff_versions_by_project,
        requirement_diff::diff_baseline_vs_current,
        verifications::list,
        verifications::list_by_project,
        verifications::get,
        verifications::get_by_project,
        activity::verification_activity_by_project,
        verification_methods::list_by_project,
        verification_diff::list_snapshots_by_project,
        verification_diff::diff_snapshots_by_project,
        verification_diff::diff_baseline_vs_current,
        categories::list,
        categories::get,
        applicability::list,
        applicability::get,
        custom_fields::list_by_project,
        custom_fields::get,
        status::list_requirement_statuses,
        status::get_requirement_status,
        verification_status::list_verification_statuses,
        verification_status::get_verification_status,
        matrix::list,
        matrix::list_by_project,
        matrix::get_verification_matrix,
        traceability::trace_up,
        traceability::trace_down,
        traceability::coverage_report,
        requirement_version_links::list,
        requirement_version_links::link_types,
        members::get_my_permissions,
        members::list_members,
        members::list_project_reviewers,
        saved_views::list,
        saved_views::get,
        exports::export_requirements_xlsx,
        exports::export_verifications_xlsx,
        exports::export_matrix_xlsx,
        exports::export_matrix_links_xlsx,
        exports::export_requirements_pdf,
        exports::export_report_pdf,
        exports::export_requirements_reqif,
        exports::export_baseline_reqif,
        exports::export_project_bundle,
        semantic_search::semantic_search,
        semantic_search::ask,
        semantic_search::index_status,
        semantic_search::search_status,
    );
    policy_routes!(r, ProjectWrite;
        baselines::create,
        requirements::create,
        requirements::create_by_project,
        requirements::delete,
        requirements::patch_requirement,
        requirements::patch_by_project,
        comments::create,
        comments::create_by_project,
        verifications::create,
        verifications::create_by_project,
        verifications::update_by_project,
        verifications::delete,
        verifications::update_field,
        verifications::update_field_by_project,
        verification_methods::create_by_project,
        verification_methods::update_by_project,
        verification_methods::delete_by_project,
        categories::create,
        categories::update,
        categories::delete,
        applicability::create,
        applicability::update,
        applicability::delete,
        custom_fields::create,
        custom_fields::update,
        custom_fields::delete,
        status::create_requirement_status,
        status::update_requirement_status,
        status::delete_requirement_status,
        verification_status::create_verification_status,
        verification_status::update_verification_status,
        verification_status::delete_verification_status,
        matrix::put_verification_matrix,
        traceability::clear_suspect,
        traceability::clear_suspect_by_project,
        requirement_version_links::create,
        requirement_version_links::delete,
        saved_views::create,
        saved_views::update,
        saved_views::delete,
        imports::preview_excel,
        imports::commit_excel,
        imports::commit_reqif,
        imports::import_project_bundle,
    );
    policy_routes!(r, ProjectApproval;
        requirements::set_version_approval,
        requirements::set_version_approval_by_project,
    );
    policy_routes!(r, ProjectManagement;
        members::set_member_role,
        members::remove_member,
        members::put_project_reviewers,
        projects::create,
        groups::list,
        groups::list_creatable,
        groups::get,
        groups::create,
        groups::update,
        groups::delete,
        groups::list_projects,
        groups::list_members,
        groups::set_member_role,
        groups::remove_member,
    );
    policy_routes!(r, Administrator;
        users::list,
        users::get,
        users::create,
        users::delete,
        logs::list,
        logs::export_json,
        logs::cleanup,
        cache::stats,
        cache::clear,
        cache::cleanup,
        cache::performance,
        cache::recommendations,
        cache::reset_counters,
        semantic_search::reindex,
    );
    policy_routes!(r, Internal;
        mcp::audit,
        mcp::internal_audit,
        mcp::principal,
    );
    r
}

pub fn routes() -> Vec<Route> {
    routes_with_policies()
        .into_iter()
        .map(|(_, route)| route)
        .collect()
}

/// Root-mounted handlers that are not part of `routes()` or OAuth.
pub fn root_routes_with_policies() -> Vec<(RoutePolicy, Route)> {
    let mut r = Vec::new();
    policy_routes!(r, Public;
        crate::routes::api_info::root_index,
    );
    policy_routes!(r, Internal;
        crate::fairings::csrf_denied,
    );
    r
}

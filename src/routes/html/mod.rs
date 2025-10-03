pub mod cache;
mod helpers;

pub mod admin;
pub mod applicability;
pub mod auth;
pub mod categories;
pub mod dashboard;
pub mod excel;
pub mod logs;
pub mod projects;
pub mod reports;
pub mod requirements;
pub mod tables;
pub mod tests;
pub mod users;

pub use admin::*;
pub use applicability::*;
pub use auth::*;
pub use categories::*;
pub use dashboard::*;
pub use excel::*;
pub use logs::*;
pub use projects::*;
pub use reports::*;
pub use requirements::*;
pub use tables::*;
pub use tests::*;
pub use users::*;

pub use cache::{
    cache_health_page, cache_stats_page, cleanup_cache, clear_cache, warm_cache_route,
};

pub(crate) mod prelude {
    pub(crate) use diesel::prelude::*;
    pub(crate) use rocket::form::Form;
    pub(crate) use rocket::fs::NamedFile;
    pub(crate) use rocket::http::{ContentType, Cookie, CookieJar};
    pub(crate) use rocket::response::status::NotFound;
    pub(crate) use rocket::response::{content, Redirect};
    pub(crate) use rocket::serde::json::json;
    pub(crate) use rocket::Route;
    pub(crate) use rocket::State;
    pub(crate) use rocket_dyn_templates::Template;
    pub(crate) use std::collections::{HashMap, HashSet};
    pub(crate) use std::path;

    pub(crate) use crate::app::AppState;
    pub(crate) use crate::auth::*;
    pub(crate) use crate::generators::*;
    pub(crate) use crate::helper_functions::*;
    pub(crate) use crate::html::*;
    pub(crate) use crate::logger::{LogCtx, Logger};
    pub(crate) use crate::models::*;
    pub(crate) use crate::repository::{
        LookupRepository, MatrixRepository, ProjectMembersRepository, ProjectsRepository,
        RequirementsRepository, TestsRepository, UserRepository,
    };
}

use rocket::Route;

pub fn routes() -> Vec<Route> {
    routes![
        auth::login_page,
        auth::login,
        auth::logout,
        auth::change_password_page,
        auth::change_password,
        dashboard::index,
        requirements::show_requirements,
        requirements::show_requirement_id,
        tests::show_tests,
        tests::show_test_id,
        dashboard::show_status,
        requirements::new_requirement,
        requirements::get_edit_requirement,
        requirements::post_edit_requirement,
        requirements::post_requirement,
        requirements::delete_requirement_route,
        tests::delete_test_route,
        tests::new_test,
        tests::get_edit_test,
        tests::post_edit_test,
        tests::post_test,
        tests::get_matrix,
        tests::get_matrix_xls,
        tests::get_requirements_xls,
        tests::get_tests_xls,
        users::new_user,
        users::post_user,
        users::show_users,
        users::show_user_id,
        users::edit_user,
        users::post_edit_user,
        categories::show_categories,
        categories::new_category,
        categories::post_category,
        categories::get_edit_category,
        categories::post_edit_category,
        categories::delete_category_route,
        requirements::show_requirements_tree,
        tables::show_requirements_table,
        tables::show_tests_table,
        reports::show_reports,
        reports::generate_pdf_report,
        projects::show_projects,
        projects::show_project_id,
        projects::new_project,
        projects::post_project,
        projects::get_edit_project,
        projects::post_edit_project,
        projects::delete_project_route,
        excel::import_excel_page,
        excel::upload_excel_file,
        excel::process_excel_import,
        admin::admin_dashboard,
        admin::admin_users_page,
        admin::admin_backup_page,
        admin::generate_backup,
        logs::show_logs,
        logs::show_entity_logs,
        logs::export_logs,
        logs::export_entity_logs,
        logs::cleanup_logs,
        logs::log_analytics,
        cache::cache_stats_page,
        cache::clear_cache,
        cache::cleanup_cache,
        cache::cache_health_page,
        cache::warm_cache_route,
    ]
}

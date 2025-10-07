pub mod cache;
mod helpers;

pub mod admin;
pub mod auth;
pub mod dashboard;
pub mod excel;
pub mod logs;
pub mod project;
pub mod projects;
pub mod tables;
pub mod user;

pub use admin::*;
pub use auth::*;
pub use excel::*;
pub use logs::*;
pub use projects::*;
pub use tables::*;
pub use user::*;

pub use cache::{
    cache_health_page, cache_stats_page, cleanup_cache, clear_cache, warm_cache_route,
};

pub(crate) mod prelude {
    pub(crate) use rocket::form::Form;
    pub(crate) use rocket::fs::NamedFile;
    pub(crate) use rocket::http::{ContentType, CookieJar};
    pub(crate) use rocket::response::{content, Redirect};
    pub(crate) use rocket::serde::json::json;
    pub(crate) use rocket::Route;
    pub(crate) use rocket::State;
    pub(crate) use rocket_dyn_templates::Template;

    pub(crate) use crate::app::AppState;
    pub(crate) use crate::auth::*;
    pub(crate) use crate::helper_functions::*;
    pub(crate) use crate::html::*;
    pub(crate) use crate::logger::{LogCtx, Logger};
    pub(crate) use crate::models::*;
    pub(crate) use crate::repository::{
        LookupRepository, ProjectsRepository, UserRepository,
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
        dashboard::show_status,
        tables::show_requirements_table,
        tables::show_tests_table,
        projects::show_projects,
        projects::new_project,
        projects::post_project,
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

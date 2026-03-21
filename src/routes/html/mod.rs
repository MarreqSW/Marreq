// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

pub mod cache;
mod helpers;

pub mod admin;
pub mod auth;
pub mod dashboard;
pub mod excel;
pub mod groups;
pub mod logs;
pub mod project;
pub mod projects;
pub mod user;

pub use admin::*;
pub use auth::*;
pub use excel::*;
pub use groups::*;
pub use logs::*;
pub use projects::*;
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
    #[allow(unused_imports)]
    pub(crate) use crate::helper_functions::*;
    pub(crate) use crate::html::*;
    pub(crate) use crate::logger::{LogCtx, Logger};
    pub(crate) use crate::models::*;
    pub(crate) use crate::repository::{LookupRepository, ProjectsRepository, UserRepository};
}

use rocket::Route;

pub fn routes() -> Vec<Route> {
    let mut routes = routes![
        auth::login_page,
        auth::login,
        auth::logout,
        auth::change_password_page,
        auth::change_password,
        dashboard::index,
        dashboard::show_status,
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
        dashboard::error_page,
    ];
    routes.extend(project::routes());
    routes.extend(groups::routes());
    routes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_non_empty() {
        let r = routes();
        assert!(!r.is_empty());
    }

    #[test]
    fn routes_include_login_and_logs() {
        let r = routes();
        let uris: Vec<String> = r.iter().map(|route| route.uri.to_string()).collect();
        assert!(
            uris.iter().any(|u| u.contains("login")),
            "expected login route, got {:?}",
            uris
        );
        assert!(
            uris.iter().any(|u| u.contains("logs")),
            "expected logs route, got {:?}",
            uris
        );
    }

    #[test]
    fn routes_include_import_excel() {
        let r = routes();
        let uris: Vec<String> = r.iter().map(|route| route.uri.to_string()).collect();
        assert!(
            uris.iter().any(|u| u.contains("import_excel")),
            "expected import_excel route, got {:?}",
            uris
        );
    }
}

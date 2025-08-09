#[macro_use]
extern crate rocket;
extern crate diesel;
use rocket::fs::{relative, FileServer};
use rocket_dyn_templates::Template;
pub mod bbdd;
pub mod cache;
pub mod cached_functions;
pub mod db;
pub mod db_operations;
pub mod generators;
pub mod helper_functions;
pub mod html;
pub mod importers;
pub mod logger;
pub mod models;
pub mod routes;
pub mod schema;

use crate::html::cors::*;
use crate::routes::routes_api::*;
use crate::routes::routes_html::*;
use crate::routes::cache_routes::*;

#[rocket_sync_db_pools::database("my_db")]
pub struct MyDbConn(rocket_sync_db_pools::diesel::PgConnection);
#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    
    // Warm up the cache on startup
    crate::cache::warm_cache();
    
    // Start background cache maintenance
    crate::cache::start_cache_maintenance();
    
    let _rocket = rocket::build()
        .mount(
            "/",
            routes![
                login_page,
                login,
                logout,
                change_password_page,
                change_password,
                index,
                show_requirements,
                show_requirement_id,
                show_tests,
                show_test_id,
                show_status,
                new_requirement,
                get_edit_requirement,
                post_edit_requirement,
                post_requirement,
                delete_requirement_route,
                new_test,
                get_edit_test,
                post_edit_test,
                post_test,
                get_matrix,
                get_matrix_xls,
                get_requirements_xls,
                get_tests_xls,
                new_user,
                post_user,
                show_users,
                show_user_id,
                edit_user,
                post_edit_user,
                show_categories,
                new_category,
                post_category,
                get_edit_category,
                post_edit_category,
                delete_category_route,
                show_applicability,
                new_applicability,
                post_applicability,
                get_edit_applicability,
                post_edit_applicability,
                delete_applicability_route,
                show_requirements_tree,
                show_reports,
                generate_pdf_report,
                show_projects,
                show_project_id,
                new_project,
                post_project,
                get_edit_project,
                post_edit_project,
                delete_project_route,
                import_excel_page,
                upload_excel_file,
                process_excel_import,
                admin_dashboard,
                admin_users_page,
                admin_backup_page,
                generate_backup,
                show_logs,
                show_entity_logs,
                export_logs,
                export_entity_logs,
                cleanup_logs,
                log_analytics,
                admin_cache_stats,
                admin_clear_cache,
                admin_warm_cache,
                admin_warm_project_cache,
                admin_cache_health,
            ],
        )
        .mount(
            "/api/v1",
            routes![
                api_get_requirement,
                api_get_requirement_by_id,
                api_post_requirement,
                api_delete_requirement_by_id,
                api_get_status,
                api_post_status,
                api_get_categories,
                api_get_test,
                api_get_test_by_id,
                api_post_test,
                api_delete_test_by_id,
                api_get_matrix,
                api_get_users,
                api_get_users_by_id,
                api_get_category_by_id,
                api_post_category,
                api_put_category,
                api_delete_category_by_id,
                api_get_applicability,
                api_get_applicability_by_id,
                api_post_applicability,
                api_put_applicability,
                api_delete_applicability_by_id,
                api_cache_stats,
                api_clear_cache,
                api_cleanup_cache,
                api_cache_health,
                api_cache_performance,
                api_cache_recommendations,
                api_reset_cache_counters,
            ],
        )
        .mount("/static", FileServer::from(relative!("src/html/static")))
        .attach(CorsFairing)
        .attach(Template::fairing())
        .attach(MyDbConn::fairing())
        .launch()
        .await?;

    Ok(())
}


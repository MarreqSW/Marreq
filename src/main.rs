#[macro_use]
extern crate rocket;
extern crate diesel;
use rocket::fs::{relative, FileServer};
use rocket_dyn_templates::Template;
pub mod auth;
pub mod bbdd;
pub mod cache;
pub mod cached_functions;
pub mod errors;
pub mod generators;
pub mod helper_functions;
pub mod html;
pub mod importers;
pub mod logger;
pub mod models;
pub mod routes;
pub mod schema;
pub mod repository;
pub mod services;
pub mod validation;

use crate::html::cors::*;
use crate::routes::routes_html::*;
use crate::routes::api::*;
use crate::routes::api::docs::*;
use crate::services::*;

#[rocket_sync_db_pools::database("my_db")]
pub struct MyDbConn(rocket_sync_db_pools::diesel::PgConnection);
#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    
    // Warm up the cache on startup
    crate::cache::warm_cache();
    
    // Start background cache maintenance
    crate::cache::start_cache_maintenance();
    
    let _rocket = rocket::build()
        // Manage service instances
        .manage(RequirementService::new())
        .manage(TestService::new())
        .manage(CategoryService::new())
        .manage(ApplicabilityService::new())
        .manage(UserService::new())
        .manage(ProjectService::new())
        .manage(StatusService::new())
        .manage(MatrixService::new())
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
                show_requirements_table,
                show_requirement_id,
                show_tests,
                show_tests_table,
                show_test_id,
                show_status,
                new_requirement,
                get_edit_requirement,
                post_edit_requirement,
                post_requirement,
                delete_requirement_route,
                delete_test_route,
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

            ],
        )
        .mount(
            "/api/v1",
            routes![
                // Health and version endpoints
                health_check,
                api_version,
                get_openapi_spec,
                get_api_docs,
                
                // Requirements endpoints
                requirements::get_requirements,
                requirements::get_requirements_by_project,
                requirements::get_requirement_by_id,
                requirements::create_requirement,
                requirements::update_requirement,
                requirements::update_requirement_field,
                requirements::delete_requirement,
                requirements::get_requirements_by_category,
                requirements::get_requirements_by_status,
                
                // Tests endpoints
                tests::get_tests,
                tests::get_tests_by_project,
                tests::get_test_by_id,
                tests::create_test,
                tests::update_test,
                tests::update_test_field,
                tests::delete_test,
                tests::get_tests_by_status,
                tests::get_tests_by_parent,
                
                // Categories endpoints
                categories::get_categories,
                categories::get_category_by_id,
                categories::create_category,
                categories::update_category,
                categories::delete_category,
                
                // Applicability endpoints
                applicability::get_applicability,
                applicability::get_applicability_by_id,
                applicability::create_applicability,
                applicability::update_applicability,
                applicability::delete_applicability,
                
                // Users endpoints
                users::get_users,
                users::get_user_by_id,
                users::create_user,
                users::delete_user,
                
                // Projects endpoints
                projects::get_projects,
                projects::get_project_by_id,
                projects::create_project,
                projects::update_project,
                projects::delete_project,
                
                // Status endpoints
                status::get_requirement_status,
                status::get_test_status,
                status::create_requirement_status,
                status::create_test_status,
                
                // Matrix endpoints
                matrix::get_matrix,
                matrix::get_matrix_by_project,
                matrix::create_matrix_link,
                matrix::delete_matrix_link,
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


#[macro_use]
extern crate rocket;
extern crate diesel;
use rocket::fs::{relative, FileServer};
use rocket_dyn_templates::Template;
use rocket_sync_db_pools::database;

pub mod bbdd;
pub mod generators;
pub mod helper_functions;
pub mod html;
pub mod models;
pub mod routes;
pub mod schema;

use crate::html::cors::*;
use crate::routes::routes_api::*;
use crate::routes::routes_html::*;

#[database("my_db")]
pub struct DbConn(rocket_sync_db_pools::diesel::PgConnection);

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _rocket = rocket::build()
        .mount(
            "/",
            routes![
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
                new_test,
                get_edit_test,
                post_edit_test,
                post_test,
                get_matrix,
                get_matrix_xls,
                new_user,
                post_user,
                show_user_id,
                edit_user,
            ],
        )
        .mount(
            "/api",
            routes![
                api_get_reqs,
                api_get_reqs_by_id,
                api_get_status,
                api_get_categories,
                api_get_tests,
                api_get_tests_by_id,
                api_get_matrix,
                api_get_users,
                api_get_users_by_id,
                api_post_requirement,
                api_post_test,
            ],
        )
        .mount("/static", FileServer::from(relative!("src/html/static")))
        .attach(CorsFairing)
        .attach(Template::fairing())
        .attach(DbConn::fairing())
        .launch()
        .await?;

    Ok(())
}

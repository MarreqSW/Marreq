#[macro_use] 
extern crate rocket;
extern crate diesel;
use rocket::fs::{FileServer, relative};
use rocket_dyn_templates::Template;
use rocket_sync_db_pools::database;

pub mod bbdd;
pub mod html;
pub mod routes;
pub mod models;
pub mod schema;
pub mod generators;
pub mod helper_functions;

use crate::routes::*;
use crate::html::cors::*;

#[database("my_db")]
pub struct DbConn(rocket_sync_db_pools::diesel::PgConnection);

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
  
    let _rocket = rocket::build()
        .mount("/", routes![ 
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
            post_test,
            get_matrix,
            get_matrix_xls,
            ])
        .mount("/api/v1/", routes![
            api_get_index,
            api_get_reqs, 
            api_get_reqs_by_id,
            api_get_status, 
            api_get_categories,
            api_get_tests,
            api_get_tests_by_id,
            api_get_matrix,
            api_post_requirement,
            api_post_test,
            api_post_status,
            ])
        .mount("/static", FileServer::from(relative!("src/html/static")))
        .attach(CorsFairing) 
        .attach(Template::fairing())
        .attach(DbConn::fairing())
        .launch()
        .await?;

    Ok(())
}


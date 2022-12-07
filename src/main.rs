//use rocket_sync_db_pools::{database};
#[macro_use] 
extern crate rocket;
extern crate diesel;
use rocket::fs::{FileServer, relative};
use rocket_dyn_templates::{Template};
//use diesel::PgConnection;

pub mod bbdd;
pub mod html;
pub mod routes;
pub mod models;
pub mod schema;
pub mod generators;
pub mod helper_functions;

use crate::routes::routes::*;
use crate::html::cors::*;

//#[database("my_db")]
//struct Db(PgConnection);

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
/* 
    let connection = &mut establish_connection();

    let a = NewRequirement {
        author : "Charles".to_owned(),
        description : "New description".to_owned(),
        author_email : "bla@example.com".to_owned(),
        title : "a brand new requirement".to_owned(),
        link: "http://ieec.cat".to_owned(),
        category: 3,
        current_status: 5,
    };
    create_requirement (connection, &a).unwrap();
*/    

    let _rocket = rocket::build()
        .mount("/", routes![ 
            index,
            show_requirements,
            show_requirement_id,
            show_tests,
            show_test_id,
            show_status,
            //edit_requirement,
            get_matrix,
            get_matrix_xls,
            ])
        .mount("/api", routes![
            api_get_reqs, 
            api_get_reqs_by_id,
            api_get_status, 
            api_get_categories,
            api_get_tests,
            api_get_tests_by_id,
            api_get_matrix,
            api_post_requirement,
            api_post_test,
            ])
        .mount("/static", FileServer::from(relative!("src/html/static")))
        .attach(CorsFairing) 
        .attach(Template::fairing())
        //.attach(DbConn::fairing())
        .launch()
        .await?;

    Ok(())
}

/* 
fn main() {
    println!("Requirements manager");

    let connection = &mut establish_connection();
    let results = requirements
        .filter(author.eq("Màrius"))
        .limit(5)
        .load::<Requirement>(connection)
        .expect("Error loading requirements");

    println!("Displaying {} requirements", results.len());
    for post in results {
        println!("{:?}", post.id);
        println!("-----------\n");
        println!("{:?}", post.title);
    }

    let a = NewRequirement {
        author : "Màrius".to_owned(),
        description : "description long long long text".to_owned(),
        author_email : "bla@example.com".to_owned(),
        title : "a brand new requirement".to_owned(),
        link: "http://example.com".to_owned(),
    };

    create_requirement (connection, &a).unwrap();
    
    update_requirement (connection, 7).unwrap();

} */





use rocket::fs::{relative, FileServer};
use rocket::{Build, Rocket};
use rocket_dyn_templates::Template;

use crate::api;
use crate::html::cors::CorsFairing;
use crate::routes::{catchers, html};

#[rocket_sync_db_pools::database("my_db")]
pub struct MyDbConn(rocket_sync_db_pools::diesel::PgConnection);

pub fn build() -> Rocket<Build> {
    rocket::build()
        .mount("/", html::routes())
        .mount("/api/v1", api::routes())
        .register("/", catchers![catchers::unauthorized, catchers::forbidden])
        .mount("/static", FileServer::from(relative!("src/html/static")))
        .attach(CorsFairing)
        .attach(Template::fairing())
        .attach(MyDbConn::fairing())
}

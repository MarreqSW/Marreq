use std::sync::{Arc, RwLock};
use rocket::{Build, Rocket};
use crate::repository::{DieselRepo, DieselCachedRepo};

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<RwLock<DieselCachedRepo>>,
}

#[rocket_sync_db_pools::database("my_db")]
pub struct MyDbConn(rocket_sync_db_pools::diesel::PgConnection);

pub fn build() -> Rocket<Build> {
    let cached = DieselCachedRepo::new(DieselRepo::new(), 5 * 60);
    let repo = Arc::new(RwLock::new(cached));

    rocket::build()
        .manage(AppState { repo })
        .mount("/", crate::routes::html::routes())
        .mount("/api", crate::api::routes())
        .register("/", catchers![crate::routes::catchers::unauthorized, crate::routes::catchers::forbidden])
        .mount("/static", rocket::fs::FileServer::from(rocket::fs::relative!("src/html/static")))
        .attach(crate::html::cors::CorsFairing)
        .attach(rocket_dyn_templates::Template::fairing())
        .attach(crate::app::MyDbConn::fairing())
}

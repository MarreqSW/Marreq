#[macro_use]
extern crate rocket;
extern crate diesel;

pub mod api;
pub mod app;
pub mod auth;
pub mod generators;
pub mod helper_functions;
pub mod html;
pub mod importers;
pub mod logger;
pub mod models;
pub mod repository;
pub mod routes;
pub mod schema;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    use crate::repository::DieselCachedRepo;

    DieselCachedRepo::write().warm_cache();
    DieselCachedRepo::read().cache().start_cache_maintenance();

    app::build().launch().await?;

    Ok(())
}

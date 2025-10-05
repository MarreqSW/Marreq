#[macro_use]
extern crate rocket;
extern crate diesel;

pub mod api;
pub mod app;
pub mod auth;
pub mod errors;
pub mod generators;
pub mod helper_functions;
pub mod html;
pub mod importers;
pub mod logger;
pub mod models;
pub mod repository;
pub mod routes;
pub mod schema;
pub mod services;
pub mod validation;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    app::build().launch().await?;

    Ok(())
}

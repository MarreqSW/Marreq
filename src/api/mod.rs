pub mod applicability;
pub mod cache;
pub mod categories;
pub mod error;
pub mod matrix;
pub mod prelude;
pub mod requirements;
pub mod status;
pub mod tests;
pub mod users;

use rocket::Route;

pub fn routes() -> Vec<Route> {
    routes![
        requirements::list,
        requirements::get,
        requirements::create,
        requirements::delete,
        requirements::patch_requirement,
        tests::list,
        tests::get,
        tests::create,
        tests::delete,
        tests::update_field,
        categories::list,
        categories::get,
        categories::create,
        categories::update,
        categories::delete,
        applicability::list,
        applicability::get,
        applicability::create,
        applicability::update,
        applicability::delete,
        status::list,
        status::get,
        status::create,
        users::list,
        users::get,
        users::create,
        users::delete,
        matrix::list,
        cache::stats,
        cache::clear,
        cache::cleanup,
        cache::performance,
        cache::recommendations,
        cache::reset_counters,
        cache::health,
    ]
}

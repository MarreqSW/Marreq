pub mod applicability;
pub mod baselines;
pub mod cache;
pub mod categories;
pub mod error;
pub mod matrix;
pub mod prelude;
pub mod requirements;
pub mod semantic_search;
pub mod status;
pub mod test_cases;
pub mod traceability;
pub mod users;

use rocket::Route;

pub fn routes() -> Vec<Route> {
    routes![
        baselines::list,
        baselines::get,
        baselines::create,
        baselines::get_requirements,
        baselines::get_traceability,
        requirements::list,
        requirements::get,
        requirements::list_versions,
        requirements::get_version,
        requirements::set_version_approval,
        requirements::create,
        requirements::delete,
        requirements::patch_requirement,
        test_cases::list,
        test_cases::get,
        test_cases::create,
        test_cases::delete,
        test_cases::update_field,
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
        status::list_requirement_statuses,
        status::get_requirement_status,
        status::create_requirement_status,
        users::list,
        users::get,
        users::create,
        users::delete,
        matrix::list,
        traceability::clear_suspect,
        cache::stats,
        cache::clear,
        cache::cleanup,
        cache::performance,
        cache::recommendations,
        cache::reset_counters,
        cache::health,
        // Semantic search endpoints
        semantic_search::semantic_search,
        semantic_search::ask,
        semantic_search::reindex,
        semantic_search::index_status,
        semantic_search::search_status,
    ]
}

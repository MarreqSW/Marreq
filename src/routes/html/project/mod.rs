mod applicability;
mod baselines;
mod categories;
mod matrix;
mod members;
mod project_routes;
mod reports;
pub mod reqif;
pub mod requirements;
mod test_cases;
#[cfg(any(test, feature = "test-helpers"))]
pub mod test_helpers;
mod verification;

use super::helpers;
use super::projects;
pub(crate) mod prelude {
    pub(crate) use rocket::form::Form;
    pub(crate) use rocket::fs::NamedFile;
    pub(crate) use rocket::http::{ContentType, Cookie, CookieJar};
    pub(crate) use rocket::response::Redirect;
    pub(crate) use rocket::serde::json::json;
    pub(crate) use rocket::Route;
    pub(crate) use rocket::State;
    pub(crate) use rocket_dyn_templates::Template;
    pub(crate) use std::collections::{HashMap, HashSet};
    pub(crate) use std::path;

    pub(crate) use crate::app::AppState;
    pub(crate) use crate::auth::*;
    pub(crate) use crate::generators::*;
    pub(crate) use crate::helper_functions::*;
    pub(crate) use crate::models::*;
    pub(crate) use crate::repository::{
        LookupRepository, MatrixRepository, ProjectMembersRepository, ProjectsRepository,
        RequirementsRepository, TestsCaseRepository, UserRepository,
    };
}

use rocket::Route;

pub fn routes() -> Vec<Route> {
    let mut routes = Vec::new();
    routes.extend(applicability::routes());
    routes.extend(baselines::routes());
    routes.extend(categories::routes());
    routes.extend(matrix::routes());
    routes.extend(members::routes());
    routes.extend(reports::routes());
    routes.extend(reqif::routes());
    routes.extend(requirements::routes());
    routes.extend(test_cases::routes());
    routes.extend(verification::routes());
    routes.extend(project_routes::routes());
    routes
}

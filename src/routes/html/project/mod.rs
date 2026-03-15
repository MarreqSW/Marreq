// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

mod applicability;
mod baselines;
mod categories;
mod custom_fields;
mod document_import;
mod matrix;
mod members;
mod project_routes;
mod reports;
pub mod reqif;
mod requirement_statuses;
pub mod requirements;
#[cfg(any(test, feature = "test-helpers"))]
pub mod test_helpers;
mod verification;
mod verification_statuses;
mod verifications;

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
        RequirementsRepository, UserRepository, VerificationsRepository,
    };
}

use rocket::Route;

pub fn routes() -> Vec<Route> {
    let mut routes = Vec::new();
    routes.extend(applicability::routes());
    routes.extend(baselines::routes());
    routes.extend(categories::routes());
    routes.extend(custom_fields::routes());
    routes.extend(document_import::routes());
    routes.extend(requirement_statuses::routes());
    routes.extend(verification_statuses::routes());
    routes.extend(matrix::routes());
    routes.extend(members::routes());
    routes.extend(reports::routes());
    routes.extend(reqif::routes());
    routes.extend(requirements::routes());
    routes.extend(verifications::routes());
    routes.extend(verification::routes());
    routes.extend(project_routes::routes());
    routes
}

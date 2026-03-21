// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Dashboard payload for the SPA home view (parity with HTML `dashboard::index` + decorated project cards).

use rocket::http::Cookie;
use rocket::http::CookieJar;
use rocket::serde::json::{json, Json};

use crate::api::guards::OptionalSessionUser;
use crate::api::prelude::*;
use crate::auth::csrf::get_or_create_csrf_token;
use crate::routes::html::helpers::{
    decorate_projects_for_listing, resolve_selected_project_slug,
};
use crate::services::ProjectService;

/// Full dashboard context for the post-login SPA (same data shape as template `index`).
#[get("/dashboard")]
pub fn dashboard_json(
    opt: OptionalSessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let user = opt
        .0
        .ok_or_else(|| ApiError::Unauthorized("not authenticated".into()))?;

    let projects = ProjectService::new(state.inner())
        .get_by_user_id(user.id)
        .unwrap_or_default();

    let mut selected_project_id = cookies
        .get("selected_project_id")
        .and_then(|cookie| cookie.value().parse::<i32>().ok());

    if selected_project_id.is_none() && !projects.is_empty() {
        selected_project_id = Some(projects[0].id);
        cookies.add(Cookie::new(
            "selected_project_id",
            projects[0].id.to_string(),
        ));
    }

    let decorated_projects = decorate_projects_for_listing(state, &user, &projects);
    let selected_project_slug =
        resolve_selected_project_slug(selected_project_id, &projects);
    let csrf_token = get_or_create_csrf_token(cookies);

    Ok(Json(json!({
        "user": user,
        "projects": decorated_projects,
        "projects_count": decorated_projects.len(),
        "selected_project_id": selected_project_id,
        "selected_project_slug": selected_project_slug,
        "csrf_token": csrf_token,
    })))
}

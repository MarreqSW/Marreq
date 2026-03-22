// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Session-scoped project listing for SPA clients (replaces HTML `/projects` data).

use rocket::serde::json::{json, Json, Value};

use crate::api::guards::OptionalSessionUser;
use crate::api::prelude::*;
use crate::models::Project;
use crate::repository::ProjectMembersRepository;
use crate::services::project_service::ProjectService;

/// List projects visible to the current user (admin: all; others: memberships).
#[get("/projects")]
pub fn list_for_session(
    opt: OptionalSessionUser,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<Project>>> {
    let user = opt
        .0
        .ok_or_else(|| ApiError::Unauthorized("not authenticated".into()))?;
    let service = ProjectService::new(state.inner());
    let projects = if user.is_admin {
        service.list_all()?
    } else {
        service.get_by_user_id(user.id)?
    };
    Ok(Json(projects))
}

/// Resolve `/{namespace}/{project_slug}` URL segments to a project (SPA deep links).
/// Mounted at `/api/project-from-path/...` (not under `/api/projects/...`) so Rocket does not
/// report route collisions with `/api/projects/<project_id>/...` handlers.
#[get("/project-from-path/<namespace>/<slug>")]
pub fn project_from_path(
    opt: OptionalSessionUser,
    namespace: &str,
    slug: &str,
    state: &State<AppState>,
) -> ApiResult<Json<Value>> {
    let user = opt
        .0
        .ok_or_else(|| ApiError::Unauthorized("not authenticated".into()))?;

    let service = ProjectService::new(state.inner());
    let project = service
        .get_by_namespace_and_slug(namespace, slug)
        .map_err(|_| ApiError::NotFound("project not found".into()))?;

    if !user.is_admin {
        let repo = state.repo_read();
        let memberships = repo.get_projects_for_user(user.id).unwrap_or_default();
        if !memberships
            .iter()
            .any(|membership| membership.project_id == project.id)
        {
            return Err(ApiError::Forbidden("no access to this project".into()));
        }
    }

    let route_slug = format!("{namespace}/{slug}");
    Ok(Json(json!({
        "id": project.id,
        "name": project.name,
        "description": project.description,
        "slug": project.slug,
        "route_slug": route_slug,
    })))
}

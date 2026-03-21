// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Session-scoped project listing for SPA clients (replaces HTML `/projects` data).

use rocket::serde::json::Json;

use crate::api::prelude::*;
use crate::api::guards::OptionalSessionUser;
use crate::models::Project;
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

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! REST API for project creation.

use rocket::serde::Deserialize;

use crate::api::prelude::*;
use crate::auth::guards::ApiUserOrBearer;
use crate::models::NewProject;
use crate::services::project_service::ProjectService;
use crate::status_enums::ProjectStatus;

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub group_id: Option<i32>,
}

/// POST /api/projects — create a new project.
#[post("/projects", data = "<body>")]
pub async fn create(
    auth: ApiUserOrBearer,
    state: &State<AppState>,
    body: Json<CreateProjectRequest>,
) -> ApiResult<Json<Value>> {
    let user = auth.user();
    let payload = NewProject {
        name: body.name.clone(),
        description: body.description.clone(),
        owner_id: Some(user.id),
        status: ProjectStatus::Active,
        group_id: body.group_id,
    };
    let service = ProjectService::new(state.inner());
    let id = service.create(user, payload).map_err(ApiError::from)?;
    let project = service.get_by_id(id).map_err(ApiError::from)?;
    Ok(Json(json!({
        "id": project.id,
        "name": project.name,
        "slug": project.slug,
        "description": project.description,
        "group_id": project.group_id,
        "status": "ok",
    })))
}

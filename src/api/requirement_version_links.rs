// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

//! API for requirement version links (multi-parent, typed traceability).

use rocket::serde::{Deserialize, Serialize};

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::models::RequirementVersionLink;
use crate::repository::{RequirementVersionLinksRepository, RequirementsRepository};
use crate::services::RequirementService;

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct CreateRequirementVersionLinkRequest {
    pub source_version_id: i32,
    pub target_version_id: i32,
    pub link_type: String,
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// Create a requirement version link. Project-scoped; both versions must belong to the project.
#[post("/projects/<project_id>/requirement-version-links", data = "<body>")]
pub async fn create(
    _access: ProjectAccessOrBearer,
    project_id: i32,
    body: Json<CreateRequirementVersionLinkRequest>,
    state: &State<AppState>,
) -> ApiResult<Json<RequirementVersionLink>> {
    let b = body.into_inner();
    let service = RequirementService::new(state.inner());
    let repo = state.inner().repo_read();
    let source_ver = repo.get_requirement_version_by_id(b.source_version_id)?;
    let source_req = repo.get_requirement_by_id(source_ver.requirement_id)?;
    if source_req.project_id != project_id {
        return Err(ApiError::NotFound("source version not in project".into()));
    }
    let target_ver = repo.get_requirement_version_by_id(b.target_version_id)?;
    let target_req = repo.get_requirement_by_id(target_ver.requirement_id)?;
    if target_req.project_id != project_id {
        return Err(ApiError::NotFound("target version not in project".into()));
    }
    drop(repo);
    let link = service.create_requirement_version_link(
        b.source_version_id,
        b.target_version_id,
        &b.link_type,
        project_id,
        b.rationale,
        b.metadata,
    )?;
    Ok(Json(link))
}

/// List requirement version links for a project. Query: source_version_id, target_version_id, link_type (all optional).
#[get("/projects/<project_id>/requirement-version-links?<source_version_id>&<target_version_id>&<link_type>")]
pub async fn list(
    _access: ProjectAccessOrBearer,
    project_id: i32,
    source_version_id: Option<i32>,
    target_version_id: Option<i32>,
    link_type: Option<String>,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<RequirementVersionLink>>> {
    let repo = state.inner().repo_read();
    let links = repo.list_links_by_project(
        project_id,
        source_version_id,
        target_version_id,
        link_type.as_deref(),
    )?;
    Ok(Json(links))
}

/// Delete a requirement version link by id. Link must belong to the project.
#[delete("/projects/<project_id>/requirement-version-links/<link_id>")]
pub async fn delete(
    _access: ProjectAccessOrBearer,
    project_id: i32,
    link_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<RequirementVersionLink>> {
    let service = RequirementService::new(state.inner());
    let link = service.delete_requirement_version_link(project_id, link_id)?;
    Ok(Json(link))
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct LinkTypeInfo {
    pub link_types: Vec<&'static str>,
}

/// List allowed link types. Project-scoped (any project access).
#[get("/projects/<_project_id>/requirement-version-links/link-types")]
pub async fn link_types(
    _access: ProjectAccessOrBearer,
    _project_id: i32,
) -> ApiResult<Json<LinkTypeInfo>> {
    Ok(Json(LinkTypeInfo {
        link_types: crate::services::requirement_service::REQUIREMENT_VERSION_LINK_TYPES.to_vec(),
    }))
}

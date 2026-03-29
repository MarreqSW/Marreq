// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Project-scoped audit activity for requirements and verifications (simple view / SPA).

use rocket::serde::Serialize;

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::models::EntityType;
use crate::permissions::Permission;
use crate::services::log_service::{change_summary, log_change_details, ChangeDetail, LogService};
use crate::services::{RequirementService, VerificationService};

/// One audit log row formatted for the entity detail UI.
#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct EntityActivityItem {
    pub log_id: i32,
    pub user_id: i32,
    pub username: String,
    pub action_type: String,
    pub summary: String,
    pub description: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub changes: Vec<ChangeDetail>,
}

fn map_logs(logs: Vec<crate::services::log_service::LogWithUser>) -> Vec<EntityActivityItem> {
    logs.into_iter()
        .map(|lw| {
            let summary = change_summary(&lw.log);
            let changes = log_change_details(&lw.log);
            EntityActivityItem {
                log_id: lw.log.log_id,
                user_id: lw.log.user_id,
                username: lw.username,
                action_type: lw.log.action_type.clone(),
                summary,
                description: lw.log.description.clone(),
                created_at: lw.log.created_at,
                changes,
            }
        })
        .collect()
}

#[get("/projects/<project_id>/requirements/<id>/activity")]
pub async fn requirement_activity_by_project(
    access: ProjectAccessOrBearer,
    project_id: i32,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<EntityActivityItem>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let req_service = RequirementService::new(state.inner());
    let requirement = req_service.get_by_id(id)?;
    if requirement.project_id != project_id {
        return Err(ApiError::NotFound("requirement not in project".into()));
    }
    let log_service = LogService::new(state.inner());
    let etype = EntityType::Requirement.to_string();
    let logs = log_service
        .entity_logs(&etype, id)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(map_logs(logs)))
}

#[get("/projects/<project_id>/verifications/<id>/activity")]
pub async fn verification_activity_by_project(
    access: ProjectAccessOrBearer,
    project_id: i32,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<EntityActivityItem>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let v_service = VerificationService::new(state.inner());
    let v = v_service.get_by_id(id)?;
    if v.project_id != project_id {
        return Err(ApiError::NotFound("verification not in project".into()));
    }
    let log_service = LogService::new(state.inner());
    let etype = EntityType::Verification.to_string();
    let logs = log_service
        .entity_logs(&etype, id)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(map_logs(logs)))
}

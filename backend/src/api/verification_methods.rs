// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! List verification methods per project (for requirement/verification forms).

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::models::VerificationMethod;
use crate::repository::LookupRepository;

#[get("/projects/<project_id>/verification-methods")]
pub async fn list_by_project(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<VerificationMethod>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let methods = state
        .repo_read()
        .get_verification_methods_by_project(project_id)?;
    Ok(Json(methods))
}

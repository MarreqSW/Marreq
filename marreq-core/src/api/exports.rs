// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::http::Header;
use rocket::Responder;

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::generators::excel;

const XLSX_CONTENT_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

#[derive(Responder)]
#[response(status = 200)]
pub struct XlsxDownload {
    bytes: Vec<u8>,
    content_type: Header<'static>,
    disposition: Header<'static>,
}

impl XlsxDownload {
    fn new(bytes: Vec<u8>, filename: String) -> Self {
        Self {
            bytes,
            content_type: Header::new("Content-Type", XLSX_CONTENT_TYPE),
            disposition: Header::new(
                "Content-Disposition",
                format!("attachment; filename=\"{filename}\""),
            ),
        }
    }
}

#[get("/projects/<project_id>/exports/requirements.xlsx")]
pub async fn export_requirements_xlsx(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<XlsxDownload> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let bytes = excel::requirements_workbook_with_repo(&*state.repo_read(), project_id)
        .map_err(|e| ApiError::Internal(format!("could not build workbook: {e}")))?;
    Ok(XlsxDownload::new(
        bytes,
        format!("requirements-project-{project_id}.xlsx"),
    ))
}

#[get("/projects/<project_id>/exports/verifications.xlsx")]
pub async fn export_verifications_xlsx(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<XlsxDownload> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let bytes = excel::verifications_workbook_with_repo(&*state.repo_read(), project_id)
        .map_err(|e| ApiError::Internal(format!("could not build workbook: {e}")))?;
    Ok(XlsxDownload::new(
        bytes,
        format!("verifications-project-{project_id}.xlsx"),
    ))
}

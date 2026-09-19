// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::http::Header;
use rocket::Responder;

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::generators::{excel, reports, GeneratorError};

const XLSX_CONTENT_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
const PDF_CONTENT_TYPE: &str = "application/pdf";

#[derive(Responder)]
#[response(status = 200)]
pub struct FileDownload {
    bytes: Vec<u8>,
    content_type: Header<'static>,
    disposition: Header<'static>,
}

impl FileDownload {
    fn new(bytes: Vec<u8>, content_type: &'static str, filename: String) -> Self {
        Self {
            bytes,
            content_type: Header::new("Content-Type", content_type),
            disposition: Header::new(
                "Content-Disposition",
                format!("attachment; filename=\"{filename}\""),
            ),
        }
    }

    fn xlsx(bytes: Vec<u8>, filename: String) -> Self {
        Self::new(bytes, XLSX_CONTENT_TYPE, filename)
    }

    fn pdf(bytes: Vec<u8>, filename: String) -> Self {
        Self::new(bytes, PDF_CONTENT_TYPE, filename)
    }
}

fn build_failed(e: GeneratorError) -> ApiError {
    ApiError::Internal(format!("could not build export: {e}"))
}

#[get("/projects/<project_id>/exports/requirements.xlsx")]
pub async fn export_requirements_xlsx(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<FileDownload> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let bytes = excel::requirements_workbook_with_repo(&*state.repo_read(), project_id)
        .map_err(build_failed)?;
    Ok(FileDownload::xlsx(
        bytes,
        format!("requirements-project-{project_id}.xlsx"),
    ))
}

#[get("/projects/<project_id>/exports/verifications.xlsx")]
pub async fn export_verifications_xlsx(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<FileDownload> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let bytes = excel::verifications_workbook_with_repo(&*state.repo_read(), project_id)
        .map_err(build_failed)?;
    Ok(FileDownload::xlsx(
        bytes,
        format!("verifications-project-{project_id}.xlsx"),
    ))
}

#[get("/projects/<project_id>/exports/matrix.xlsx")]
pub async fn export_matrix_xlsx(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<FileDownload> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let bytes =
        excel::matrix_workbook_with_repo(&*state.repo_read(), project_id).map_err(build_failed)?;
    Ok(FileDownload::xlsx(
        bytes,
        format!("matrix-project-{project_id}.xlsx"),
    ))
}

#[get("/projects/<project_id>/exports/requirements.pdf")]
pub async fn export_requirements_pdf(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<FileDownload> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let bytes = reports::requirements_pdf_with_repo(&*state.repo_read(), project_id)
        .map_err(build_failed)?;
    Ok(FileDownload::pdf(
        bytes,
        format!("requirements-project-{project_id}.pdf"),
    ))
}

#[get("/projects/<project_id>/exports/report.pdf")]
pub async fn export_report_pdf(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<FileDownload> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let bytes = reports::project_report_pdf_with_repo(&*state.repo_read(), project_id)
        .map_err(build_failed)?;
    Ok(FileDownload::pdf(
        bytes,
        format!("report-project-{project_id}.pdf"),
    ))
}

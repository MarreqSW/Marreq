// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::http::Header;
use rocket::Responder;

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::generators::{excel, reports, GeneratorError};
use crate::importers::project_bundle;
use crate::repository::ProjectsRepository;
use crate::services::ReqIFService;

const XLSX_CONTENT_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
const PDF_CONTENT_TYPE: &str = "application/pdf";
const XML_CONTENT_TYPE: &str = "application/xml";

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

    fn xml(bytes: Vec<u8>, filename: String) -> Self {
        Self::new(bytes, XML_CONTENT_TYPE, filename)
    }

    fn json(bytes: Vec<u8>, filename: String) -> Self {
        Self::new(bytes, "application/json", filename)
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

#[get("/projects/<project_id>/exports/requirements.reqif")]
pub async fn export_requirements_reqif(
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
    let xml = ReqIFService::new(state.inner())
        .export_project(project_id)
        .map_err(ApiError::from)?;
    Ok(FileDownload::xml(
        xml.into_bytes(),
        format!("requirements-project-{project_id}.reqif"),
    ))
}

#[get("/projects/<project_id>/exports/baselines/<filename>")]
pub async fn export_baseline_reqif(
    access: ProjectAccessOrBearer,
    project_id: i32,
    filename: &str,
    state: &State<AppState>,
) -> ApiResult<FileDownload> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let baseline_id = filename
        .strip_suffix(".reqif")
        .and_then(|id| id.parse::<i32>().ok())
        .ok_or_else(|| ApiError::NotFound("baseline ReqIF export not found".into()))?;
    let xml = ReqIFService::new(state.inner())
        .export_baseline(project_id, baseline_id)
        .map_err(ApiError::from)?;
    Ok(FileDownload::xml(
        xml.into_bytes(),
        format!("baseline-{baseline_id}-project-{project_id}.reqif"),
    ))
}

#[get("/projects/<project_id>/exports/bundle.json")]
pub async fn export_project_bundle(
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
    let project = state
        .repo_read()
        .get_project_by_id(project_id)
        .map_err(ApiError::from)?;
    let bundle =
        project_bundle::export_bundle(state.inner(), project_id).map_err(ApiError::from)?;
    let bytes = serde_json::to_vec_pretty(&bundle)
        .map_err(|e| ApiError::Internal(format!("could not serialize project bundle: {e}")))?;
    Ok(FileDownload::json(
        bytes,
        format!("project-{}-bundle.json", project.slug),
    ))
}

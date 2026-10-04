// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::Responder;
use rocket::fs::NamedFile;
use rocket::http::Header;

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::generators::{GeneratorError, excel, reports};
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
    pub(crate) fn new(bytes: Vec<u8>, content_type: &'static str, filename: String) -> Self {
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

/// `GET /api/projects/<id>/exports/dsm.xlsx` — the Dependency Structure Matrix
/// (same query parameters as `GET /api/projects/<id>/dsm`).
#[get("/projects/<project_id>/exports/dsm.xlsx?<link_types>&<category_id>&<root_id>&<order>")]
#[allow(clippy::too_many_arguments)]
pub async fn export_dsm_xlsx(
    access: ProjectAccessOrBearer,
    project_id: i32,
    link_types: Option<&str>,
    category_id: Option<i32>,
    root_id: Option<i32>,
    order: Option<&str>,
    state: &State<AppState>,
) -> ApiResult<FileDownload> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let options = crate::api::dsm::dsm_options(link_types, category_id, root_id, order)?;
    let dsm = crate::api::dsm::load(state, project_id, &options)?;
    let bytes = excel::dsm_workbook(&dsm).map_err(build_failed)?;
    Ok(FileDownload::xlsx(
        bytes,
        format!("dsm-project-{project_id}.xlsx"),
    ))
}

#[get("/projects/<project_id>/exports/matrix-links.xlsx")]
pub async fn export_matrix_links_xlsx(
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
    let bytes = excel::matrix_links_workbook_with_repo(&*state.repo_read(), project_id)
        .map_err(build_failed)?;
    Ok(FileDownload::xlsx(
        bytes,
        format!("matrix-links-project-{project_id}.xlsx"),
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
    // The built-in Traceability & coverage report (issue #354).
    let definition = crate::api::reports::default_definition(
        state,
        access.user(),
        project_id,
        crate::reports::definition::ReportType::Coverage,
    )?;
    let owned = state.inner().clone();
    let report = rocket::tokio::task::spawn_blocking(move || {
        crate::reports::generate(
            &owned,
            project_id,
            definition,
            crate::reports::ReportFormat::Pdf,
        )
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .map_err(crate::api::reports::report_error)?;
    Ok(FileDownload::pdf(
        report.bytes,
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

/// A ZIP archive (ReqIFZ, project bundle) streamed from a temp file that is already unlinked.
#[derive(Responder)]
#[response(status = 200, content_type = "application/zip")]
pub struct ArchiveDownload {
    file: NamedFile,
    disposition: Header<'static>,
}

/// Build the archive off the async workers, open it and unlink it (the open
/// handle keeps it readable while the response streams).
async fn reqifz_download(
    input: crate::services::reqif_service::ExportInput,
    storage: std::sync::Arc<crate::storage::AttachmentStorage>,
    filename: String,
) -> ApiResult<ArchiveDownload> {
    let document_name = filename.trim_end_matches(".reqifz").to_string() + ".reqif";
    let path = rocket::tokio::task::spawn_blocking(move || {
        crate::services::reqifz_service::export_archive(&input, &storage, &document_name)
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .map_err(|e| ApiError::Internal(format!("could not build the ReqIFZ archive: {e}")))?;
    stream_archive(path, filename).await
}

fn require_storage(
    storage: Option<crate::api::attachments::Storage>,
) -> ApiResult<std::sync::Arc<crate::storage::AttachmentStorage>> {
    storage.map(|s| s.0).ok_or_else(|| {
        ApiError::Internal(
            "attachment storage is not configured; exports with files are unavailable".into(),
        )
    })
}

/// Open an archive built in a temp file, unlink it and stream it.
async fn stream_archive(path: std::path::PathBuf, filename: String) -> ApiResult<ArchiveDownload> {
    let opened = NamedFile::open(&path).await;
    let _ = std::fs::remove_file(&path);
    let file =
        opened.map_err(|e| ApiError::Internal(format!("could not read the archive: {e}")))?;
    Ok(ArchiveDownload {
        file,
        disposition: Header::new(
            "Content-Disposition",
            format!("attachment; filename=\"{filename}\""),
        ),
    })
}

/// `GET /projects/<id>/exports/requirements.reqifz`: ReqIF plus the
/// requirements' attachment files (issue #343).
#[get("/projects/<project_id>/exports/requirements.reqifz")]
pub async fn export_requirements_reqifz(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
    storage: Option<crate::api::attachments::Storage>,
) -> ApiResult<ArchiveDownload> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let storage = require_storage(storage)?;
    let input = ReqIFService::new(state.inner())
        .collect_project_export(project_id)
        .map_err(ApiError::from)?;
    reqifz_download(
        input,
        storage,
        format!("requirements-project-{project_id}.reqifz"),
    )
    .await
}

/// A baseline export: plain ReqIF or a ReqIFZ archive.
#[derive(Responder)]
pub enum BaselineExport {
    Xml(FileDownload),
    Archive(ArchiveDownload),
}

/// `GET /projects/<id>/exports/baselines/<n>.reqif` or `<n>.reqifz` (with the
/// files the baseline recorded, including ones deleted since).
#[get("/projects/<project_id>/exports/baselines/<filename>")]
pub async fn export_baseline_reqif(
    access: ProjectAccessOrBearer,
    project_id: i32,
    filename: &str,
    state: &State<AppState>,
    storage: Option<crate::api::attachments::Storage>,
) -> ApiResult<BaselineExport> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let not_found = || ApiError::NotFound("baseline ReqIF export not found".into());
    let (stem, archive) = match filename.strip_suffix(".reqifz") {
        Some(stem) => (stem, true),
        None => (
            filename.strip_suffix(".reqif").ok_or_else(not_found)?,
            false,
        ),
    };
    let baseline_id = stem.parse::<i32>().map_err(|_| not_found())?;
    let service = ReqIFService::new(state.inner());
    if archive {
        let storage = require_storage(storage)?;
        let input = service
            .collect_baseline_export(project_id, baseline_id)
            .map_err(ApiError::from)?;
        let name = format!("baseline-{baseline_id}-project-{project_id}.reqifz");
        return Ok(BaselineExport::Archive(
            reqifz_download(input, storage, name).await?,
        ));
    }
    let xml = service
        .export_baseline(project_id, baseline_id)
        .map_err(ApiError::from)?;
    Ok(BaselineExport::Xml(FileDownload::xml(
        xml.into_bytes(),
        format!("baseline-{baseline_id}-project-{project_id}.reqif"),
    )))
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

/// `GET /projects/<id>/exports/bundle.zip`: the project bundle plus the
/// attachment files of its requirements and verifications (issue #341).
#[get("/projects/<project_id>/exports/bundle.zip")]
pub async fn export_project_bundle_zip(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
    storage: Option<crate::api::attachments::Storage>,
) -> ApiResult<ArchiveDownload> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let storage = require_storage(storage)?;
    let project = state
        .repo_read()
        .get_project_by_id(project_id)
        .map_err(ApiError::from)?;
    let app_state = state.inner().clone();
    let path = rocket::tokio::task::spawn_blocking(move || {
        project_bundle::export_bundle_archive(&app_state, &storage, project_id)
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .map_err(bundle_archive_error)?;
    stream_archive(path, format!("project-{}-bundle.zip", project.slug)).await
}

pub(crate) fn bundle_archive_error(error: project_bundle::BundleArchiveError) -> ApiError {
    match error {
        project_bundle::BundleArchiveError::Invalid(msg) => ApiError::BadRequest(msg),
        project_bundle::BundleArchiveError::Repo(e) => ApiError::from(e),
        project_bundle::BundleArchiveError::Io(e) => ApiError::Internal(e.to_string()),
    }
}

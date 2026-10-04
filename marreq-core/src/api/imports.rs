// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::api::prelude::*;
use crate::auth::guards::{ApiUserOrBearer, ProjectAccessOrBearer};
use crate::importers::{ColumnMapping, ExcelImporter, ImportConfig, ValueMapping, project_bundle};
use crate::repository::{GroupsRepository, LookupRepository, ProjectMembersRepository};
use crate::reqif::import::ImportConfig as ReqifImportConfig;
use crate::services::ReqIFService;

const MAX_UPLOAD_BYTES: u64 = 20 * 1024 * 1024;

#[derive(FromForm)]
pub struct ExcelPreviewForm<'r> {
    file: TempFile<'r>,
}

#[derive(FromForm)]
pub struct ExcelCommitForm<'r> {
    file: TempFile<'r>,
    import_type: String,
    column_mappings: String,
    value_mappings: Option<String>,
}

#[derive(FromForm)]
pub struct BundleImportForm<'r> {
    file: TempFile<'r>,
    group_id: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ExcelPreviewResponse {
    import_type: String,
    columns: Vec<crate::importers::ExcelColumn>,
    sample_rows: Vec<Vec<String>>,
    row_count: usize,
    available_fields: ExcelAvailableFields,
    unique_values: HashMap<String, Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ExcelAvailableFields {
    requirements: Vec<String>,
    tests: Vec<String>,
    matrix: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
struct MappingItem {
    excel_column: String,
    target_field: String,
}

#[post("/projects/<project_id>/imports/excel/preview", data = "<form>")]
pub async fn preview_excel(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
    mut form: Form<ExcelPreviewForm<'_>>,
) -> ApiResult<Json<ExcelPreviewResponse>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    let (filename, bytes) = read_upload(&mut form.file).await?;
    let importer = ExcelImporter::from_bytes(&filename, &bytes)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok(Json(ExcelPreviewResponse {
        import_type: importer.import_type.clone(),
        columns: importer.columns.clone(),
        sample_rows: importer.sample_rows(5),
        row_count: importer.data.len(),
        available_fields: ExcelAvailableFields {
            requirements: ExcelImporter::fields_for("requirements"),
            tests: ExcelImporter::fields_for("tests"),
            matrix: ExcelImporter::fields_for("matrix"),
        },
        unique_values: importer.unique_values_by_column(),
    }))
}

#[post("/projects/<project_id>/imports/excel", data = "<form>")]
pub async fn commit_excel(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
    mut form: Form<ExcelCommitForm<'_>>,
) -> ApiResult<Value> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;

    let import_type = form.import_type.trim().to_string();
    if import_type != "requirements" && import_type != "tests" && import_type != "matrix" {
        return Err(ApiError::BadRequest(
            "import_type must be 'requirements', 'tests', or 'matrix'".into(),
        ));
    }

    let mappings: Vec<MappingItem> = serde_json::from_str(form.column_mappings.trim())
        .map_err(|e| ApiError::BadRequest(format!("invalid column_mappings JSON: {e}")))?;
    let column_mappings: Vec<ColumnMapping> = mappings
        .into_iter()
        .filter(|m| !m.target_field.is_empty() && m.target_field != "skip")
        .map(|m| ColumnMapping {
            excel_column: m.excel_column,
            target_field: m.target_field,
        })
        .collect();
    let value_mappings: Vec<ValueMapping> = match form.value_mappings.as_deref() {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw.trim())
            .map_err(|e| ApiError::BadRequest(format!("invalid value_mappings JSON: {e}")))?,
        _ => Vec::new(),
    };
    if import_type != "matrix" {
        validate_value_mappings(state, project_id, &import_type, &value_mappings)?;
    }

    if import_type == "matrix" {
        let has_req = column_mappings
            .iter()
            .any(|m| m.target_field == "requirement_reference_code");
        let has_ver = column_mappings
            .iter()
            .any(|m| m.target_field == "verification_reference_code");
        if !has_req || !has_ver {
            return Err(ApiError::BadRequest(
                "map columns to requirement_reference_code and verification_reference_code".into(),
            ));
        }
    } else {
        let required = if import_type == "requirements" {
            "title"
        } else {
            "name"
        };
        if !column_mappings.iter().any(|m| m.target_field == required) {
            return Err(ApiError::BadRequest(format!(
                "map at least one column to {required}"
            )));
        }
    }

    let (filename, bytes) = read_upload(&mut form.file).await?;
    let importer = ExcelImporter::from_bytes(&filename, &bytes)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let config = ImportConfig {
        import_type,
        column_mappings,
        value_mappings,
        project_id,
    };
    let result = importer
        .import_data(state.inner(), access.user(), &config)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok(json!({
        "success": result.success,
        "message": result.message,
        "imported_count": result.imported_count,
        "errors": result.errors,
        "imported_requirement_ids": result.imported_requirement_ids,
    }))
}

#[derive(FromForm)]
pub struct ReqifImportForm<'r> {
    file: TempFile<'r>,
}

/// Upload persisted to a private temp file (removed on drop).
struct TempUpload(std::path::PathBuf);

impl Drop for TempUpload {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

async fn persist_upload(file: &mut TempFile<'_>) -> ApiResult<TempUpload> {
    let suffix: u64 = rand::random();
    let path = std::env::temp_dir().join(format!("marreq-import-{suffix:016x}"));
    file.move_copy_to(&path)
        .await
        .map_err(|e| ApiError::BadRequest(format!("could not store upload: {e}")))?;
    Ok(TempUpload(path))
}

fn reqif_result_json(
    result: &crate::reqif::ImportResult,
    documents: &[String],
    imported_attachment_count: usize,
) -> Value {
    json!({
        "success": result.success,
        "message": result.message,
        "imported_count": result.imported_count,
        "created_link_count": result.created_link_count,
        "imported_attachment_count": imported_attachment_count,
        "documents": documents,
        "errors": result.errors,
        "warnings": result.warnings,
        "imported_requirement_ids": result.imported_requirement_ids,
    })
}

/// `POST /projects/<id>/imports/reqif`: a `.reqif`/`.xml` document, or a
/// ReqIFZ archive (detected by its ZIP signature) whose documents are all
/// imported and whose referenced files become attachments (issue #343).
#[post("/projects/<project_id>/imports/reqif", data = "<form>")]
pub async fn commit_reqif(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
    storage: Option<crate::api::attachments::Storage>,
    mut form: Form<ReqifImportForm<'_>>,
) -> ApiResult<Value> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    let size = form.file.len();
    if size == 0 {
        return Err(ApiError::BadRequest("empty file".into()));
    }
    let named_reqifz = form
        .file
        .raw_name()
        .map(|n| {
            n.dangerous_unsafe_unsanitized_raw()
                .as_str()
                .to_ascii_lowercase()
        })
        .is_some_and(|n| n.ends_with(".reqifz"));
    let upload = persist_upload(&mut form.file).await?;
    let mut head = [0u8; 4];
    let head_len = std::fs::File::open(&upload.0)
        .and_then(|mut f| std::io::Read::read(&mut f, &mut head))
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let is_zip = head_len == 4 && (head == *b"PK\x03\x04" || head == *b"PK\x05\x06");
    let config = reqif_import_config(state, project_id, access.user().id)?;

    if !is_zip {
        if named_reqifz {
            return Err(ApiError::BadRequest(
                "the .reqifz file is not a ZIP archive".into(),
            ));
        }
        if size > MAX_UPLOAD_BYTES {
            return Err(ApiError::BadRequest("file is larger than 20 MiB".into()));
        }
        let bytes = std::fs::read(&upload.0).map_err(|e| ApiError::Internal(e.to_string()))?;
        let result = ReqIFService::new(state.inner())
            .import_into_project(&bytes, &config, access.user())
            .map_err(ApiError::BadRequest)?;
        return Ok(reqif_result_json(&result, &[], 0));
    }

    let Some(crate::api::attachments::Storage(storage)) = storage else {
        return Err(ApiError::Internal(
            "attachment storage is not configured; ReqIFZ import is unavailable".into(),
        ));
    };
    if size > storage.config.max_reqifz_bytes {
        return Err(ApiError::PayloadTooLarge(format!(
            "the archive is {}; ReqIFZ imports can be at most {}",
            crate::services::attachment_service::format_mib(size as i64),
            crate::services::attachment_service::format_mib(storage.config.max_reqifz_bytes as i64)
        )));
    }
    let app_state = state.inner().clone();
    let user = access.user().clone();
    let imported = rocket::tokio::task::spawn_blocking(move || {
        let outcome = crate::services::reqifz_service::import_archive(
            &app_state, &storage, &user, &config, &upload.0,
        );
        drop(upload);
        outcome
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .map_err(ApiError::BadRequest)?;
    Ok(reqif_result_json(
        &imported.result,
        &imported.documents,
        imported.imported_attachment_count,
    ))
}

fn reqif_import_config(
    state: &State<AppState>,
    project_id: i32,
    user_id: i32,
) -> ApiResult<ReqifImportConfig> {
    let repo = state.repo_read();
    let default_status_id = repo
        .get_requirement_status_by_project(project_id)?
        .into_iter()
        .next()
        .map(|item| item.id)
        .ok_or_else(|| {
            ApiError::BadRequest(
                "project has no requirement statuses to use as import default".into(),
            )
        })?;
    let default_category_id = repo
        .get_categories_by_project(project_id)?
        .into_iter()
        .next()
        .map(|item| item.id)
        .ok_or_else(|| {
            ApiError::BadRequest("project has no categories to use as import default".into())
        })?;
    let default_applicability_id = repo
        .get_applicability_by_project(project_id)?
        .into_iter()
        .next()
        .map(|item| item.id)
        .ok_or_else(|| {
            ApiError::BadRequest(
                "project has no applicability values to use as import default".into(),
            )
        })?;
    let default_verification_method_id = repo
        .get_verification_methods_by_project(project_id)?
        .into_iter()
        .next()
        .map(|item| item.id)
        .ok_or_else(|| {
            ApiError::BadRequest(
                "project has no verification methods to use as import default".into(),
            )
        })?;
    Ok(ReqifImportConfig {
        project_id,
        default_status_id,
        default_category_id,
        default_applicability_id,
        default_verification_method_id,
        author_id: user_id,
        reviewer_id: user_id,
    })
}

fn validate_value_mappings(
    state: &State<AppState>,
    project_id: i32,
    import_type: &str,
    mappings: &[ValueMapping],
) -> ApiResult<()> {
    let repo = state.repo_read();
    let categories = repo
        .get_categories_by_project(project_id)?
        .into_iter()
        .map(|item| item.id)
        .collect::<HashSet<_>>();
    let applicability = repo
        .get_applicability_by_project(project_id)?
        .into_iter()
        .map(|item| item.id)
        .collect::<HashSet<_>>();
    let methods = repo
        .get_verification_methods_by_project(project_id)?
        .into_iter()
        .map(|item| item.id)
        .collect::<HashSet<_>>();
    let statuses = if import_type == "tests" {
        repo.get_verification_status_by_project(project_id)?
            .into_iter()
            .map(|item| item.id)
            .collect::<HashSet<_>>()
    } else {
        repo.get_requirement_status_by_project(project_id)?
            .into_iter()
            .map(|item| item.id)
            .collect::<HashSet<_>>()
    };
    let users = repo
        .get_members_by_project(project_id)?
        .into_iter()
        .map(|item| item.user_id)
        .collect::<HashSet<_>>();

    for mapping in mappings {
        let valid = match mapping.target_field.as_str() {
            "category_id" if import_type == "requirements" => {
                categories.contains(&mapping.target_id)
            }
            "applicability_id" if import_type == "requirements" => {
                applicability.contains(&mapping.target_id)
            }
            "verification_method_id" if import_type == "requirements" => {
                methods.contains(&mapping.target_id)
            }
            "status_id" => statuses.contains(&mapping.target_id),
            "author_id" | "reviewer_id" if import_type == "requirements" => {
                users.contains(&mapping.target_id)
            }
            _ => false,
        };
        if !valid {
            return Err(ApiError::BadRequest(format!(
                "invalid value mapping for {}: target {} is not available in this project",
                mapping.target_field, mapping.target_id
            )));
        }
    }
    Ok(())
}

async fn read_upload(file: &mut TempFile<'_>) -> ApiResult<(String, Vec<u8>)> {
    if file.len() == 0 {
        return Err(ApiError::BadRequest("empty file".into()));
    }
    if file.len() > MAX_UPLOAD_BYTES {
        return Err(ApiError::BadRequest("file is larger than 20 MiB".into()));
    }
    let filename = file.name().unwrap_or("upload.csv").to_string();
    let path = std::env::temp_dir().join(format!(
        "marreq-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    file.persist_to(&path)
        .await
        .map_err(|e| ApiError::BadRequest(format!("could not store upload: {e}")))?;
    let bytes = std::fs::read(&path).map_err(|e| ApiError::Internal(e.to_string()))?;
    let _ = std::fs::remove_file(&path);
    Ok((filename, bytes))
}

/// POST /api/projects/imports/bundle — create a new project from a bundle:
/// `bundle.json`, or a `bundle.zip` (detected by its ZIP signature) whose
/// attachment files are imported within the new project's quota; files that do
/// not fit are skipped with a warning (issue #341).
#[post("/projects/imports/bundle", data = "<form>")]
pub async fn import_project_bundle(
    auth: ApiUserOrBearer,
    state: &State<AppState>,
    storage: Option<crate::api::attachments::Storage>,
    mut form: Form<BundleImportForm<'_>>,
) -> ApiResult<Json<project_bundle::BundleImportResult>> {
    let user = auth.user();
    if let Some(group_id) = form.group_id {
        state
            .repo_read()
            .get_group_by_id(group_id)
            .map_err(ApiError::from)?;
        require_group_permission(state, user, group_id, GroupPermission::ManageProjects)?;
    }
    let size = form.file.len();
    if size == 0 {
        return Err(ApiError::BadRequest("empty file".into()));
    }
    let upload = persist_upload(&mut form.file).await?;
    let mut head = [0u8; 4];
    let head_len = std::fs::File::open(&upload.0)
        .and_then(|mut f| std::io::Read::read(&mut f, &mut head))
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let is_zip = head_len == 4 && (head == *b"PK\x03\x04" || head == *b"PK\x05\x06");

    if !is_zip {
        if size > MAX_UPLOAD_BYTES {
            return Err(ApiError::BadRequest("file is larger than 20 MiB".into()));
        }
        let bytes = std::fs::read(&upload.0).map_err(|e| ApiError::Internal(e.to_string()))?;
        let bundle = project_bundle::parse_bundle(&bytes).map_err(ApiError::BadRequest)?;
        let result = project_bundle::import_bundle(state.inner(), user, bundle, form.group_id)
            .map_err(ApiError::from)?;
        return Ok(Json(result));
    }

    let storage = storage.map(|s| s.0);
    let max_bytes = storage
        .as_ref()
        .map(|s| s.config.max_reqifz_bytes)
        .unwrap_or_else(|| crate::storage::AttachmentsConfig::default().max_reqifz_bytes);
    if size > max_bytes {
        return Err(ApiError::PayloadTooLarge(format!(
            "the archive is {}; bundle archives can be at most {}",
            crate::services::attachment_service::format_mib(size as i64),
            crate::services::attachment_service::format_mib(max_bytes as i64)
        )));
    }
    let app_state = state.inner().clone();
    let actor = user.clone();
    let group_id = form.group_id;
    let result = rocket::tokio::task::spawn_blocking(move || {
        let outcome = project_bundle::import_bundle_archive(
            &app_state,
            storage.as_ref(),
            &actor,
            &upload.0,
            group_id,
        );
        drop(upload);
        outcome
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .map_err(crate::api::exports::bundle_archive_error)?;
    Ok(Json(result))
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::api::prelude::*;
use crate::auth::guards::{ApiUserOrBearer, ProjectAccessOrBearer};
use crate::importers::{project_bundle, ColumnMapping, ExcelImporter, ImportConfig, ValueMapping};
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

#[post("/projects/<project_id>/imports/reqif", data = "<form>")]
pub async fn commit_reqif(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
    mut form: Form<ReqifImportForm<'_>>,
) -> ApiResult<Value> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    let (filename, bytes) = read_upload(&mut form.file).await?;
    reject_reqifz(&filename, &bytes)?;
    let config = reqif_import_config(state, project_id, access.user().id)?;
    let result = ReqIFService::new(state.inner())
        .import_into_project(&bytes, &config, access.user())
        .map_err(ApiError::BadRequest)?;
    Ok(json!({
        "success": result.success,
        "message": result.message,
        "imported_count": result.imported_count,
        "created_link_count": result.created_link_count,
        "errors": result.errors,
        "warnings": result.warnings,
        "imported_requirement_ids": result.imported_requirement_ids,
    }))
}

fn reject_reqifz(filename: &str, bytes: &[u8]) -> ApiResult<()> {
    let lower = filename.to_ascii_lowercase();
    let zip_magic = bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06");
    if lower.ends_with(".reqifz") || zip_magic {
        return Err(ApiError::BadRequest(
            "ReqIFZ archives are not supported; upload a .reqif or .xml file".into(),
        ));
    }
    Ok(())
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

/// POST /api/projects/imports/bundle — create a new project from a JSON snapshot.
#[post("/projects/imports/bundle", data = "<form>")]
pub async fn import_project_bundle(
    auth: ApiUserOrBearer,
    state: &State<AppState>,
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
    let (_filename, bytes) = read_upload(&mut form.file).await?;
    let bundle = project_bundle::parse_bundle(&bytes).map_err(ApiError::BadRequest)?;
    let result = project_bundle::import_bundle(state.inner(), user, bundle, form.group_id)
        .map_err(ApiError::from)?;
    Ok(Json(result))
}

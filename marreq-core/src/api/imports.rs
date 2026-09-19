// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::importers::{ColumnMapping, ExcelImporter, ImportConfig, ValueMapping};
use crate::repository::{LookupRepository, ProjectMembersRepository};

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
    if import_type != "requirements" && import_type != "tests" {
        return Err(ApiError::BadRequest(
            "import_type must be 'requirements' or 'tests'".into(),
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
    validate_value_mappings(state, project_id, &import_type, &value_mappings)?;

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

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::app::{AppState, DieselCachedRepo};
use crate::authorization::{
    require_project_reviewer_unless_requirement_create_status_is_draft_like,
    require_project_reviewer_unless_verification_create_status_is_initial,
};
use crate::models::{NewRequirement, NewVerification, User};
use crate::repository::{
    LookupRepository, MatrixRepository, ProjectMembersRepository, RequirementsRepository,
    UserRepository, VerificationsRepository,
};
use crate::services::{MatrixService, RequirementService, VerificationService};
use anyhow::{anyhow, Result};
use calamine::{open_workbook_auto_from_rs, Data, Reader};
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExcelColumn {
    pub index: usize,
    pub name: String,
    pub sample_value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ColumnMapping {
    pub excel_column: String,
    pub target_field: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValueMapping {
    pub target_field: String,
    pub source_value: String,
    pub target_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportConfig {
    pub import_type: String, // "requirements", "tests", or "matrix"
    pub column_mappings: Vec<ColumnMapping>,
    #[serde(default)]
    pub value_mappings: Vec<ValueMapping>,
    pub project_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub message: String,
    pub imported_count: usize,
    pub errors: Vec<String>,
    /// IDs of imported requirements (for semantic search indexing)
    #[serde(default)]
    pub imported_requirement_ids: Vec<i32>,
}

#[derive(Debug)]
pub struct ExcelImporter {
    pub columns: Vec<ExcelColumn>,
    pub data: Vec<Vec<String>>,
    pub import_type: String,
}

struct CatalogDefaults {
    category_id: i32,
    applicability_id: i32,
    req_status_id: i32,
    ver_status_id: i32,
    verification_method_id: Option<i32>,
}

struct RowImportContext<'a> {
    state: &'a AppState<DieselCachedRepo>,
    actor: &'a User,
    value_mappings: &'a [ValueMapping],
    project_id: i32,
    defaults: &'a CatalogDefaults,
}

impl ExcelImporter {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let bytes = std::fs::read(path_ref)
            .map_err(|e| anyhow!("failed to read {}: {e}", path_ref.display()))?;
        let name = path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("upload");
        Self::from_bytes(name, &bytes)
    }

    pub fn from_bytes(filename: &str, bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() {
            return Err(anyhow!("uploaded file is empty"));
        }
        let extension = Path::new(filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        match extension.as_str() {
            "csv" | "txt" => Self::from_csv_bytes(bytes),
            "xlsx" | "xls" | "xlsm" => Self::from_spreadsheet_bytes(bytes),
            "" => {
                if looks_like_zip(bytes) {
                    Self::from_spreadsheet_bytes(bytes)
                } else {
                    Self::from_csv_bytes(bytes)
                }
            }
            other => Err(anyhow!(
                "unsupported file type '.{other}'; use .xlsx or .csv"
            )),
        }
    }

    fn from_spreadsheet_bytes(bytes: &[u8]) -> Result<Self> {
        let cursor = Cursor::new(bytes.to_vec());
        let mut workbook = open_workbook_auto_from_rs(cursor)
            .map_err(|e| anyhow!("failed to read spreadsheet: {e}"))?;
        let sheet_name = workbook
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("spreadsheet has no sheets"))?;
        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|e| anyhow!("Failed to read sheet `{sheet_name}`: {e}"))?;

        let mut columns = Vec::new();
        let mut data = Vec::new();
        let mut is_first_row = true;

        for row in range.rows() {
            if is_first_row {
                for (i, cell) in row.iter().enumerate() {
                    columns.push(ExcelColumn {
                        index: i,
                        name: cell_text(cell),
                        sample_value: String::new(),
                    });
                }
                is_first_row = false;
                continue;
            }

            if row.iter().all(|cell| cell_text(cell).trim().is_empty()) {
                continue;
            }

            if data.is_empty() {
                for (i, cell) in row.iter().enumerate() {
                    if i < columns.len() {
                        columns[i].sample_value = cell_text(cell);
                    }
                }
            }

            data.push(row.iter().map(cell_text).collect());
        }

        if columns.is_empty() {
            return Err(anyhow!("spreadsheet has no header row"));
        }

        Ok(ExcelImporter {
            import_type: guess_import_type(&columns),
            columns,
            data,
        })
    }

    fn from_csv_bytes(bytes: &[u8]) -> Result<Self> {
        let mut reader = ReaderBuilder::new()
            .flexible(true)
            .from_reader(Cursor::new(bytes));
        let headers = reader.headers()?.clone();

        let mut columns = headers
            .iter()
            .enumerate()
            .map(|(index, name)| ExcelColumn {
                index,
                name: name.to_string(),
                sample_value: String::new(),
            })
            .collect::<Vec<_>>();

        if columns.is_empty() {
            return Err(anyhow!("CSV has no header row"));
        }

        let mut data = Vec::new();

        for result in reader.records() {
            let record = result?;
            if record.iter().all(|cell| cell.trim().is_empty()) {
                continue;
            }

            if data.is_empty() {
                for (i, cell) in record.iter().enumerate() {
                    if i < columns.len() {
                        columns[i].sample_value = cell.to_string();
                    }
                }
            }

            data.push(record.iter().map(|cell| cell.to_string()).collect());
        }

        Ok(ExcelImporter {
            import_type: guess_import_type(&columns),
            columns,
            data,
        })
    }

    pub fn fields_for(import_type: &str) -> Vec<String> {
        match import_type {
            "requirements" => vec![
                "title".to_string(),
                "description".to_string(),
                "reference_code".to_string(),
                "category_id".to_string(),
                "applicability_id".to_string(),
                "status_id".to_string(),
                "verification_method_id".to_string(),
                "author_id".to_string(),
                "reviewer_id".to_string(),
                "parent_id".to_string(),
                "justification".to_string(),
            ],
            "tests" => vec![
                "reference_code".to_string(),
                "name".to_string(),
                "description".to_string(),
                "status_id".to_string(),
                "source".to_string(),
                "parent_id".to_string(),
            ],
            "matrix" => vec![
                "requirement_reference_code".to_string(),
                "verification_reference_code".to_string(),
            ],
            _ => vec![],
        }
    }

    pub fn get_available_fields(&self) -> Vec<String> {
        Self::fields_for(&self.import_type)
    }

    pub fn sample_rows(&self, limit: usize) -> Vec<Vec<String>> {
        self.data.iter().take(limit).cloned().collect()
    }

    pub fn unique_values_by_column(&self) -> HashMap<String, Vec<String>> {
        let mut result = HashMap::new();
        for column in &self.columns {
            let mut values = self
                .data
                .iter()
                .filter_map(|row| row.get(column.index))
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            values.sort_by_key(|value| value.to_lowercase());
            values.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
            result.insert(column.name.clone(), values);
        }
        result
    }

    pub fn import_data(
        &self,
        state: &AppState<DieselCachedRepo>,
        actor: &User,
        config: &ImportConfig,
    ) -> Result<ImportResult> {
        if config.import_type != "requirements"
            && config.import_type != "tests"
            && config.import_type != "matrix"
        {
            return Err(anyhow!("Unknown import type: {}", config.import_type));
        }

        if config.import_type == "matrix" {
            return self.import_matrix_links(state, actor, config);
        }

        let defaults = CatalogDefaults::load(state, config.project_id, &config.import_type)?;

        let mut imported_count = 0;
        let mut errors = Vec::new();
        let mut imported_requirement_ids = Vec::new();
        let context = RowImportContext {
            state,
            actor,
            value_mappings: &config.value_mappings,
            project_id: config.project_id,
            defaults: &defaults,
        };

        for (row_index, row_data) in self.data.iter().enumerate() {
            let result = match config.import_type.as_str() {
                "requirements" => {
                    self.import_requirement_row(row_data, &config.column_mappings, &context)
                }
                "tests" => self
                    .import_test_row(row_data, &config.column_mappings, &context)
                    .map(|_| None),
                _ => Err(anyhow!("Unknown import type: {}", config.import_type)),
            };

            match result {
                Ok(opt_id) => {
                    imported_count += 1;
                    if let Some(id) = opt_id {
                        imported_requirement_ids.push(id);
                    }
                }
                Err(e) => {
                    errors.push(format!("Row {}: {}", row_index + 2, e));
                }
            }
        }

        Ok(ImportResult {
            success: errors.is_empty(),
            message: if errors.is_empty() {
                format!("Successfully imported {} records", imported_count)
            } else {
                format!(
                    "Imported {} records with {} errors",
                    imported_count,
                    errors.len()
                )
            },
            imported_count,
            errors,
            imported_requirement_ids,
        })
    }

    fn import_matrix_links(
        &self,
        state: &AppState<DieselCachedRepo>,
        actor: &User,
        config: &ImportConfig,
    ) -> Result<ImportResult> {
        let reqs: HashMap<String, i32> = state
            .repo_read()
            .get_requirements_by_project(config.project_id)
            .map_err(|e| anyhow!("{}", e))?
            .into_iter()
            .filter(|r| r.project_id == config.project_id && !r.reference_code.trim().is_empty())
            .map(|r| (r.reference_code, r.id))
            .collect();
        let vers: HashMap<String, i32> = state
            .repo_read()
            .get_verifications_by_project(config.project_id)
            .map_err(|e| anyhow!("{}", e))?
            .into_iter()
            .filter(|v| v.project_id == config.project_id && !v.reference_code.trim().is_empty())
            .map(|v| (v.reference_code, v.id))
            .collect();
        let existing: HashSet<(i32, i32)> = state
            .repo_read()
            .get_matrix_by_project(config.project_id)
            .map_err(|e| anyhow!("{}", e))?
            .into_iter()
            .map(|m| (m.req_id, m.verification_id))
            .collect();

        let service = MatrixService::new(state);
        let mut imported_count = 0;
        let mut errors = Vec::new();
        let mut seen_this_file: HashSet<(i32, i32)> = HashSet::new();

        for (row_index, row_data) in self.data.iter().enumerate() {
            let values = self.mapped_values(row_data, &config.column_mappings);
            let req_code = values
                .get("requirement_reference_code")
                .map(|s| s.trim())
                .unwrap_or("");
            let ver_code = values
                .get("verification_reference_code")
                .map(|s| s.trim())
                .unwrap_or("");
            if req_code.is_empty() && ver_code.is_empty() {
                continue;
            }
            if req_code.is_empty() || ver_code.is_empty() {
                errors.push(format!(
                    "Row {}: both requirement and verification reference codes are required",
                    row_index + 2
                ));
                continue;
            }
            let Some(req_id) = reqs.get(req_code).copied() else {
                errors.push(format!(
                    "Row {}: requirement '{req_code}' not found",
                    row_index + 2
                ));
                continue;
            };
            let Some(ver_id) = vers.get(ver_code).copied() else {
                errors.push(format!(
                    "Row {}: verification '{ver_code}' not found",
                    row_index + 2
                ));
                continue;
            };
            if existing.contains(&(req_id, ver_id)) || seen_this_file.contains(&(req_id, ver_id)) {
                continue;
            }
            match service.link(actor, req_id, ver_id, config.project_id) {
                Ok(()) => {
                    seen_this_file.insert((req_id, ver_id));
                    imported_count += 1;
                }
                Err(e) => errors.push(format!("Row {}: {e}", row_index + 2)),
            }
        }

        Ok(ImportResult {
            success: errors.is_empty(),
            message: if errors.is_empty() {
                format!("Successfully imported {imported_count} matrix links")
            } else {
                format!(
                    "Imported {imported_count} matrix links with {} errors",
                    errors.len()
                )
            },
            imported_count,
            errors,
            imported_requirement_ids: Vec::new(),
        })
    }

    fn mapped_values(
        &self,
        row_data: &[String],
        mappings: &[ColumnMapping],
    ) -> HashMap<String, String> {
        let mut values = HashMap::new();
        for mapping in mappings {
            if mapping.target_field.trim().is_empty() || mapping.target_field == "skip" {
                continue;
            }
            if let Some(column) = self
                .columns
                .iter()
                .find(|col| col.name == mapping.excel_column)
            {
                if column.index < row_data.len() {
                    values.insert(mapping.target_field.clone(), row_data[column.index].clone());
                }
            }
        }
        values
    }

    fn import_requirement_row(
        &self,
        row_data: &[String],
        mappings: &[ColumnMapping],
        context: &RowImportContext<'_>,
    ) -> Result<Option<i32>> {
        let req_data = self.mapped_values(row_data, mappings);
        let title = req_data
            .get("title")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("title is required"))?;

        let description = req_data
            .get("description")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Imported.".to_string());

        let category_id = self.resolve_optional_named(
            req_data.get("category_id"),
            context.defaults.category_id,
            "category_id",
            context.value_mappings,
            |name| self.resolve_category_id(context.state, name, context.project_id),
        );
        let applicability_id = self.resolve_optional_named(
            req_data.get("applicability_id"),
            context.defaults.applicability_id,
            "applicability_id",
            context.value_mappings,
            |name| self.resolve_applicability_id(context.state, name, context.project_id),
        );
        let status_id = self.resolve_optional_named(
            req_data.get("status_id"),
            context.defaults.req_status_id,
            "status_id",
            context.value_mappings,
            |name| self.resolve_requirement_status_id(context.state, name, context.project_id),
        );
        let verification_method_id = match req_data.get("verification_method_id") {
            Some(name) if !name.trim().is_empty() => {
                let resolved = self.resolve_named(
                    name,
                    context.defaults.verification_method_id.unwrap_or(0),
                    "verification_method_id",
                    context.value_mappings,
                    |value| {
                        self.resolve_verification_method_id(
                            context.state,
                            value,
                            context.project_id,
                        )
                    },
                );
                (resolved > 0).then_some(resolved)
            }
            _ => context.defaults.verification_method_id,
        };
        let author_id = self.resolve_optional_named(
            req_data.get("author_id"),
            context.actor.id,
            "author_id",
            context.value_mappings,
            |name| self.resolve_user_id(context.state, name, context.project_id),
        );
        let reviewer_id = self.resolve_optional_named(
            req_data.get("reviewer_id"),
            context.actor.id,
            "reviewer_id",
            context.value_mappings,
            |name| self.resolve_user_id(context.state, name, context.project_id),
        );

        {
            let repo = context.state.repo_read();
            require_project_reviewer_unless_requirement_create_status_is_draft_like(
                &*repo,
                context.actor,
                context.project_id,
                status_id,
            )
            .map_err(|e| anyhow!("{e}"))?;
        }

        let parent_links = match req_data.get("parent_id") {
            Some(parent) if !parent.trim().is_empty() && parent.trim() != "None" => {
                let parent_req_id = self.resolve_requirement_id_by_title_or_ref(
                    context.state,
                    parent,
                    context.project_id,
                )?;
                let parent_vid = context
                    .state
                    .repo_read()
                    .get_requirement_by_id(parent_req_id)?
                    .current_version_id
                    .ok_or_else(|| anyhow!("parent requirement has no current version"))?;
                Some(vec![(
                    parent_vid,
                    "DERIVES_FROM".to_string(),
                    None::<String>,
                )])
            }
            _ => None,
        };

        let new_req = NewRequirement {
            id: None,
            title,
            description,
            reference_code: req_data
                .get("reference_code")
                .map(|s| s.trim().to_string())
                .unwrap_or_default(),
            category_id,
            applicability_id,
            status_id,
            author_id,
            reviewer_id,
            justification: req_data
                .get("justification")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            project_id: context.project_id,
        };

        let method_ids: Vec<i32> = verification_method_id.into_iter().collect();
        let id = RequirementService::new(context.state)
            .create(context.actor, new_req, &method_ids, None, parent_links)
            .map_err(|e| anyhow!("{}", e))?;
        Ok(Some(id))
    }

    fn import_test_row(
        &self,
        row_data: &[String],
        mappings: &[ColumnMapping],
        context: &RowImportContext<'_>,
    ) -> Result<()> {
        let test_data = self.mapped_values(row_data, mappings);
        let name = test_data
            .get("name")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("name is required"))?;

        let status_id = self.resolve_optional_named(
            test_data.get("status_id"),
            context.defaults.ver_status_id,
            "status_id",
            context.value_mappings,
            |name| self.resolve_verification_status_id(context.state, name, context.project_id),
        );

        {
            let repo = context.state.repo_read();
            require_project_reviewer_unless_verification_create_status_is_initial(
                &*repo,
                context.actor,
                context.project_id,
                status_id,
            )
            .map_err(|e| anyhow!("{e}"))?;
        }

        let parent_id = match test_data.get("parent_id") {
            Some(parent) if !parent.trim().is_empty() && parent.trim() != "None" => {
                Some(self.resolve_test_id_by_name(context.state, parent, context.project_id)?)
            }
            _ => None,
        };

        let reference_code = test_data
            .get("reference_code")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("TEST-{}", chrono::Utc::now().timestamp()));

        let new_verification = NewVerification {
            id: None,
            name,
            description: test_data.get("description").cloned().unwrap_or_default(),
            source: test_data.get("source").cloned().unwrap_or_default(),
            reference_code,
            status_id,
            parent_id,
            project_id: context.project_id,
            verification_method_id: None,
            author_id: context.actor.id,
            reviewer_id: context.actor.id,
        };

        VerificationService::new(context.state)
            .create(context.actor, new_verification)
            .map_err(|e| anyhow!("{}", e))?;
        Ok(())
    }

    fn resolve_optional_named<F>(
        &self,
        raw: Option<&String>,
        default: i32,
        target_field: &str,
        value_mappings: &[ValueMapping],
        resolve: F,
    ) -> i32
    where
        F: FnOnce(&str) -> Result<i32>,
    {
        match raw {
            Some(value) if !value.trim().is_empty() => {
                self.resolve_named(value, default, target_field, value_mappings, resolve)
            }
            _ => default,
        }
    }

    fn resolve_named<F>(
        &self,
        raw: &str,
        default: i32,
        target_field: &str,
        value_mappings: &[ValueMapping],
        resolve: F,
    ) -> i32
    where
        F: FnOnce(&str) -> Result<i32>,
    {
        value_mappings
            .iter()
            .find(|mapping| {
                mapping.target_field == target_field
                    && names_match(&mapping.source_value, raw)
                    && mapping.target_id > 0
            })
            .map(|mapping| mapping.target_id)
            .or_else(|| resolve(raw.trim()).ok())
            .unwrap_or(default)
    }

    fn resolve_category_id(
        &self,
        state: &AppState<DieselCachedRepo>,
        category_name: &str,
        project_id: i32,
    ) -> Result<i32> {
        let categories = state
            .repo_read()
            .get_categories_by_project(project_id)
            .map_err(|e| anyhow!("{}", e))?;
        categories
            .iter()
            .find(|c| names_match(&c.title, category_name) || names_match(&c.tag, category_name))
            .map(|c| c.id)
            .ok_or_else(|| anyhow!("unknown category '{category_name}'"))
    }

    fn resolve_applicability_id(
        &self,
        state: &AppState<DieselCachedRepo>,
        app_name: &str,
        project_id: i32,
    ) -> Result<i32> {
        let items = state
            .repo_read()
            .get_applicability_by_project(project_id)
            .map_err(|e| anyhow!("{}", e))?;
        items
            .iter()
            .find(|a| names_match(&a.title, app_name) || names_match(&a.tag, app_name))
            .map(|a| a.id)
            .ok_or_else(|| anyhow!("unknown applicability '{app_name}'"))
    }

    fn resolve_requirement_status_id(
        &self,
        state: &AppState<DieselCachedRepo>,
        status_name: &str,
        project_id: i32,
    ) -> Result<i32> {
        let statuses = state
            .repo_read()
            .get_requirement_status_by_project(project_id)
            .map_err(|e| anyhow!("{}", e))?;
        statuses
            .iter()
            .find(|s| names_match(&s.title, status_name) || names_match(&s.tag, status_name))
            .map(|s| s.id)
            .ok_or_else(|| anyhow!("unknown requirement status '{status_name}'"))
    }

    fn resolve_verification_status_id(
        &self,
        state: &AppState<DieselCachedRepo>,
        status_name: &str,
        project_id: i32,
    ) -> Result<i32> {
        let statuses = state
            .repo_read()
            .get_verification_status_by_project(project_id)
            .map_err(|e| anyhow!("{}", e))?;
        statuses
            .iter()
            .find(|s| names_match(&s.title, status_name) || names_match(&s.tag, status_name))
            .map(|s| s.id)
            .ok_or_else(|| anyhow!("unknown verification status '{status_name}'"))
    }

    fn resolve_verification_method_id(
        &self,
        state: &AppState<DieselCachedRepo>,
        verification_name: &str,
        project_id: i32,
    ) -> Result<i32> {
        let methods = state
            .repo_read()
            .get_verification_methods_by_project(project_id)
            .map_err(|e| anyhow!("{}", e))?;
        methods
            .iter()
            .find(|m| {
                names_match(&m.title, verification_name) || names_match(&m.tag, verification_name)
            })
            .map(|m| m.id)
            .ok_or_else(|| anyhow!("unknown verification method '{verification_name}'"))
    }

    fn resolve_user_id(
        &self,
        state: &AppState<DieselCachedRepo>,
        name: &str,
        project_id: i32,
    ) -> Result<i32> {
        let repo = state.repo_read();
        let member_ids = repo
            .get_members_by_project(project_id)
            .map_err(|e| anyhow!("{}", e))?
            .into_iter()
            .map(|member| member.user_id)
            .collect::<Vec<_>>();
        let users = repo.get_users_all().map_err(|e| anyhow!("{}", e))?;
        users
            .iter()
            .filter(|user| member_ids.contains(&user.id))
            .find(|u| {
                names_match(&u.name, name)
                    || names_match(&u.username, name)
                    || names_match(&u.email, name)
            })
            .map(|u| u.id)
            .ok_or_else(|| anyhow!("unknown user '{name}'"))
    }

    fn resolve_requirement_id_by_title_or_ref(
        &self,
        state: &AppState<DieselCachedRepo>,
        title: &str,
        project_id: i32,
    ) -> Result<i32> {
        let requirements = state
            .repo_read()
            .get_requirements_by_project(project_id)
            .map_err(|e| anyhow!("{}", e))?;
        requirements
            .iter()
            .find(|r| names_match(&r.title, title) || names_match(&r.reference_code, title))
            .map(|r| r.id)
            .ok_or_else(|| anyhow!("requirement '{title}' not found"))
    }

    fn resolve_test_id_by_name(
        &self,
        state: &AppState<DieselCachedRepo>,
        name: &str,
        project_id: i32,
    ) -> Result<i32> {
        let tests = state
            .repo_read()
            .get_verifications_by_project(project_id)
            .map_err(|e| anyhow!("{}", e))?;
        tests
            .iter()
            .find(|t| names_match(&t.name, name) || names_match(&t.reference_code, name))
            .map(|t| t.id)
            .ok_or_else(|| anyhow!("verification '{name}' not found"))
    }
}

impl CatalogDefaults {
    fn load(
        state: &AppState<DieselCachedRepo>,
        project_id: i32,
        import_type: &str,
    ) -> Result<Self> {
        let repo = state.repo_read();
        if import_type == "requirements" {
            let categories = repo
                .get_categories_by_project(project_id)
                .map_err(|e| anyhow!("{}", e))?;
            let category_id =
                categories.iter().map(|c| c.id).min().ok_or_else(|| {
                    anyhow!("project has no categories; add one before importing")
                })?;
            let applicability = repo
                .get_applicability_by_project(project_id)
                .map_err(|e| anyhow!("{}", e))?;
            let applicability_id = applicability.iter().map(|a| a.id).min().ok_or_else(|| {
                anyhow!("project has no applicability values; add one before importing")
            })?;
            let statuses = repo
                .get_requirement_status_by_project(project_id)
                .map_err(|e| anyhow!("{}", e))?;
            let req_status_id = statuses
                .iter()
                .find(|s| {
                    s.title.eq_ignore_ascii_case("draft")
                        || s.tag.eq_ignore_ascii_case("draft")
                        || s.tag.eq_ignore_ascii_case("drf")
                })
                .map(|s| s.id)
                .or_else(|| statuses.iter().map(|s| s.id).min())
                .ok_or_else(|| anyhow!("project has no requirement statuses"))?;
            let methods = repo
                .get_verification_methods_by_project(project_id)
                .map_err(|e| anyhow!("{}", e))?;
            let verification_method_id = methods.iter().map(|m| m.id).min();
            Ok(Self {
                category_id,
                applicability_id,
                req_status_id,
                ver_status_id: 0,
                verification_method_id,
            })
        } else {
            let statuses = repo
                .get_verification_status_by_project(project_id)
                .map_err(|e| anyhow!("{}", e))?;
            let ver_status_id = statuses
                .iter()
                .find(|s| s.tag.eq_ignore_ascii_case("nr"))
                .map(|s| s.id)
                .or_else(|| statuses.iter().map(|s| s.id).min())
                .ok_or_else(|| anyhow!("project has no verification statuses"))?;
            Ok(Self {
                category_id: 0,
                applicability_id: 0,
                req_status_id: 0,
                ver_status_id,
                verification_method_id: None,
            })
        }
    }
}

fn header_looks_like_requirement_code(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("requirement_reference_code")
        || n.contains("requirement_code")
        || n.contains("requirement code")
        || (n.contains("req") && (n.contains("code") || n.contains("id") || n.contains("ref")))
}

fn header_looks_like_verification_code(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("verification_reference_code")
        || n.contains("verification_code")
        || n.contains("verification code")
        || ((n.contains("test") || n.contains("verif"))
            && (n.contains("code") || n.contains("id") || n.contains("ref")))
}

fn guess_import_type(columns: &[ExcelColumn]) -> String {
    let has_req_code = columns
        .iter()
        .any(|col| header_looks_like_requirement_code(&col.name));
    let has_ver_code = columns
        .iter()
        .any(|col| header_looks_like_verification_code(&col.name));
    if has_req_code && has_ver_code {
        return "matrix".to_string();
    }
    if columns
        .iter()
        .any(|col| col.name.to_lowercase().contains("req"))
    {
        "requirements".to_string()
    } else if columns
        .iter()
        .any(|col| col.name.to_lowercase().contains("test"))
    {
        "tests".to_string()
    } else {
        "requirements".to_string()
    }
}

fn names_match(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

fn looks_like_zip(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0] == 0x50 && bytes[1] == 0x4b
}

fn cell_text(cell: &Data) -> String {
    cell.to_string()
}

use crate::models::{NewApplicability, NewCategory, NewRequirement, NewTest};
use crate::repository::{
    DieselRepo, LookupRepository, RequirementsRepository, TestsRepository, UserRepository,
};
use anyhow::{anyhow, Result};
use calamine::{open_workbook, Reader, Xlsx};
use diesel::{Connection, PgConnection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExcelColumn {
    pub index: usize,
    pub name: String,
    pub sample_value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ColumnMapping {
    pub excel_column: String,
    pub target_field: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportConfig {
    pub import_type: String, // "requirements" or "tests"
    pub column_mappings: Vec<ColumnMapping>,
    pub project_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub success: bool,
    pub message: String,
    pub imported_count: usize,
    pub errors: Vec<String>,
}

pub struct ExcelImporter {
    pub columns: Vec<ExcelColumn>,
    pub data: Vec<Vec<String>>,
    pub import_type: String,
}

impl ExcelImporter {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut workbook: Xlsx<_> = open_workbook(path)?;

        // Get the first sheet
        let sheet_name = workbook.sheet_names()[0].clone();
        let range = workbook
            .worksheet_range(&sheet_name)
            .ok_or_else(|| anyhow!("Sheet not found: {}", sheet_name))?
            .map_err(|e| anyhow!("Failed to read sheet: {}", e))?;

        let mut columns = Vec::new();
        let mut data = Vec::new();
        let mut is_first_row = true;

        for row in range.rows() {
            if is_first_row {
                // Parse headers
                for (i, cell) in row.iter().enumerate() {
                    columns.push(ExcelColumn {
                        index: i,
                        name: cell.to_string(),
                        sample_value: String::new(),
                    });
                }
                is_first_row = false;
                continue;
            }

            if row.iter().all(|cell| cell.is_empty()) {
                continue; // Skip empty rows
            }

            // Store sample values from first few rows
            if data.len() < 3 {
                for (i, cell) in row.iter().enumerate() {
                    if i < columns.len() && data.len() == 0 {
                        columns[i].sample_value = cell.to_string();
                    }
                }
            }

            // Store row data
            let row_data: Vec<String> = row.iter().map(|cell| cell.to_string()).collect();
            data.push(row_data);
        }

        // Determine import type based on column names
        let import_type = if columns
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
            "requirements".to_string() // Default
        };

        Ok(ExcelImporter {
            columns,
            data,
            import_type,
        })
    }

    pub fn get_available_fields(&self) -> Vec<String> {
        match self.import_type.as_str() {
            "requirements" => vec![
                "req_title".to_string(),
                "req_description".to_string(),
                "req_reference".to_string(),
                "req_category".to_string(),
                "req_applicability".to_string(),
                "req_current_status".to_string(),
                "req_verification".to_string(),
                "req_author".to_string(),
                "req_reviewer".to_string(),
                "req_parent".to_string(),
                "req_link".to_string(),
                "req_justification".to_string(),
            ],
            "tests" => vec![
                "test_name".to_string(),
                "test_description".to_string(),
                "test_status".to_string(),
                "test_source".to_string(),
                "test_parent".to_string(),
            ],
            _ => vec![],
        }
    }

    pub fn import_data(
        &self,
        config: &ImportConfig,
        conn: &mut PgConnection,
    ) -> Result<ImportResult> {
        let mut imported_count = 0;
        let mut errors = Vec::new();

        // Start transaction
        conn.transaction(|conn| {
            for (row_index, row_data) in self.data.iter().enumerate() {
                let result = match config.import_type.as_str() {
                    "requirements" => self.import_requirement_row(
                        row_data,
                        &config.column_mappings,
                        config.project_id,
                        conn,
                    ),
                    "tests" => self.import_test_row(
                        row_data,
                        &config.column_mappings,
                        config.project_id,
                        conn,
                    ),
                    _ => Err(anyhow!("Unknown import type: {}", config.import_type)),
                };

                match result {
                    Ok(_) => imported_count += 1,
                    Err(e) => {
                        errors.push(format!("Row {}: {}", row_index + 2, e));
                        // Continue processing other rows
                    }
                }
            }

            Ok::<(), anyhow::Error>(())
        })?;

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
        })
    }

    fn import_requirement_row(
        &self,
        row_data: &[String],
        mappings: &[ColumnMapping],
        project_id: i32,
        conn: &mut PgConnection,
    ) -> Result<()> {
        let mut req_data = HashMap::new();

        // Map Excel columns to requirement fields
        for mapping in mappings {
            if let Some(column) = self
                .columns
                .iter()
                .find(|col| col.name == mapping.excel_column)
            {
                if column.index < row_data.len() {
                    req_data.insert(mapping.target_field.clone(), row_data[column.index].clone());
                }
            }
        }

        // Resolve foreign key references
        let category_id = if let Some(category_name) = req_data.get("req_category") {
            self.resolve_category_id(category_name, project_id)?
        } else {
            1 // Default category
        };

        let applicability_id = if let Some(app_name) = req_data.get("req_applicability") {
            self.resolve_applicability_id(app_name, project_id)?
        } else {
            1 // Default applicability
        };

        let status_id = if let Some(status_name) = req_data.get("req_current_status") {
            self.resolve_requirement_status_id(status_name, conn)?
        } else {
            1 // Default status
        };

        let author_id = if let Some(author_name) = req_data.get("req_author") {
            self.resolve_user_id(author_name, project_id, conn)?
        } else {
            1 // Default user
        };

        let reviewer_id = if let Some(reviewer_name) = req_data.get("req_reviewer") {
            self.resolve_user_id(reviewer_name, project_id, conn)?
        } else {
            1 // Default user
        };

        let parent_id = if let Some(parent_title) = req_data.get("req_parent") {
            if !parent_title.is_empty() && parent_title != "None" {
                self.resolve_requirement_id_by_title(parent_title, project_id, conn)
                    .ok()
            } else {
                None
            }
        } else {
            None
        };

        // Create new requirement
        let new_req = NewRequirement {
            req_id: None,
            req_title: req_data
                .get("req_title")
                .unwrap_or(&"Imported Requirement".to_string())
                .clone(),
            req_description: req_data
                .get("req_description")
                .unwrap_or(&"".to_string())
                .clone(),
            req_reference: req_data
                .get("req_reference")
                .unwrap_or(&"".to_string())
                .clone(),
            req_category: category_id,
            req_applicability: applicability_id,
            req_current_status: status_id,
            req_verification: 1, // Default verification
            req_author: author_id,
            req_reviewer: reviewer_id,
            req_parent: parent_id.unwrap_or(0),
            req_link: req_data.get("req_link").unwrap_or(&"".to_string()).clone(),
            req_justification: req_data.get("req_justification").cloned(),
            project_id,
        };

        DieselRepo::new()
            .insert_new_requirement(&new_req)
            .map_err(|e| anyhow!("{}", e))?;
        Ok(())
    }

    fn import_test_row(
        &self,
        row_data: &[String],
        mappings: &[ColumnMapping],
        project_id: i32,
        conn: &mut PgConnection,
    ) -> Result<()> {
        let mut test_data = HashMap::new();

        // Map Excel columns to test fields
        for mapping in mappings {
            if let Some(column) = self
                .columns
                .iter()
                .find(|col| col.name == mapping.excel_column)
            {
                if column.index < row_data.len() {
                    test_data.insert(mapping.target_field.clone(), row_data[column.index].clone());
                }
            }
        }

        // Resolve foreign key references
        let status_id = if let Some(status_name) = test_data.get("test_status") {
            self.resolve_test_status_id(status_name, conn)?
        } else {
            1 // Default status
        };

        let parent_id = if let Some(parent_name) = test_data.get("test_parent") {
            if !parent_name.is_empty() && parent_name != "None" {
                self.resolve_test_id_by_name(parent_name, project_id, conn)
                    .ok()
            } else {
                None
            }
        } else {
            None
        };

        // Create new test
        let new_test = NewTest {
            test_id: None,
            test_name: test_data
                .get("test_name")
                .unwrap_or(&"Imported Test".to_string())
                .clone(),
            test_description: test_data
                .get("test_description")
                .unwrap_or(&"".to_string())
                .clone(),
            test_source: test_data
                .get("test_source")
                .unwrap_or(&"".to_string())
                .clone(),
            test_reference: test_data
                .get("test_reference")
                .unwrap_or(&format!("TEST-{}", chrono::Utc::now().timestamp()))
                .clone(),
            test_status: status_id,
            test_parent: parent_id.unwrap_or(0),
            project_id,
        };

        DieselRepo::new()
            .insert_test(&new_test)
            .map_err(|e| anyhow!("{}", e))?;
        Ok(())
    }

    fn resolve_category_id(&self, category_name: &str, project_id: i32) -> Result<i32> {
        let repo = DieselRepo::new();
        let categories = repo
            .get_categories_by_project(project_id)
            .map_err(|e| anyhow!("{}", e))?;
        for category in categories {
            if category.cat_title == category_name {
                return Ok(category.cat_id);
            }
        }

        // Create new category if not found
        let new_category = NewCategory {
            cat_id: None,
            cat_title: category_name.to_string(),
            cat_description: format!("Imported category: {}", category_name),
            cat_tag: category_name.to_lowercase().replace(" ", "_"),
            project_id,
        };

        DieselRepo::new()
            .insert_new_category(&new_category)
            .map_err(|e| anyhow!("{}", e))
    }

    fn resolve_applicability_id(&self, app_name: &str, project_id: i32) -> Result<i32> {
        let applicability_list = DieselRepo::new()
            .get_applicability_by_project(project_id)
            .map_err(|e| anyhow!("{}", e))?;
        for app in applicability_list {
            if app.app_title == app_name {
                return Ok(app.app_id);
            }
        }

        // Create new applicability if not found
        let new_app = NewApplicability {
            app_id: None,
            app_title: app_name.to_string(),
            app_description: format!("Imported applicability: {}", app_name),
            app_tag: app_name.to_lowercase().replace(" ", "_"),
            project_id,
        };

        DieselRepo::new()
            .insert_new_applicability(&new_app)
            .map_err(|e| anyhow!("{}", e))
    }

    fn resolve_requirement_status_id(
        &self,
        status_name: &str,
        _conn: &mut PgConnection,
    ) -> Result<i32> {
        let repo = DieselRepo::new();
        let statuses = repo
            .get_requirement_status_all()
            .map_err(|e| anyhow!("{}", e))?;
        for status in statuses {
            if status.req_st_title == status_name {
                return Ok(status.req_st_id);
            }
        }

        // Return default status ID if not found
        Ok(1)
    }

    fn resolve_test_status_id(&self, status_name: &str, _conn: &mut PgConnection) -> Result<i32> {
        let repo = DieselRepo::new();
        let statuses = repo.get_test_status_all().map_err(|e| anyhow!("{}", e))?;
        for status in statuses {
            if status.test_st_title == status_name {
                return Ok(status.test_st_id);
            }
        }

        // Return default status ID if not found
        Ok(1)
    }

    fn resolve_user_id(
        &self,
        user_name: &str,
        _project_id: i32,
        _conn: &mut PgConnection,
    ) -> Result<i32> {
        let repo = DieselRepo::new();
        let users = repo.get_users_all().map_err(|e| anyhow!("{}", e))?;
        for user in users {
            if user.user_name == user_name {
                return Ok(user.user_id);
            }
        }

        // Return default user ID if not found
        Ok(1)
    }

    fn resolve_requirement_id_by_title(
        &self,
        title: &str,
        project_id: i32,
        _conn: &mut PgConnection,
    ) -> Result<i32> {
        let repo = DieselRepo::new();
        let requirements = repo
            .get_requirements_by_project(project_id)
            .map_err(|e| anyhow!("{}", e))?;
        for req in requirements {
            if req.req_title == title {
                return Ok(req.req_id);
            }
        }

        Err(anyhow!("Requirement with title '{}' not found", title))
    }

    fn resolve_test_id_by_name(
        &self,
        name: &str,
        project_id: i32,
        _conn: &mut PgConnection,
    ) -> Result<i32> {
        let repo = DieselRepo::new();
        let tests = repo
            .get_tests_by_project(project_id)
            .map_err(|e| anyhow!("{}", e))?;
        for test in tests {
            if test.test_name == name {
                return Ok(test.test_id);
            }
        }

        Err(anyhow!("Test with name '{}' not found", name))
    }
}

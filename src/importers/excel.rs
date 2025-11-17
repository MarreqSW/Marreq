use crate::models::{NewApplicability, NewCategory, NewRequirement, NewTestCase};
use crate::repository::{
    DieselRepo, LookupRepository, RequirementsRepository, TestsRepository, UserRepository,
};
use anyhow::{anyhow, Result};
use calamine::{open_workbook, DataType, Reader, Xlsx};
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
            .map_err(|e| anyhow!("Failed to read sheet `{}`: {}", sheet_name, e))?;

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
                "title".to_string(),
                "description".to_string(),
                "reference_code".to_string(),
                "category_id".to_string(),
                "applicability_id".to_string(),
                "current_status_id".to_string(),
                "verification_method_id".to_string(),
                "author_id".to_string(),
                "reviewer_id".to_string(),
                "parent_id".to_string(),
                "justification".to_string(),
            ],
            "tests" => vec![
                "name".to_string(),
                "description".to_string(),
                "status_id".to_string(),
                "source".to_string(),
                "parent_id".to_string(),
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
        let category_id = if let Some(category_name) = req_data.get("category_id") {
            self.resolve_category_id(category_name, project_id)?
        } else {
            1 // Default category
        };

        let applicability_id = if let Some(app_name) = req_data.get("applicability_id") {
            self.resolve_applicability_id(app_name, project_id)?
        } else {
            1 // Default applicability
        };

        let status_id = if let Some(status_name) = req_data.get("current_status_id") {
            self.resolve_requirement_status_id(status_name, conn)?
        } else {
            1 // Default status
        };

        let author_id = if let Some(author_name) = req_data.get("author_id") {
            self.resolve_user_id(author_name, project_id, conn)?
        } else {
            1 // Default user
        };

        let reviewer_id = if let Some(reviewer_name) = req_data.get("reviewer_id") {
            self.resolve_user_id(reviewer_name, project_id, conn)?
        } else {
            1 // Default user
        };

        let parent_id = if let Some(parent_title) = req_data.get("parent_id") {
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
            id: None,
            title: req_data
                .get("title")
                .unwrap_or(&"Imported Requirement".to_string())
                .clone(),
            description: req_data
                .get("description")
                .unwrap_or(&"".to_string())
                .clone(),
            reference_code: req_data
                .get("reference_code")
                .unwrap_or(&"".to_string())
                .clone(),
            category_id: category_id,
            applicability_id: applicability_id,
            current_status_id: status_id,
            verification_method_id: 1, // Default verification
            author_id: author_id,
            reviewer_id: reviewer_id,
            parent_id: parent_id.unwrap_or(0),
            justification: req_data.get("justification").cloned(),
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
        let status_id = if let Some(status_name) = test_data.get("status_id") {
            self.resolve_test_status_id(status_name, conn)?
        } else {
            1 // Default status
        };

        let parent_id = if let Some(parent_name) = test_data.get("parent_id") {
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
        let new_test = NewTestCase {
            id: None,
            name: test_data
                .get("name")
                .unwrap_or(&"Imported Test".to_string())
                .clone(),
            description: test_data
                .get("description")
                .unwrap_or(&"".to_string())
                .clone(),
            source: test_data
                .get("source")
                .unwrap_or(&"".to_string())
                .clone(),
            reference_code: test_data
                .get("reference_code")
                .unwrap_or(&format!("TEST-{}", chrono::Utc::now().timestamp()))
                .clone(),
            status_id: status_id,
            parent_id: parent_id.unwrap_or(0),
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
            if category.title == category_name {
                return Ok(category.id);
            }
        }

        // Create new category if not found
        let new_category = NewCategory {
            id: None,
            title: category_name.to_string(),
            description: format!("Imported category: {}", category_name),
            tag: category_name.to_lowercase().replace(" ", "_"),
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
            if app.title == app_name {
                return Ok(app.id);
            }
        }

        // Create new applicability if not found
        let new_app = NewApplicability {
            id: None,
            title: app_name.to_string(),
            description: format!("Imported applicability: {}", app_name),
            tag: app_name.to_lowercase().replace(" ", "_"),
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
            if status.title == status_name {
                return Ok(status.id);
            }
        }

        // Return default status ID if not found
        Ok(1)
    }

    fn resolve_test_status_id(&self, status_name: &str, _conn: &mut PgConnection) -> Result<i32> {
        let repo = DieselRepo::new();
        let statuses = repo.get_test_status_all().map_err(|e| anyhow!("{}", e))?;
        for status in statuses {
            if status.title == status_name {
                return Ok(status.id);
            }
        }

        // Return default status ID if not found
        Ok(1)
    }

    fn resolve_user_id(
        &self,
        name: &str,
        _project_id: i32,
        _conn: &mut PgConnection,
    ) -> Result<i32> {
        let repo = DieselRepo::new();
        let users = repo.get_users_all().map_err(|e| anyhow!("{}", e))?;
        for user in users {
            if user.name == name {
                return Ok(user.id);
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
            if req.title == title {
                return Ok(req.id);
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
            if test.name == name {
                return Ok(test.id);
            }
        }

        Err(anyhow!("Test with name '{}' not found", name))
    }
}

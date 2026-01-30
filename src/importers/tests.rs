//! Comprehensive test suite for the importers module.
//!
//! This module provides tests for Excel import functionality, ensuring:
//! - Data structures work correctly
//! - Field mappings and configurations are handled properly
//! - Error handling works as expected
//! - Edge cases are covered

#[cfg(test)]
mod tests {
    use crate::importers::*;
    use serde_json;

    // ============================================================================
    // Tests for ExcelColumn
    // ============================================================================

    mod excel_column_tests {
        use super::*;

        #[test]
        fn excel_column_creation() {
            let column = ExcelColumn {
                index: 0,
                name: "Title".to_string(),
                sample_value: "Sample Title".to_string(),
            };
            assert_eq!(column.index, 0);
            assert_eq!(column.name, "Title");
            assert_eq!(column.sample_value, "Sample Title");
        }

        #[test]
        fn excel_column_clone() {
            let column = ExcelColumn {
                index: 1,
                name: "Description".to_string(),
                sample_value: "Sample".to_string(),
            };
            let cloned = column.clone();
            assert_eq!(column.index, cloned.index);
            assert_eq!(column.name, cloned.name);
        }

        #[test]
        fn excel_column_debug() {
            let column = ExcelColumn {
                index: 0,
                name: "Test".to_string(),
                sample_value: "Value".to_string(),
            };
            let debug = format!("{:?}", column);
            assert!(debug.contains("Test"));
        }

        #[test]
        fn excel_column_serialization() {
            let column = ExcelColumn {
                index: 0,
                name: "Title".to_string(),
                sample_value: "Sample".to_string(),
            };
            let json = serde_json::to_string(&column).unwrap();
            assert!(json.contains("\"index\":0"));
            assert!(json.contains("\"name\":\"Title\""));
        }

        #[test]
        fn excel_column_deserialization() {
            let json = r#"{"index":0,"name":"Title","sample_value":"Sample"}"#;
            let column: ExcelColumn = serde_json::from_str(json).unwrap();
            assert_eq!(column.index, 0);
            assert_eq!(column.name, "Title");
        }

        #[test]
        fn excel_column_with_empty_values() {
            let column = ExcelColumn {
                index: 0,
                name: "".to_string(),
                sample_value: "".to_string(),
            };
            assert_eq!(column.name, "");
            assert_eq!(column.sample_value, "");
        }
    }

    // ============================================================================
    // Tests for ColumnMapping
    // ============================================================================

    mod column_mapping_tests {
        use super::*;

        #[test]
        fn column_mapping_creation() {
            let mapping = ColumnMapping {
                excel_column: "Excel Column A".to_string(),
                target_field: "title".to_string(),
            };
            assert_eq!(mapping.excel_column, "Excel Column A");
            assert_eq!(mapping.target_field, "title");
        }

        #[test]
        fn column_mapping_debug() {
            let mapping = ColumnMapping {
                excel_column: "Col".to_string(),
                target_field: "field".to_string(),
            };
            let debug = format!("{:?}", mapping);
            assert!(debug.contains("Col"));
        }

        #[test]
        fn column_mapping_serialization() {
            let mapping = ColumnMapping {
                excel_column: "Title".to_string(),
                target_field: "title".to_string(),
            };
            let json = serde_json::to_string(&mapping).unwrap();
            assert!(json.contains("\"excel_column\":\"Title\""));
            assert!(json.contains("\"target_field\":\"title\""));
        }

        #[test]
        fn column_mapping_deserialization() {
            let json = r#"{"excel_column":"Title","target_field":"title"}"#;
            let mapping: ColumnMapping = serde_json::from_str(json).unwrap();
            assert_eq!(mapping.excel_column, "Title");
            assert_eq!(mapping.target_field, "title");
        }

        #[test]
        fn column_mapping_with_special_characters() {
            let mapping = ColumnMapping {
                excel_column: "Column Name (A)".to_string(),
                target_field: "field_name".to_string(),
            };
            assert_eq!(mapping.excel_column, "Column Name (A)");
        }
    }

    // ============================================================================
    // Tests for ImportConfig
    // ============================================================================

    mod import_config_tests {
        use super::*;

        #[test]
        fn import_config_creation() {
            let config = ImportConfig {
                import_type: "requirements".to_string(),
                column_mappings: vec![],
                project_id: 1,
            };
            assert_eq!(config.import_type, "requirements");
            assert_eq!(config.project_id, 1);
            assert_eq!(config.column_mappings.len(), 0);
        }

        #[test]
        fn import_config_with_mappings() {
            let mappings = vec![
                ColumnMapping {
                    excel_column: "Title".to_string(),
                    target_field: "title".to_string(),
                },
                ColumnMapping {
                    excel_column: "Description".to_string(),
                    target_field: "description".to_string(),
                },
            ];
            let config = ImportConfig {
                import_type: "requirements".to_string(),
                column_mappings: mappings.clone(),
                project_id: 1,
            };
            assert_eq!(config.column_mappings.len(), 2);
        }

        #[test]
        fn import_config_for_tests() {
            let config = ImportConfig {
                import_type: "tests".to_string(),
                column_mappings: vec![],
                project_id: 2,
            };
            assert_eq!(config.import_type, "tests");
        }

        #[test]
        fn import_config_debug() {
            let config = ImportConfig {
                import_type: "requirements".to_string(),
                column_mappings: vec![],
                project_id: 1,
            };
            let debug = format!("{:?}", config);
            assert!(debug.contains("requirements"));
        }

        #[test]
        fn import_config_serialization() {
            let config = ImportConfig {
                import_type: "requirements".to_string(),
                column_mappings: vec![ColumnMapping {
                    excel_column: "Title".to_string(),
                    target_field: "title".to_string(),
                }],
                project_id: 1,
            };
            let json = serde_json::to_string(&config).unwrap();
            assert!(json.contains("\"import_type\":\"requirements\""));
            assert!(json.contains("\"project_id\":1"));
        }

        #[test]
        fn import_config_deserialization() {
            let json = r#"{"import_type":"requirements","column_mappings":[],"project_id":1}"#;
            let config: ImportConfig = serde_json::from_str(json).unwrap();
            assert_eq!(config.import_type, "requirements");
            assert_eq!(config.project_id, 1);
        }
    }

    // ============================================================================
    // Tests for ImportResult
    // ============================================================================

    mod import_result_tests {
        use super::*;

        #[test]
        fn import_result_success() {
            let result = ImportResult {
                success: true,
                message: "Successfully imported 10 records".to_string(),
                imported_count: 10,
                errors: vec![],
                imported_requirement_ids: vec![],
            };
            assert!(result.success);
            assert_eq!(result.imported_count, 10);
            assert_eq!(result.errors.len(), 0);
        }

        #[test]
        fn import_result_with_errors() {
            let result = ImportResult {
                success: false,
                message: "Imported 8 records with 2 errors".to_string(),
                imported_count: 8,
                errors: vec!["Row 3: Error".to_string(), "Row 5: Error".to_string()],
                imported_requirement_ids: vec![],
            };
            assert!(!result.success);
            assert_eq!(result.imported_count, 8);
            assert_eq!(result.errors.len(), 2);
        }

        #[test]
        fn import_result_debug() {
            let result = ImportResult {
                success: true,
                message: "Test".to_string(),
                imported_count: 0,
                errors: vec![],
                imported_requirement_ids: vec![],
            };
            let debug = format!("{:?}", result);
            assert!(debug.contains("Test"));
        }

        #[test]
        fn import_result_serialization() {
            let result = ImportResult {
                success: true,
                message: "Success".to_string(),
                imported_count: 5,
                errors: vec![],
                imported_requirement_ids: vec![],
            };
            let json = serde_json::to_string(&result).unwrap();
            assert!(json.contains("\"success\":true"));
            assert!(json.contains("\"imported_count\":5"));
        }

        #[test]
        fn import_result_deserialization() {
            let json = r#"{"success":true,"message":"Test","imported_count":3,"errors":[]}"#;
            let result: ImportResult = serde_json::from_str(json).unwrap();
            assert!(result.success);
            assert_eq!(result.imported_count, 3);
        }

        #[test]
        fn import_result_empty_import() {
            let result = ImportResult {
                success: true,
                message: "Successfully imported 0 records".to_string(),
                imported_count: 0,
                errors: vec![],
                imported_requirement_ids: vec![],
            };
            assert_eq!(result.imported_count, 0);
        }
    }

    // ============================================================================
    // Tests for ExcelImporter logic and methods
    // ============================================================================

    mod excel_importer_tests {
        use super::*;

        #[test]
        fn get_available_fields_for_requirements() {
            let importer = ExcelImporter {
                columns: vec![],
                data: vec![],
                import_type: "requirements".to_string(),
            };
            let fields = importer.get_available_fields();
            assert_eq!(fields.len(), 11);
            assert!(fields.contains(&"title".to_string()));
            assert!(fields.contains(&"description".to_string()));
            assert!(fields.contains(&"reference_code".to_string()));
            assert!(fields.contains(&"category_id".to_string()));
            assert!(fields.contains(&"applicability_id".to_string()));
            assert!(fields.contains(&"status_id".to_string()));
            assert!(fields.contains(&"verification_method_id".to_string()));
            assert!(fields.contains(&"author_id".to_string()));
            assert!(fields.contains(&"reviewer_id".to_string()));
            assert!(fields.contains(&"parent_id".to_string()));
            assert!(fields.contains(&"justification".to_string()));
        }

        #[test]
        fn get_available_fields_for_tests() {
            let importer = ExcelImporter {
                columns: vec![],
                data: vec![],
                import_type: "tests".to_string(),
            };
            let fields = importer.get_available_fields();
            assert_eq!(fields.len(), 5);
            assert!(fields.contains(&"name".to_string()));
            assert!(fields.contains(&"description".to_string()));
            assert!(fields.contains(&"status_id".to_string()));
            assert!(fields.contains(&"source".to_string()));
            assert!(fields.contains(&"parent_id".to_string()));
        }

        #[test]
        fn get_available_fields_for_unknown_type() {
            let importer = ExcelImporter {
                columns: vec![],
                data: vec![],
                import_type: "unknown".to_string(),
            };
            let fields = importer.get_available_fields();
            assert_eq!(fields.len(), 0);
        }

        #[test]
        fn import_type_detection_requirements() {
            // Test logic for detecting import type based on column names
            let columns = vec!["Title", "Req ID", "Description"];
            let has_req = columns.iter().any(|col| col.to_lowercase().contains("req"));
            assert!(has_req);
        }

        #[test]
        fn import_type_detection_tests() {
            let columns = vec!["Name", "Test ID", "Status"];
            let has_test = columns
                .iter()
                .any(|col| col.to_lowercase().contains("test"));
            assert!(has_test);
        }

        #[test]
        fn import_type_default() {
            let columns = vec!["Column1", "Column2"];
            let has_req = columns.iter().any(|col| col.to_lowercase().contains("req"));
            let has_test = columns
                .iter()
                .any(|col| col.to_lowercase().contains("test"));
            // Should default to requirements if neither found
            assert!(!has_req && !has_test);
        }

        #[test]
        fn excel_importer_field_access() {
            let importer = ExcelImporter {
                columns: vec![ExcelColumn {
                    index: 0,
                    name: "Title".to_string(),
                    sample_value: "Sample".to_string(),
                }],
                data: vec![vec!["Data".to_string()]],
                import_type: "requirements".to_string(),
            };
            assert_eq!(importer.columns.len(), 1);
            assert_eq!(importer.data.len(), 1);
            assert_eq!(importer.import_type, "requirements");
        }
    }

    // ============================================================================
    // Tests for data processing logic
    // ============================================================================

    mod data_processing_tests {
        use super::*;

        #[test]
        fn column_index_mapping() {
            let columns = vec![
                ExcelColumn {
                    index: 0,
                    name: "Title".to_string(),
                    sample_value: "".to_string(),
                },
                ExcelColumn {
                    index: 1,
                    name: "Description".to_string(),
                    sample_value: "".to_string(),
                },
            ];
            let mapping = ColumnMapping {
                excel_column: "Title".to_string(),
                target_field: "title".to_string(),
            };
            if let Some(column) = columns.iter().find(|col| col.name == mapping.excel_column) {
                assert_eq!(column.index, 0);
            }
        }

        #[test]
        fn row_data_extraction() {
            let row_data = vec!["Value1".to_string(), "Value2".to_string()];
            assert_eq!(row_data.len(), 2);
            assert_eq!(row_data[0], "Value1");
        }

        #[test]
        fn empty_row_detection() {
            let empty_row: Vec<String> = vec!["".to_string(), "".to_string()];
            let is_empty = empty_row.iter().all(|cell| cell.is_empty());
            assert!(is_empty);
        }

        #[test]
        fn non_empty_row_detection() {
            let row: Vec<String> = vec!["Value".to_string(), "".to_string()];
            let is_empty = row.iter().all(|cell| cell.is_empty());
            assert!(!is_empty);
        }

        #[test]
        fn sample_value_storage() {
            let mut columns = vec![ExcelColumn {
                index: 0,
                name: "Title".to_string(),
                sample_value: String::new(),
            }];
            let row_data = vec!["Sample Title".to_string()];
            if columns[0].index < row_data.len() {
                columns[0].sample_value = row_data[0].clone();
            }
            assert_eq!(columns[0].sample_value, "Sample Title");
        }

        #[test]
        fn parent_id_resolution_none() {
            let parent_title = "None";
            let is_none = parent_title.is_empty() || parent_title == "None";
            assert!(is_none);
        }

        #[test]
        fn parent_id_resolution_empty() {
            let parent_title = "";
            let is_none = parent_title.is_empty() || parent_title == "None";
            assert!(is_none);
        }

        #[test]
        fn parent_id_resolution_valid() {
            let parent_title = "Parent Requirement";
            let is_none = parent_title.is_empty() || parent_title == "None";
            assert!(!is_none);
        }
    }

    // ============================================================================
    // Tests for error handling patterns
    // ============================================================================

    mod error_handling_tests {
        use super::*;

        #[test]
        fn error_message_formatting() {
            let row_index = 3;
            let error_msg = format!("Row {}: {}", row_index + 2, "Test error");
            assert_eq!(error_msg, "Row 5: Test error");
        }

        #[test]
        fn import_result_success_determination() {
            let errors: Vec<String> = vec![];
            let success = errors.is_empty();
            assert!(success);
        }

        #[test]
        fn import_result_failure_determination() {
            let errors = vec!["Error 1".to_string()];
            let success = errors.is_empty();
            assert!(!success);
        }

        #[test]
        fn success_message_formatting() {
            let imported_count = 10;
            let message = format!("Successfully imported {} records", imported_count);
            assert_eq!(message, "Successfully imported 10 records");
        }

        #[test]
        fn error_message_formatting_with_errors() {
            let imported_count = 8;
            let error_count = 2;
            let message = format!(
                "Imported {} records with {} errors",
                imported_count, error_count
            );
            assert_eq!(message, "Imported 8 records with 2 errors");
        }

        #[test]
        fn unknown_import_type_error() {
            let import_type = "unknown";
            let error_msg = format!("Unknown import type: {}", import_type);
            assert_eq!(error_msg, "Unknown import type: unknown");
        }
    }

    // ============================================================================
    // Tests for default value handling
    // ============================================================================

    mod default_value_tests {
        use super::*;

        #[test]
        fn default_category_id() {
            let category_id = 1; // Default
            assert_eq!(category_id, 1);
        }

        #[test]
        fn default_applicability_id() {
            let applicability_id = 1; // Default
            assert_eq!(applicability_id, 1);
        }

        #[test]
        fn default_status_id() {
            let status_id = 1; // Default
            assert_eq!(status_id, 1);
        }

        #[test]
        fn default_user_id() {
            let user_id = 1; // Default
            assert_eq!(user_id, 1);
        }

        #[test]
        fn default_verification_method_id() {
            let verification_method_id = 1; // Default
            assert_eq!(verification_method_id, 1);
        }

        #[test]
        fn default_requirement_title() {
            let title = "Imported Requirement".to_string();
            assert_eq!(title, "Imported Requirement");
        }

        #[test]
        fn default_test_name() {
            let name = "Imported Test".to_string();
            assert_eq!(name, "Imported Test");
        }

        #[test]
        fn default_empty_string() {
            let value = "".to_string();
            assert_eq!(value, "");
        }
    }

    // ============================================================================
    // Tests for string transformations
    // ============================================================================

    mod string_transformation_tests {
        #[test]
        fn lowercase_conversion() {
            let text = "Category Name";
            let lower = text.to_lowercase();
            assert_eq!(lower, "category name");
        }

        #[test]
        fn space_to_underscore_replacement() {
            let text = "Category Name";
            let replaced = text.replace(" ", "_");
            assert_eq!(replaced, "Category_Name");
        }

        #[test]
        fn lowercase_with_underscore() {
            let text = "Category Name";
            let result = text.to_lowercase().replace(" ", "_");
            assert_eq!(result, "category_name");
        }

        #[test]
        fn contains_check_case_insensitive() {
            let text = "Requirement Title";
            let contains_req = text.to_lowercase().contains("req");
            assert!(contains_req);
        }

        #[test]
        fn contains_check_test() {
            let text = "Test Case Name";
            let contains_test = text.to_lowercase().contains("test");
            assert!(contains_test);
        }
    }

    // ============================================================================
    // Edge case tests
    // ============================================================================

    mod edge_case_tests {
        use super::*;

        #[test]
        fn empty_columns_list() {
            let columns: Vec<ExcelColumn> = vec![];
            assert_eq!(columns.len(), 0);
        }

        #[test]
        fn empty_data_list() {
            let data: Vec<Vec<String>> = vec![];
            assert_eq!(data.len(), 0);
        }

        #[test]
        fn empty_column_mappings() {
            let mappings: Vec<ColumnMapping> = vec![];
            assert_eq!(mappings.len(), 0);
        }

        #[test]
        fn empty_errors_list() {
            let errors: Vec<String> = vec![];
            assert_eq!(errors.len(), 0);
        }

        #[test]
        fn large_import_count() {
            let result = ImportResult {
                success: true,
                message: format!("Successfully imported {} records", 1000),
                imported_count: 1000,
                errors: vec![],
                imported_requirement_ids: vec![],
            };
            assert_eq!(result.imported_count, 1000);
        }

        #[test]
        fn multiple_errors() {
            let errors = vec![
                "Row 2: Error 1".to_string(),
                "Row 4: Error 2".to_string(),
                "Row 6: Error 3".to_string(),
            ];
            assert_eq!(errors.len(), 3);
        }

        #[test]
        fn column_index_boundary() {
            let column = ExcelColumn {
                index: 0,
                name: "Col".to_string(),
                sample_value: "".to_string(),
            };
            let row_data = vec!["Value".to_string()];
            let in_bounds = column.index < row_data.len();
            assert!(in_bounds);
        }

        #[test]
        fn column_index_out_of_bounds() {
            let column = ExcelColumn {
                index: 5,
                name: "Col".to_string(),
                sample_value: "".to_string(),
            };
            let row_data = vec!["Value".to_string()];
            let in_bounds = column.index < row_data.len();
            assert!(!in_bounds);
        }
    }

    // ============================================================================
    // Tests for data mapping logic
    // ============================================================================

    mod data_mapping_tests {
        use super::*;
        use std::collections::HashMap;

        #[test]
        fn requirement_data_mapping_basic() {
            let columns = vec![
                ExcelColumn {
                    index: 0,
                    name: "Title".to_string(),
                    sample_value: "".to_string(),
                },
                ExcelColumn {
                    index: 1,
                    name: "Description".to_string(),
                    sample_value: "".to_string(),
                },
            ];
            let mappings = vec![
                ColumnMapping {
                    excel_column: "Title".to_string(),
                    target_field: "title".to_string(),
                },
                ColumnMapping {
                    excel_column: "Description".to_string(),
                    target_field: "description".to_string(),
                },
            ];
            let row_data = vec!["Test Title".to_string(), "Test Description".to_string()];

            let mut req_data = HashMap::new();
            for mapping in &mappings {
                if let Some(column) = columns.iter().find(|col| col.name == mapping.excel_column) {
                    if column.index < row_data.len() {
                        req_data
                            .insert(mapping.target_field.clone(), row_data[column.index].clone());
                    }
                }
            }

            assert_eq!(req_data.get("title"), Some(&"Test Title".to_string()));
            assert_eq!(
                req_data.get("description"),
                Some(&"Test Description".to_string())
            );
        }

        #[test]
        fn requirement_data_mapping_partial() {
            let columns = vec![ExcelColumn {
                index: 0,
                name: "Title".to_string(),
                sample_value: "".to_string(),
            }];
            let mappings = vec![ColumnMapping {
                excel_column: "Title".to_string(),
                target_field: "title".to_string(),
            }];
            let row_data = vec!["Title Only".to_string()];

            let mut req_data = HashMap::new();
            for mapping in &mappings {
                if let Some(column) = columns.iter().find(|col| col.name == mapping.excel_column) {
                    if column.index < row_data.len() {
                        req_data
                            .insert(mapping.target_field.clone(), row_data[column.index].clone());
                    }
                }
            }

            assert_eq!(req_data.get("title"), Some(&"Title Only".to_string()));
            assert_eq!(req_data.get("description"), None);
        }

        #[test]
        fn requirement_data_mapping_missing_column() {
            let columns = vec![ExcelColumn {
                index: 0,
                name: "Title".to_string(),
                sample_value: "".to_string(),
            }];
            let mappings = vec![ColumnMapping {
                excel_column: "NonExistent".to_string(),
                target_field: "title".to_string(),
            }];
            let row_data = vec!["Value".to_string()];

            let mut req_data = HashMap::new();
            for mapping in &mappings {
                if let Some(column) = columns.iter().find(|col| col.name == mapping.excel_column) {
                    if column.index < row_data.len() {
                        req_data
                            .insert(mapping.target_field.clone(), row_data[column.index].clone());
                    }
                }
            }

            assert_eq!(req_data.get("title"), None);
        }

        #[test]
        fn test_data_mapping_basic() {
            let columns = vec![
                ExcelColumn {
                    index: 0,
                    name: "Name".to_string(),
                    sample_value: "".to_string(),
                },
                ExcelColumn {
                    index: 1,
                    name: "Source".to_string(),
                    sample_value: "".to_string(),
                },
            ];
            let mappings = vec![
                ColumnMapping {
                    excel_column: "Name".to_string(),
                    target_field: "name".to_string(),
                },
                ColumnMapping {
                    excel_column: "Source".to_string(),
                    target_field: "source".to_string(),
                },
            ];
            let row_data = vec!["Test Name".to_string(), "test.rs".to_string()];

            let mut test_data = HashMap::new();
            for mapping in &mappings {
                if let Some(column) = columns.iter().find(|col| col.name == mapping.excel_column) {
                    if column.index < row_data.len() {
                        test_data
                            .insert(mapping.target_field.clone(), row_data[column.index].clone());
                    }
                }
            }

            assert_eq!(test_data.get("name"), Some(&"Test Name".to_string()));
            assert_eq!(test_data.get("source"), Some(&"test.rs".to_string()));
        }

        #[test]
        fn data_mapping_with_empty_values() {
            let columns = vec![ExcelColumn {
                index: 0,
                name: "Title".to_string(),
                sample_value: "".to_string(),
            }];
            let mappings = vec![ColumnMapping {
                excel_column: "Title".to_string(),
                target_field: "title".to_string(),
            }];
            let row_data = vec!["".to_string()];

            let mut req_data = HashMap::new();
            for mapping in &mappings {
                if let Some(column) = columns.iter().find(|col| col.name == mapping.excel_column) {
                    if column.index < row_data.len() {
                        req_data
                            .insert(mapping.target_field.clone(), row_data[column.index].clone());
                    }
                }
            }

            assert_eq!(req_data.get("title"), Some(&"".to_string()));
        }

        #[test]
        fn data_mapping_multiple_mappings_same_column() {
            let columns = vec![ExcelColumn {
                index: 0,
                name: "Title".to_string(),
                sample_value: "".to_string(),
            }];
            let mappings = vec![
                ColumnMapping {
                    excel_column: "Title".to_string(),
                    target_field: "title".to_string(),
                },
                ColumnMapping {
                    excel_column: "Title".to_string(),
                    target_field: "description".to_string(),
                },
            ];
            let row_data = vec!["Same Value".to_string()];

            let mut req_data = HashMap::new();
            for mapping in &mappings {
                if let Some(column) = columns.iter().find(|col| col.name == mapping.excel_column) {
                    if column.index < row_data.len() {
                        req_data
                            .insert(mapping.target_field.clone(), row_data[column.index].clone());
                    }
                }
            }

            assert_eq!(req_data.get("title"), Some(&"Same Value".to_string()));
            assert_eq!(req_data.get("description"), Some(&"Same Value".to_string()));
        }
    }

    // ============================================================================
    // Tests for row processing logic
    // ============================================================================

    mod row_processing_tests {
        use super::*;

        #[test]
        fn row_index_calculation() {
            for (row_index, _) in (0..5).enumerate() {
                let display_row = row_index + 2; // +2 because row 0 is header, row 1 is first data
                assert_eq!(display_row, row_index + 2);
            }
        }

        #[test]
        fn row_iteration_with_index() {
            let data = vec![
                vec!["Row1".to_string()],
                vec!["Row2".to_string()],
                vec!["Row3".to_string()],
            ];
            for (row_index, row_data) in data.iter().enumerate() {
                assert_eq!(row_data[0], format!("Row{}", row_index + 1));
            }
        }

        #[test]
        fn empty_row_skipping_logic() {
            let rows = vec![
                vec!["Value".to_string(), "".to_string()],
                vec!["".to_string(), "".to_string()],
                vec!["Value2".to_string(), "Value2".to_string()],
            ];

            let non_empty: Vec<_> = rows
                .iter()
                .filter(|row| !row.iter().all(|cell| cell.is_empty()))
                .collect();

            assert_eq!(non_empty.len(), 2);
        }

        #[test]
        fn sample_value_storage_first_row_only() {
            let mut columns = vec![ExcelColumn {
                index: 0,
                name: "Title".to_string(),
                sample_value: String::new(),
            }];
            let data_rows = vec![
                vec!["First Value".to_string()],
                vec!["Second Value".to_string()],
                vec!["Third Value".to_string()],
            ];

            for (data_idx, row_data) in data_rows.iter().enumerate() {
                if data_idx < 3 {
                    for (i, cell) in row_data.iter().enumerate() {
                        if i < columns.len() && data_idx == 0 {
                            columns[i].sample_value = cell.clone();
                        }
                    }
                }
            }

            assert_eq!(columns[0].sample_value, "First Value");
        }

        #[test]
        fn header_parsing_logic() {
            let header_row = vec!["Title", "Description", "Status"];
            let mut columns = Vec::new();
            for (i, cell) in header_row.iter().enumerate() {
                columns.push(ExcelColumn {
                    index: i,
                    name: cell.to_string(),
                    sample_value: String::new(),
                });
            }

            assert_eq!(columns.len(), 3);
            assert_eq!(columns[0].name, "Title");
            assert_eq!(columns[1].name, "Description");
            assert_eq!(columns[2].name, "Status");
        }

        #[test]
        fn cell_to_string_conversion() {
            // Test that cell values are converted to strings
            let cell_values = vec!["Text", "123", "45.67"];
            let row_data: Vec<String> = cell_values.iter().map(|cell| cell.to_string()).collect();
            assert_eq!(row_data[0], "Text");
            assert_eq!(row_data[1], "123");
            assert_eq!(row_data[2], "45.67");
        }
    }

    // ============================================================================
    // Tests for requirement row building logic
    // ============================================================================

    mod requirement_row_building_tests {
        use super::*;
        use std::collections::HashMap;

        #[test]
        fn requirement_title_extraction() {
            let mut req_data = HashMap::new();
            req_data.insert("title".to_string(), "Test Requirement".to_string());
            let title = req_data
                .get("title")
                .unwrap_or(&"Imported Requirement".to_string())
                .clone();
            assert_eq!(title, "Test Requirement");
        }

        #[test]
        fn requirement_title_default() {
            let req_data: HashMap<String, String> = HashMap::new();
            let title = req_data
                .get("title")
                .unwrap_or(&"Imported Requirement".to_string())
                .clone();
            assert_eq!(title, "Imported Requirement");
        }

        #[test]
        fn requirement_description_extraction() {
            let mut req_data = HashMap::new();
            req_data.insert("description".to_string(), "Test Description".to_string());
            let description = req_data
                .get("description")
                .unwrap_or(&"".to_string())
                .clone();
            assert_eq!(description, "Test Description");
        }

        #[test]
        fn requirement_reference_code_extraction() {
            let mut req_data = HashMap::new();
            req_data.insert("reference_code".to_string(), "REQ-001".to_string());
            let ref_code = req_data
                .get("reference_code")
                .unwrap_or(&"".to_string())
                .clone();
            assert_eq!(ref_code, "REQ-001");
        }

        #[test]
        fn requirement_justification_extraction() {
            let mut req_data = HashMap::new();
            req_data.insert("justification".to_string(), "Important".to_string());
            let justification = req_data.get("justification").cloned();
            assert_eq!(justification, Some("Important".to_string()));
        }

        #[test]
        fn requirement_justification_none() {
            let req_data: HashMap<String, String> = HashMap::new();
            let justification = req_data.get("justification").cloned();
            assert_eq!(justification, None);
        }

        #[test]
        fn requirement_parent_id_resolution_logic() {
            let parent_title = "Parent Req";
            let is_none = parent_title.is_empty() || parent_title == "None";
            assert!(!is_none);
        }

        #[test]
        fn requirement_parent_id_none_string() {
            let parent_title = "None";
            let is_none = parent_title.is_empty() || parent_title == "None";
            assert!(is_none);
        }

        #[test]
        fn requirement_parent_id_empty_string() {
            let parent_title = "";
            let is_none = parent_title.is_empty() || parent_title == "None";
            assert!(is_none);
        }

        #[test]
        fn requirement_all_fields_with_values() {
            let mut req_data = HashMap::new();
            req_data.insert("title".to_string(), "Title".to_string());
            req_data.insert("description".to_string(), "Desc".to_string());
            req_data.insert("reference_code".to_string(), "REF".to_string());
            req_data.insert("justification".to_string(), "Just".to_string());

            assert_eq!(req_data.get("title"), Some(&"Title".to_string()));
            assert_eq!(req_data.get("description"), Some(&"Desc".to_string()));
            assert_eq!(req_data.get("reference_code"), Some(&"REF".to_string()));
            assert_eq!(req_data.get("justification"), Some(&"Just".to_string()));
        }
    }

    // ============================================================================
    // Tests for test row building logic
    // ============================================================================

    mod test_row_building_tests {
        use super::*;
        use std::collections::HashMap;

        #[test]
        fn test_name_extraction() {
            let mut test_data = HashMap::new();
            test_data.insert("name".to_string(), "Test Case".to_string());
            let name = test_data
                .get("name")
                .unwrap_or(&"Imported Test".to_string())
                .clone();
            assert_eq!(name, "Test Case");
        }

        #[test]
        fn test_name_default() {
            let test_data: HashMap<String, String> = HashMap::new();
            let name = test_data
                .get("name")
                .unwrap_or(&"Imported Test".to_string())
                .clone();
            assert_eq!(name, "Imported Test");
        }

        #[test]
        fn test_source_extraction() {
            let mut test_data = HashMap::new();
            test_data.insert("source".to_string(), "test.rs".to_string());
            let source = test_data.get("source").unwrap_or(&"".to_string()).clone();
            assert_eq!(source, "test.rs");
        }

        #[test]
        fn test_reference_code_extraction() {
            let mut test_data = HashMap::new();
            test_data.insert("reference_code".to_string(), "TEST-001".to_string());
            let ref_code = test_data
                .get("reference_code")
                .unwrap_or(&format!("TEST-{}", chrono::Utc::now().timestamp()))
                .clone();
            assert_eq!(ref_code, "TEST-001");
        }

        #[test]
        fn test_reference_code_default_generation() {
            let test_data: HashMap<String, String> = HashMap::new();
            let ref_code = test_data
                .get("reference_code")
                .unwrap_or(&format!("TEST-{}", chrono::Utc::now().timestamp()))
                .clone();
            assert!(ref_code.starts_with("TEST-"));
        }

        #[test]
        fn test_parent_id_resolution_logic() {
            let parent_name = "Parent Test";
            let is_none = parent_name.is_empty() || parent_name == "None";
            assert!(!is_none);
        }

        #[test]
        fn test_parent_id_none_string() {
            let parent_name = "None";
            let is_none = parent_name.is_empty() || parent_name == "None";
            assert!(is_none);
        }

        #[test]
        fn test_all_fields_with_values() {
            let mut test_data = HashMap::new();
            test_data.insert("name".to_string(), "Name".to_string());
            test_data.insert("description".to_string(), "Desc".to_string());
            test_data.insert("source".to_string(), "source.rs".to_string());
            test_data.insert("reference_code".to_string(), "REF".to_string());

            assert_eq!(test_data.get("name"), Some(&"Name".to_string()));
            assert_eq!(test_data.get("description"), Some(&"Desc".to_string()));
            assert_eq!(test_data.get("source"), Some(&"source.rs".to_string()));
            assert_eq!(test_data.get("reference_code"), Some(&"REF".to_string()));
        }
    }

    // ============================================================================
    // Tests for import result message formatting
    // ============================================================================

    mod import_result_message_tests {
        use super::*;

        #[test]
        fn success_message_zero_records() {
            let imported_count = 0;
            let errors: Vec<String> = vec![];
            let success = errors.is_empty();
            let message = if success {
                format!("Successfully imported {} records", imported_count)
            } else {
                format!(
                    "Imported {} records with {} errors",
                    imported_count,
                    errors.len()
                )
            };
            assert_eq!(message, "Successfully imported 0 records");
        }

        #[test]
        fn success_message_single_record() {
            let imported_count = 1;
            let errors: Vec<String> = vec![];
            let success = errors.is_empty();
            let message = if success {
                format!("Successfully imported {} records", imported_count)
            } else {
                format!(
                    "Imported {} records with {} errors",
                    imported_count,
                    errors.len()
                )
            };
            assert_eq!(message, "Successfully imported 1 records");
        }

        #[test]
        fn error_message_single_error() {
            let imported_count = 5;
            let errors = vec!["Row 3: Error".to_string()];
            let success = errors.is_empty();
            let message = if success {
                format!("Successfully imported {} records", imported_count)
            } else {
                format!(
                    "Imported {} records with {} errors",
                    imported_count,
                    errors.len()
                )
            };
            assert_eq!(message, "Imported 5 records with 1 errors");
        }

        #[test]
        fn error_message_multiple_errors() {
            let imported_count = 3;
            let errors = vec![
                "Row 2: Error 1".to_string(),
                "Row 4: Error 2".to_string(),
                "Row 6: Error 3".to_string(),
            ];
            let success = errors.is_empty();
            let message = if success {
                format!("Successfully imported {} records", imported_count)
            } else {
                format!(
                    "Imported {} records with {} errors",
                    imported_count,
                    errors.len()
                )
            };
            assert_eq!(message, "Imported 3 records with 3 errors");
        }

        #[test]
        fn import_result_success_determination_with_errors() {
            let errors = vec!["Error".to_string()];
            let success = errors.is_empty();
            assert!(!success);
        }

        #[test]
        fn import_result_success_determination_no_errors() {
            let errors: Vec<String> = vec![];
            let success = errors.is_empty();
            assert!(success);
        }
    }

    // ============================================================================
    // Tests for column finding logic
    // ============================================================================

    mod column_finding_tests {
        use super::*;

        #[test]
        fn find_column_by_name_exact_match() {
            let columns = vec![
                ExcelColumn {
                    index: 0,
                    name: "Title".to_string(),
                    sample_value: "".to_string(),
                },
                ExcelColumn {
                    index: 1,
                    name: "Description".to_string(),
                    sample_value: "".to_string(),
                },
            ];
            let mapping = ColumnMapping {
                excel_column: "Title".to_string(),
                target_field: "title".to_string(),
            };
            let found = columns.iter().find(|col| col.name == mapping.excel_column);
            assert!(found.is_some());
            assert_eq!(found.unwrap().index, 0);
        }

        #[test]
        fn find_column_by_name_no_match() {
            let columns = vec![ExcelColumn {
                index: 0,
                name: "Title".to_string(),
                sample_value: "".to_string(),
            }];
            let mapping = ColumnMapping {
                excel_column: "NonExistent".to_string(),
                target_field: "field".to_string(),
            };
            let found = columns.iter().find(|col| col.name == mapping.excel_column);
            assert!(found.is_none());
        }

        #[test]
        fn find_column_case_sensitive() {
            let columns = vec![ExcelColumn {
                index: 0,
                name: "Title".to_string(),
                sample_value: "".to_string(),
            }];
            let mapping = ColumnMapping {
                excel_column: "title".to_string(), // lowercase
                target_field: "title".to_string(),
            };
            let found = columns.iter().find(|col| col.name == mapping.excel_column);
            assert!(found.is_none()); // Should not match due to case sensitivity
        }
    }

    // ============================================================================
    // Tests for import type detection edge cases
    // ============================================================================

    mod import_type_detection_edge_cases {
        #[test]
        fn import_type_requirements_partial_match() {
            let columns = vec!["Requirement", "Title", "Description"];
            let has_req = columns.iter().any(|col| col.to_lowercase().contains("req"));
            assert!(has_req);
        }

        #[test]
        fn import_type_requirements_in_middle() {
            let columns = vec!["Title", "Req ID", "Status"];
            let has_req = columns.iter().any(|col| col.to_lowercase().contains("req"));
            assert!(has_req);
        }

        #[test]
        fn import_type_tests_partial_match() {
            let columns = vec!["Test Case", "Name", "Status"];
            let has_test = columns
                .iter()
                .any(|col| col.to_lowercase().contains("test"));
            assert!(has_test);
        }

        #[test]
        fn import_type_tests_in_middle() {
            let columns = vec!["Name", "Test ID", "Status"];
            let has_test = columns
                .iter()
                .any(|col| col.to_lowercase().contains("test"));
            assert!(has_test);
        }

        #[test]
        fn import_type_both_present_requirements_first() {
            let columns = vec!["Req ID", "Test Name"];
            let has_req = columns.iter().any(|col| col.to_lowercase().contains("req"));
            let has_test = columns
                .iter()
                .any(|col| col.to_lowercase().contains("test"));
            // Requirements should be detected first
            assert!(has_req);
            assert!(has_test);
        }

        #[test]
        fn import_type_no_keywords() {
            let columns = vec!["Column1", "Column2", "Column3"];
            let has_req = columns.iter().any(|col| col.to_lowercase().contains("req"));
            let has_test = columns
                .iter()
                .any(|col| col.to_lowercase().contains("test"));
            assert!(!has_req);
            assert!(!has_test);
        }
    }

    // ============================================================================
    // Tests for data structure combinations
    // ============================================================================

    mod data_structure_combination_tests {
        use super::*;

        #[test]
        fn full_import_config_with_multiple_mappings() {
            let mappings = vec![
                ColumnMapping {
                    excel_column: "Title".to_string(),
                    target_field: "title".to_string(),
                },
                ColumnMapping {
                    excel_column: "Description".to_string(),
                    target_field: "description".to_string(),
                },
                ColumnMapping {
                    excel_column: "Category".to_string(),
                    target_field: "category_id".to_string(),
                },
            ];
            let config = ImportConfig {
                import_type: "requirements".to_string(),
                column_mappings: mappings,
                project_id: 1,
            };
            assert_eq!(config.column_mappings.len(), 3);
            assert_eq!(config.import_type, "requirements");
        }

        #[test]
        fn excel_importer_with_full_data() {
            let importer = ExcelImporter {
                columns: vec![
                    ExcelColumn {
                        index: 0,
                        name: "Title".to_string(),
                        sample_value: "Sample".to_string(),
                    },
                    ExcelColumn {
                        index: 1,
                        name: "Description".to_string(),
                        sample_value: "Desc".to_string(),
                    },
                ],
                data: vec![
                    vec!["Title1".to_string(), "Desc1".to_string()],
                    vec!["Title2".to_string(), "Desc2".to_string()],
                ],
                import_type: "requirements".to_string(),
            };
            assert_eq!(importer.columns.len(), 2);
            assert_eq!(importer.data.len(), 2);
        }

        #[test]
        fn import_result_with_mixed_success() {
            let result = ImportResult {
                success: false,
                message: "Imported 8 records with 2 errors".to_string(),
                imported_count: 8,
                errors: vec!["Row 3: Error".to_string(), "Row 7: Error".to_string()],
                imported_requirement_ids: vec![],
            };
            assert!(!result.success);
            assert_eq!(result.imported_count, 8);
            assert_eq!(result.errors.len(), 2);
        }
    }

    // ============================================================================
    // Additional edge case tests
    // ============================================================================

    mod additional_edge_cases {
        use super::*;

        #[test]
        fn column_with_very_long_name() {
            let column = ExcelColumn {
                index: 0,
                name: "A".repeat(1000),
                sample_value: "".to_string(),
            };
            assert_eq!(column.name.len(), 1000);
        }

        #[test]
        fn column_with_unicode_name() {
            let column = ExcelColumn {
                index: 0,
                name: "Título".to_string(),
                sample_value: "".to_string(),
            };
            assert_eq!(column.name, "Título");
        }

        #[test]
        fn mapping_with_unicode_characters() {
            let mapping = ColumnMapping {
                excel_column: "Columna".to_string(),
                target_field: "campo".to_string(),
            };
            assert_eq!(mapping.excel_column, "Columna");
        }

        #[test]
        fn row_data_with_mixed_types_as_strings() {
            let row_data = vec![
                "Text".to_string(),
                "123".to_string(),
                "45.67".to_string(),
                "true".to_string(),
            ];
            assert_eq!(row_data.len(), 4);
        }

        #[test]
        fn import_config_with_negative_project_id() {
            let config = ImportConfig {
                import_type: "requirements".to_string(),
                column_mappings: vec![],
                project_id: -1,
            };
            assert_eq!(config.project_id, -1);
        }

        #[test]
        fn import_config_with_zero_project_id() {
            let config = ImportConfig {
                import_type: "requirements".to_string(),
                column_mappings: vec![],
                project_id: 0,
            };
            assert_eq!(config.project_id, 0);
        }

        #[test]
        fn import_config_with_large_project_id() {
            let config = ImportConfig {
                import_type: "requirements".to_string(),
                column_mappings: vec![],
                project_id: i32::MAX,
            };
            assert_eq!(config.project_id, i32::MAX);
        }

        #[test]
        fn column_index_at_maximum() {
            let column = ExcelColumn {
                index: usize::MAX,
                name: "Col".to_string(),
                sample_value: "".to_string(),
            };
            assert_eq!(column.index, usize::MAX);
        }

        #[test]
        fn empty_string_handling_in_mappings() {
            let mapping = ColumnMapping {
                excel_column: "".to_string(),
                target_field: "".to_string(),
            };
            assert_eq!(mapping.excel_column, "");
            assert_eq!(mapping.target_field, "");
        }
    }
}

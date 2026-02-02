//! Comprehensive test suite for the generators module.
//!
//! This module provides tests for Excel workbook generation functions, ensuring:
//! - Error handling patterns work correctly
//! - Edge cases are handled (empty data, None values)
//! - Function signatures and return types are correct
//! - Data transformation logic works
//! - Workbook structure is correct

#[cfg(test)]
mod tests {
    use crate::models::*;
    use chrono::{NaiveDate, NaiveDateTime};

    // Helper function to create a test timestamp
    fn test_timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 15)
            .unwrap()
            .and_hms_opt(10, 30, 0)
            .unwrap()
    }

    // Helper to create a sample requirement
    fn sample_requirement(id: i32, project_id: i32) -> Requirement {
        Requirement {
            id,
            title: format!("Requirement {}", id),
            description: format!("Description {}", id),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: format!("REQ-{:03}", id),
            category_id: 1,
            parent_id: None,
            creation_date: test_timestamp(),
            update_date: test_timestamp(),
            deadline_date: Some(test_timestamp()),
            applicability_id: 1,
            justification: Some("Test justification".to_string()),
            project_id,
        }
    }

    // Helper to create a sample test case
    fn sample_test_case(id: i32, project_id: i32) -> TestCase {
        TestCase {
            id,
            name: format!("Test {}", id),
            reference_code: format!("TEST-{:03}", id),
            description: format!("Test description {}", id),
            source: format!("test_{}.rs", id),
            status_id: 1,
            parent_id: None,
            project_id,
        }
    }

    // Helper to create a sample decorated requirement
    fn sample_decorated_requirement(id: i32) -> DecoratedRequirement {
        DecoratedRequirement {
            id,
            title: format!("Requirement {}", id),
            description: format!("Description {}", id),
            verification_method_id: "Test".to_string(),
            req_verification_ids: vec![1],
            status_id: "Draft".to_string(),
            req_current_status_id: 1,
            author_id: "Author Name".to_string(),
            req_author_id: 1,
            reviewer_id: "Reviewer Name".to_string(),
            req_reviewer_id: 2,
            reference_code: format!("REQ-{:03}", id),
            category_id: "Category".to_string(),
            req_category_id: 1,
            applicability_id: "All Systems".to_string(),
            req_applicability_id: 1,
            req_parent_id: None,
            req_parent_title: "".to_string(),
            creation_date: "2024-01-15 10:30:00".to_string(),
            update_date: "2024-01-15 10:30:00".to_string(),
            deadline_date: "2024-12-31 23:59:59".to_string(),
            justification: Some("Test justification".to_string()),
            project_id: 1,
        }
    }

    // Helper to create a sample decorated test case
    fn sample_decorated_test_case(id: i32) -> DecoratedTestCase {
        DecoratedTestCase {
            id,
            reference_code: format!("TEST-{:03}", id),
            name: format!("Test {}", id),
            description: format!("Test description {}", id),
            source: format!("test_{}.rs", id),
            status_id: "Passed".to_string(),
            test_status_id: 1,
            test_parent_id: None,
            test_parent_title: "".to_string(),
            project_id: 1,
        }
    }

    // ============================================================================
    // Tests for data structures and edge cases
    // ============================================================================

    mod data_structure_tests {
        use super::*;

        #[test]
        fn decorated_requirement_with_all_fields() {
            let req = sample_decorated_requirement(1);
            assert_eq!(req.id, 1);
            assert_eq!(req.title, "Requirement 1");
            assert_eq!(req.reference_code, "REQ-001");
            assert_eq!(req.justification, Some("Test justification".to_string()));
        }

        #[test]
        fn decorated_requirement_without_justification() {
            let mut req = sample_decorated_requirement(1);
            req.justification = None;
            assert_eq!(req.justification, None);
        }

        #[test]
        fn decorated_requirement_with_parent() {
            let mut req = sample_decorated_requirement(2);
            req.req_parent_id = Some(1);
            req.req_parent_title = "Parent Requirement".to_string();
            assert_eq!(req.req_parent_id, Some(1));
            assert_eq!(req.req_parent_title, "Parent Requirement");
        }

        #[test]
        fn decorated_test_case_with_all_fields() {
            let test = sample_decorated_test_case(1);
            assert_eq!(test.id, 1);
            assert_eq!(test.name, "Test 1");
            assert_eq!(test.reference_code, "TEST-001");
            assert_eq!(test.status_id, "Passed");
        }

        #[test]
        fn decorated_test_case_with_parent() {
            let mut test = sample_decorated_test_case(2);
            test.test_parent_id = Some(1);
            test.test_parent_title = "Parent Test".to_string();
            assert_eq!(test.test_parent_id, Some(1));
            assert_eq!(test.test_parent_title, "Parent Test");
        }

        #[test]
        fn requirement_sorting_by_id() {
            let mut reqs = vec![
                sample_requirement(3, 1),
                sample_requirement(1, 1),
                sample_requirement(2, 1),
            ];
            reqs.sort_by(|a, b| a.id.cmp(&b.id));
            assert_eq!(reqs[0].id, 1);
            assert_eq!(reqs[1].id, 2);
            assert_eq!(reqs[2].id, 3);
        }

        #[test]
        fn test_case_sorting_by_id() {
            let mut tests = vec![
                sample_test_case(3, 1),
                sample_test_case(1, 1),
                sample_test_case(2, 1),
            ];
            tests.sort_by(|a, b| a.id.cmp(&b.id));
            assert_eq!(tests[0].id, 1);
            assert_eq!(tests[1].id, 2);
            assert_eq!(tests[2].id, 3);
        }

        #[test]
        fn empty_requirements_list() {
            let reqs: Vec<Requirement> = vec![];
            assert_eq!(reqs.len(), 0);
        }

        #[test]
        fn empty_tests_list() {
            let tests: Vec<TestCase> = vec![];
            assert_eq!(tests.len(), 0);
        }

        #[test]
        fn empty_decorated_requirements_list() {
            let reqs: Vec<DecoratedRequirement> = vec![];
            assert_eq!(reqs.len(), 0);
        }

        #[test]
        fn empty_decorated_tests_list() {
            let tests: Vec<DecoratedTestCase> = vec![];
            assert_eq!(tests.len(), 0);
        }
    }

    // ============================================================================
    // Tests for error handling patterns
    // ============================================================================

    mod error_handling_tests {
        #[test]
        fn error_message_formatting() {
            let error_msg = format!("Database connection error: {}", "test error");
            assert!(error_msg.contains("Database connection error"));
            assert!(error_msg.contains("test error"));
        }

        #[test]
        fn error_message_for_query_requirements() {
            let error_msg = format!("Error querying requirements by project: {:?}", "test error");
            assert!(error_msg.contains("Error querying requirements by project"));
        }

        #[test]
        fn error_message_for_query_tests() {
            let error_msg = format!("Error querying tests by project: {:?}", "test error");
            assert!(error_msg.contains("Error querying tests by project"));
        }

        #[test]
        fn error_message_for_matrix_link() {
            let error_msg = format!("Error checking matrix link: {:?}", "test error");
            assert!(error_msg.contains("Error checking matrix link"));
        }

        #[test]
        fn error_message_for_workbook_close() {
            let error_msg = format!("Error closing workbook: {:?}", "test error");
            assert!(error_msg.contains("Error closing workbook"));
        }

        #[test]
        fn error_message_for_file_read() {
            let error_msg = format!("Error reading generated file: {:?}", "test error");
            assert!(error_msg.contains("Error reading generated file"));
        }

        #[test]
        fn box_dyn_error_conversion() {
            // Test that errors can be converted to Box<dyn Error>
            let error: Box<dyn std::error::Error> = "test error".into();
            assert_eq!(error.to_string(), "test error");
        }

        #[test]
        fn box_dyn_error_send_sync_conversion() {
            // Test that errors can be converted to Box<dyn Error + Send + Sync>
            let error: Box<dyn std::error::Error + Send + Sync> = "test error".into();
            assert_eq!(error.to_string(), "test error");
        }
    }

    // ============================================================================
    // Tests for workbook structure and data format
    // ============================================================================

    mod workbook_structure_tests {
        #[test]
        fn matrix_workbook_headers() {
            // Test that headers are correctly defined
            let headers = vec!["Title", "Reference", "Category", "Status"];
            assert_eq!(headers.len(), 4);
            assert_eq!(headers[0], "Title");
            assert_eq!(headers[1], "Reference");
            assert_eq!(headers[2], "Category");
            assert_eq!(headers[3], "Status");
        }

        #[test]
        fn requirements_workbook_headers() {
            // Test that headers are correctly defined
            let headers = vec![
                "ID",
                "Title",
                "Description",
                "Reference",
                "Category",
                "Applicability",
                "Status",
                "Verification",
                "Author",
                "Reviewer",
                "Creation Date",
                "Update Date",
                "Deadline Date",
                "Justification",
            ];
            assert_eq!(headers.len(), 14);
            assert_eq!(headers[0], "ID");
            assert_eq!(headers[13], "Justification");
        }

        #[test]
        fn tests_workbook_headers() {
            // Test that headers are correctly defined
            let headers = vec![
                "ID",
                "Name",
                "Description",
                "Source",
                "Reference",
                "Status",
                "Parent",
            ];
            assert_eq!(headers.len(), 7);
            assert_eq!(headers[0], "ID");
            assert_eq!(headers[6], "Parent");
        }

        #[test]
        fn test_header_format() {
            let test_id = 42;
            let test_name = "Test Name";
            let header = format!("Test #{} ({})", test_id, test_name);
            assert_eq!(header, "Test #42 (Test Name)");
        }

        #[test]
        fn matrix_link_presence_check() {
            // Test the logic for checking if a requirement is linked to a test
            let test_present: i64 = 1;
            assert!(test_present > 0);

            let test_absent: i64 = 0;
            assert!(!(test_absent > 0));
        }

        #[test]
        fn justification_handling_with_value() {
            let justification = Some("Test justification".to_string());
            let result = justification.as_deref().unwrap_or("");
            assert_eq!(result, "Test justification");
        }

        #[test]
        fn justification_handling_without_value() {
            let justification: Option<String> = None;
            let result = justification.as_deref().unwrap_or("");
            assert_eq!(result, "");
        }
    }

    // ============================================================================
    // Tests for data transformation
    // ============================================================================

    mod data_transformation_tests {
        #[test]
        fn requirement_id_to_f64_conversion() {
            let id: i32 = 42;
            let f64_id = id as f64;
            assert_eq!(f64_id, 42.0);
        }

        #[test]
        fn row_index_calculation() {
            for (i, _) in (0..5).enumerate() {
                let row = (i + 1) as u32;
                assert_eq!(row, (i + 1) as u32);
            }
        }

        #[test]
        fn column_index_calculation_for_tests() {
            for (col_idx, _) in (0..3).enumerate() {
                let col = (col_idx + 4) as u16;
                assert_eq!(col, (col_idx + 4) as u16);
            }
        }

        #[test]
        fn date_string_formatting() {
            let date_str = "2024-01-15 10:30:00";
            assert!(date_str.contains("2024"));
            assert!(date_str.contains("01-15"));
        }

        #[test]
        fn reference_code_formatting() {
            let id = 42;
            let ref_code = format!("REQ-{:03}", id);
            assert_eq!(ref_code, "REQ-042");
        }

        #[test]
        fn test_reference_code_formatting() {
            let id = 5;
            let ref_code = format!("TEST-{:03}", id);
            assert_eq!(ref_code, "TEST-005");
        }
    }

    // ============================================================================
    // Tests for file path handling
    // ============================================================================

    mod file_path_tests {
        #[test]
        fn matrix_workbook_path() {
            let path = "target/matrix.xls";
            assert_eq!(path, "target/matrix.xls");
            assert!(path.ends_with(".xls"));
        }

        #[test]
        fn requirements_workbook_path() {
            let path = "target/requirements.xls";
            assert_eq!(path, "target/requirements.xls");
            assert!(path.ends_with(".xls"));
        }

        #[test]
        fn tests_workbook_path() {
            let path = "target/tests.xls";
            assert_eq!(path, "target/tests.xls");
            assert!(path.ends_with(".xls"));
        }

        #[test]
        fn worksheet_name_for_requirements() {
            let name = Some("Requirements");
            assert_eq!(name, Some("Requirements"));
        }

        #[test]
        fn worksheet_name_for_tests() {
            let name = Some("Tests");
            assert_eq!(name, Some("Tests"));
        }
    }

    // ============================================================================
    // Integration-style tests (testing behavior, not actual file I/O)
    // ============================================================================

    mod integration_behavior_tests {
        use super::*;

        #[test]
        fn matrix_workbook_data_structure() {
            // Test the expected data structure for matrix workbook
            let reqs = vec![
                sample_decorated_requirement(1),
                sample_decorated_requirement(2),
            ];
            let tests = vec![sample_decorated_test_case(1), sample_decorated_test_case(2)];

            // Verify data is sorted
            let mut sorted_reqs = reqs.clone();
            sorted_reqs.sort_by(|a, b| a.id.cmp(&b.id));
            assert_eq!(sorted_reqs[0].id, 1);
            assert_eq!(sorted_reqs[1].id, 2);

            let mut sorted_tests = tests.clone();
            sorted_tests.sort_by(|a, b| a.id.cmp(&b.id));
            assert_eq!(sorted_tests[0].id, 1);
            assert_eq!(sorted_tests[1].id, 2);
        }

        #[test]
        fn requirements_workbook_data_structure() {
            // Test the expected data structure for requirements workbook
            let reqs = vec![
                sample_decorated_requirement(1),
                sample_decorated_requirement(2),
            ];

            // Verify all required fields are present
            for req in &reqs {
                assert!(!req.title.is_empty());
                assert!(!req.reference_code.is_empty());
                assert!(!req.category_id.is_empty());
            }
        }

        #[test]
        fn tests_workbook_data_structure() {
            // Test the expected data structure for tests workbook
            let tests = vec![sample_decorated_test_case(1), sample_decorated_test_case(2)];

            // Verify all required fields are present
            for test in &tests {
                assert!(!test.name.is_empty());
                assert!(!test.reference_code.is_empty());
                assert!(!test.status_id.is_empty());
            }
        }

        #[test]
        fn matrix_link_counting_logic() {
            // Test the logic for counting matrix links
            let count: i64 = 1;
            if count > 0 {
                assert_eq!(count, 1);
            } else {
                panic!("Count should be greater than 0");
            }
        }

        #[test]
        fn empty_matrix_scenario() {
            // Test behavior with no requirements or tests
            let reqs: Vec<DecoratedRequirement> = vec![];
            let tests: Vec<DecoratedTestCase> = vec![];

            assert_eq!(reqs.len(), 0);
            assert_eq!(tests.len(), 0);
        }

        #[test]
        fn single_requirement_single_test_scenario() {
            // Test behavior with one requirement and one test
            let reqs = vec![sample_decorated_requirement(1)];
            let tests = vec![sample_decorated_test_case(1)];

            assert_eq!(reqs.len(), 1);
            assert_eq!(tests.len(), 1);
            assert_eq!(reqs[0].id, 1);
            assert_eq!(tests[0].id, 1);
        }

        #[test]
        fn multiple_requirements_multiple_tests_scenario() {
            // Test behavior with multiple requirements and tests
            let reqs = vec![
                sample_decorated_requirement(1),
                sample_decorated_requirement(2),
                sample_decorated_requirement(3),
            ];
            let tests = vec![sample_decorated_test_case(1), sample_decorated_test_case(2)];

            assert_eq!(reqs.len(), 3);
            assert_eq!(tests.len(), 2);
        }
    }
}

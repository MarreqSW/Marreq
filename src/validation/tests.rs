// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Comprehensive test suite for the validation module.
//!
//! This module provides tests for all validation functions, ensuring:
//! - All validation rules are tested
//! - Valid inputs pass validation
//! - Invalid inputs produce appropriate errors
//! - Edge cases and boundary values are covered
//! - Regex patterns work correctly

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::errors::ValidationError;
    use crate::status_enums::ProjectStatus;
    use crate::validation::*;

    // ============================================================================
    // Tests for validate_requirement
    // ============================================================================

    mod validate_requirement_tests {
        use super::*;

        fn valid_requirement() -> NewRequirement {
            NewRequirement {
                id: None,
                title: "Valid Requirement Title".to_string(),
                description: "Valid description text".to_string(),
                author_id: 1,
                category_id: 1,
                status_id: 1,
                reference_code: "REQ-001".to_string(),
                reviewer_id: 1,
                applicability_id: 1,
                justification: None,
                project_id: 1,
            }
        }

        #[test]
        fn valid_requirement_passes() {
            let req = valid_requirement();
            assert!(validate_requirement(&req).is_ok());
        }

        #[test]
        fn requirement_title_required() {
            let mut req = valid_requirement();
            req.title = "".to_string();
            let result = validate_requirement(&req);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::Required { field } => assert_eq!(field, "title"),
                _ => panic!("Expected Required error"),
            }
        }

        #[test]
        fn requirement_title_whitespace_only() {
            let mut req = valid_requirement();
            req.title = "   ".to_string();
            let result = validate_requirement(&req);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_title_too_short() {
            let mut req = valid_requirement();
            req.title = "AB".to_string(); // Less than 3
            let result = validate_requirement(&req);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TooShort { field, min } => {
                    assert_eq!(field, "title");
                    assert_eq!(min, 3);
                }
                _ => panic!("Expected TooShort error"),
            }
        }

        #[test]
        fn requirement_title_exactly_min_length() {
            let mut req = valid_requirement();
            req.title = "ABC".to_string(); // Exactly 3
            assert!(validate_requirement(&req).is_ok());
        }

        #[test]
        fn requirement_title_too_long() {
            let mut req = valid_requirement();
            req.title = "A".repeat(256); // More than 255
            let result = validate_requirement(&req);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TooLong { field, max } => {
                    assert_eq!(field, "title");
                    assert_eq!(max, 255);
                }
                _ => panic!("Expected TooLong error"),
            }
        }

        #[test]
        fn requirement_title_exactly_max_length() {
            let mut req = valid_requirement();
            req.title = "A".repeat(255); // Exactly 255
            assert!(validate_requirement(&req).is_ok());
        }

        #[test]
        fn requirement_description_required() {
            let mut req = valid_requirement();
            req.description = "".to_string();
            let result = validate_requirement(&req);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_description_whitespace_only() {
            let mut req = valid_requirement();
            req.description = "   ".to_string();
            let result = validate_requirement(&req);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_description_too_long() {
            let mut req = valid_requirement();
            req.description = "A".repeat(2001); // More than 2000
            let result = validate_requirement(&req);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TooLong { field, max } => {
                    assert_eq!(field, "description");
                    assert_eq!(max, 2000);
                }
                _ => panic!("Expected TooLong error"),
            }
        }

        #[test]
        fn requirement_description_exactly_max_length() {
            let mut req = valid_requirement();
            req.description = "A".repeat(2000); // Exactly 2000
            assert!(validate_requirement(&req).is_ok());
        }

        #[test]
        fn requirement_reference_code_valid_format() {
            let mut req = valid_requirement();
            req.reference_code = "REQ-001".to_string();
            assert!(validate_requirement(&req).is_ok());
        }

        #[test]
        fn requirement_reference_code_valid_with_letters() {
            let mut req = valid_requirement();
            req.reference_code = "REQ-ABC-001".to_string();
            assert!(validate_requirement(&req).is_ok());
        }

        #[test]
        fn requirement_reference_code_valid_short_prefix() {
            let mut req = valid_requirement();
            req.reference_code = "AB-001".to_string();
            assert!(validate_requirement(&req).is_ok());
        }

        #[test]
        fn requirement_reference_code_valid_long_prefix() {
            let mut req = valid_requirement();
            req.reference_code = "ABCD-001".to_string();
            assert!(validate_requirement(&req).is_ok());
        }

        #[test]
        fn requirement_reference_code_empty_allowed() {
            let mut req = valid_requirement();
            req.reference_code = "".to_string();
            assert!(validate_requirement(&req).is_ok());
        }

        #[test]
        fn requirement_reference_code_whitespace_only() {
            let mut req = valid_requirement();
            req.reference_code = "   ".to_string();
            assert!(validate_requirement(&req).is_ok()); // Empty after trim
        }

        #[test]
        fn requirement_reference_code_invalid_format_lowercase() {
            let mut req = valid_requirement();
            req.reference_code = "req-001".to_string();
            let result = validate_requirement(&req);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::InvalidFormat { field, .. } => {
                    assert_eq!(field, "reference_code");
                }
                _ => panic!("Expected InvalidFormat error"),
            }
        }

        #[test]
        fn requirement_reference_code_invalid_no_dash() {
            let mut req = valid_requirement();
            req.reference_code = "REQ001".to_string();
            let result = validate_requirement(&req);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_reference_code_invalid_special_chars() {
            let mut req = valid_requirement();
            req.reference_code = "REQ-001!".to_string();
            let result = validate_requirement(&req);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_status_id_zero() {
            let mut req = valid_requirement();
            req.status_id = 0;
            let result = validate_requirement(&req);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_status_id_negative() {
            let mut req = valid_requirement();
            req.status_id = -1;
            let result = validate_requirement(&req);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_author_id_zero() {
            let mut req = valid_requirement();
            req.author_id = 0;
            let result = validate_requirement(&req);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_reviewer_id_zero() {
            let mut req = valid_requirement();
            req.reviewer_id = 0;
            let result = validate_requirement(&req);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_category_id_zero() {
            let mut req = valid_requirement();
            req.category_id = 0;
            let result = validate_requirement(&req);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_project_id_zero() {
            let mut req = valid_requirement();
            req.project_id = 0;
            let result = validate_requirement(&req);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_project_id_negative() {
            let mut req = valid_requirement();
            req.project_id = -1;
            let result = validate_requirement(&req);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_all_ids_positive() {
            let req = valid_requirement();
            assert!(validate_requirement(&req).is_ok());
        }

        #[test]
        fn requirement_with_justification() {
            let mut req = valid_requirement();
            req.justification = Some("Test justification".to_string());
            assert!(validate_requirement(&req).is_ok());
        }
    }

    // ============================================================================
    // Tests for validate_test
    // ============================================================================

    mod validate_test_tests {
        use super::*;

        fn valid_test() -> NewVerification {
            NewVerification {
                id: None,
                reference_code: "TEST-1".to_string(),
                name: "Valid Test Name".to_string(),
                description: "Valid test description".to_string(),
                source: "test.rs".to_string(),
                status_id: 1,
                parent_id: None,
                project_id: 1,
                verification_method_id: None,
            }
        }

        #[test]
        fn valid_test_passes() {
            let test = valid_test();
            assert!(validate_verification(&test).is_ok());
        }

        #[test]
        fn test_name_required() {
            let mut test = valid_test();
            test.name = "".to_string();
            let result = validate_verification(&test);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::Required { field } => assert_eq!(field, "name"),
                _ => panic!("Expected Required error"),
            }
        }

        #[test]
        fn test_name_whitespace_only() {
            let mut test = valid_test();
            test.name = "   ".to_string();
            let result = validate_verification(&test);
            assert!(result.is_err());
        }

        #[test]
        fn test_name_too_short() {
            let mut test = valid_test();
            test.name = "AB".to_string();
            let result = validate_verification(&test);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TooShort { field, min } => {
                    assert_eq!(field, "name");
                    assert_eq!(min, 3);
                }
                _ => panic!("Expected TooShort error"),
            }
        }

        #[test]
        fn test_name_exactly_min_length() {
            let mut test = valid_test();
            test.name = "ABC".to_string();
            assert!(validate_verification(&test).is_ok());
        }

        #[test]
        fn test_name_too_long() {
            let mut test = valid_test();
            test.name = "A".repeat(256);
            let result = validate_verification(&test);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TooLong { field, max } => {
                    assert_eq!(field, "name");
                    assert_eq!(max, 255);
                }
                _ => panic!("Expected TooLong error"),
            }
        }

        #[test]
        fn test_reference_code_required() {
            let mut test = valid_test();
            test.reference_code = "".to_string();
            let result = validate_verification(&test);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::Required { field } => assert_eq!(field, "reference_code"),
                _ => panic!("Expected Required error"),
            }
        }

        #[test]
        fn test_reference_code_whitespace_only() {
            let mut test = valid_test();
            test.reference_code = "   ".to_string();
            let result = validate_verification(&test);
            assert!(result.is_err());
        }

        #[test]
        fn test_reference_code_too_long() {
            let mut test = valid_test();
            test.reference_code = "TEST-".to_string() + &"1".repeat(50);
            let result = validate_verification(&test);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TooLong { field, max } => {
                    assert_eq!(field, "reference_code");
                    assert_eq!(max, 50);
                }
                _ => panic!("Expected TooLong error"),
            }
        }

        #[test]
        fn test_reference_code_valid_format() {
            let mut test = valid_test();
            test.reference_code = "TEST-1".to_string();
            assert!(validate_verification(&test).is_ok());
        }

        #[test]
        fn test_reference_code_valid_large_number() {
            let mut test = valid_test();
            test.reference_code = "TEST-12345".to_string();
            assert!(validate_verification(&test).is_ok());
        }

        #[test]
        fn test_reference_code_invalid_no_dash() {
            let mut test = valid_test();
            test.reference_code = "TEST1".to_string();
            let result = validate_verification(&test);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::Custom(msg) => {
                    assert!(msg.contains("TEST-NUMBER"));
                }
                _ => panic!("Expected Custom error"),
            }
        }

        #[test]
        fn test_reference_code_invalid_lowercase() {
            let mut test = valid_test();
            test.reference_code = "test-1".to_string();
            let result = validate_verification(&test);
            assert!(result.is_err());
        }

        #[test]
        fn test_reference_code_invalid_letters_after_dash() {
            let mut test = valid_test();
            test.reference_code = "TEST-ABC".to_string();
            let result = validate_verification(&test);
            assert!(result.is_err());
        }

        #[test]
        fn test_description_required() {
            let mut test = valid_test();
            test.description = "".to_string();
            let result = validate_verification(&test);
            assert!(result.is_err());
        }

        #[test]
        fn test_description_too_long() {
            let mut test = valid_test();
            test.description = "A".repeat(2001);
            let result = validate_verification(&test);
            assert!(result.is_err());
        }

        #[test]
        fn test_status_id_zero() {
            let mut test = valid_test();
            test.status_id = 0;
            let result = validate_verification(&test);
            assert!(result.is_err());
        }

        #[test]
        fn test_parent_id_zero() {
            let mut test = valid_test();
            test.parent_id = Some(0);
            let result = validate_verification(&test);
            assert!(result.is_err());
        }

        #[test]
        fn test_parent_id_negative() {
            let mut test = valid_test();
            test.parent_id = Some(-1);
            let result = validate_verification(&test);
            assert!(result.is_err());
        }

        #[test]
        fn test_parent_id_none_allowed() {
            let mut test = valid_test();
            test.parent_id = None;
            assert!(validate_verification(&test).is_ok());
        }

        #[test]
        fn test_project_id_zero() {
            let mut test = valid_test();
            test.project_id = 0;
            let result = validate_verification(&test);
            assert!(result.is_err());
        }
    }

    // ============================================================================
    // Tests for validate_category
    // ============================================================================

    mod validate_category_tests {
        use super::*;

        fn valid_category() -> NewCategory {
            NewCategory {
                id: None,
                title: "Valid Category".to_string(),
                description: "Valid description".to_string(),
                tag: "VALID_TAG".to_string(),
                project_id: 1,
            }
        }

        #[test]
        fn valid_category_passes() {
            let cat = valid_category();
            assert!(validate_category(&cat).is_ok());
        }

        #[test]
        fn category_title_required() {
            let mut cat = valid_category();
            cat.title = "".to_string();
            let result = validate_category(&cat);
            assert!(result.is_err());
        }

        #[test]
        fn category_title_too_short() {
            let mut cat = valid_category();
            cat.title = "A".to_string(); // Less than 2
            let result = validate_category(&cat);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TooShort { field, min } => {
                    assert_eq!(field, "title");
                    assert_eq!(min, 2);
                }
                _ => panic!("Expected TooShort error"),
            }
        }

        #[test]
        fn category_title_exactly_min_length() {
            let mut cat = valid_category();
            cat.title = "AB".to_string();
            assert!(validate_category(&cat).is_ok());
        }

        #[test]
        fn category_title_too_long() {
            let mut cat = valid_category();
            cat.title = "A".repeat(101);
            let result = validate_category(&cat);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TooLong { field, max } => {
                    assert_eq!(field, "title");
                    assert_eq!(max, 100);
                }
                _ => panic!("Expected TooLong error"),
            }
        }

        #[test]
        fn category_description_required() {
            let mut cat = valid_category();
            cat.description = "".to_string();
            let result = validate_category(&cat);
            assert!(result.is_err());
        }

        #[test]
        fn category_description_too_long() {
            let mut cat = valid_category();
            cat.description = "A".repeat(501);
            let result = validate_category(&cat);
            assert!(result.is_err());
        }

        #[test]
        fn category_tag_required() {
            let mut cat = valid_category();
            cat.tag = "".to_string();
            let result = validate_category(&cat);
            assert!(result.is_err());
        }

        #[test]
        fn category_tag_too_long() {
            let mut cat = valid_category();
            cat.tag = "A".repeat(51);
            let result = validate_category(&cat);
            assert!(result.is_err());
        }

        #[test]
        fn category_tag_valid_alphanumeric() {
            let mut cat = valid_category();
            cat.tag = "TAG123".to_string();
            assert!(validate_category(&cat).is_ok());
        }

        #[test]
        fn category_tag_valid_with_underscore() {
            let mut cat = valid_category();
            cat.tag = "TAG_NAME_123".to_string();
            assert!(validate_category(&cat).is_ok());
        }

        #[test]
        fn category_tag_invalid_with_dash() {
            let mut cat = valid_category();
            cat.tag = "TAG-NAME".to_string();
            let result = validate_category(&cat);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::InvalidFormat { field, .. } => {
                    assert_eq!(field, "tag");
                }
                _ => panic!("Expected InvalidFormat error"),
            }
        }

        #[test]
        fn category_tag_invalid_with_space() {
            let mut cat = valid_category();
            cat.tag = "TAG NAME".to_string();
            let result = validate_category(&cat);
            assert!(result.is_err());
        }

        #[test]
        fn category_tag_invalid_with_special_chars() {
            let mut cat = valid_category();
            cat.tag = "TAG@NAME".to_string();
            let result = validate_category(&cat);
            assert!(result.is_err());
        }

        #[test]
        fn category_project_id_zero() {
            let mut cat = valid_category();
            cat.project_id = 0;
            let result = validate_category(&cat);
            assert!(result.is_err());
        }
    }

    // ============================================================================
    // Tests for validate_applicability
    // ============================================================================

    mod validate_applicability_tests {
        use super::*;

        fn valid_applicability() -> NewApplicability {
            NewApplicability {
                id: None,
                title: "Valid Applicability".to_string(),
                description: "Valid description".to_string(),
                tag: "VALID_TAG".to_string(),
                project_id: 1,
            }
        }

        #[test]
        fn valid_applicability_passes() {
            let app = valid_applicability();
            assert!(validate_applicability(&app).is_ok());
        }

        #[test]
        fn applicability_title_required() {
            let mut app = valid_applicability();
            app.title = "".to_string();
            let result = validate_applicability(&app);
            assert!(result.is_err());
        }

        #[test]
        fn applicability_title_too_short() {
            let mut app = valid_applicability();
            app.title = "A".to_string();
            let result = validate_applicability(&app);
            assert!(result.is_err());
        }

        #[test]
        fn applicability_title_too_long() {
            let mut app = valid_applicability();
            app.title = "A".repeat(101);
            let result = validate_applicability(&app);
            assert!(result.is_err());
        }

        #[test]
        fn applicability_description_required() {
            let mut app = valid_applicability();
            app.description = "".to_string();
            let result = validate_applicability(&app);
            assert!(result.is_err());
        }

        #[test]
        fn applicability_description_too_long() {
            let mut app = valid_applicability();
            app.description = "A".repeat(501);
            let result = validate_applicability(&app);
            assert!(result.is_err());
        }

        #[test]
        fn applicability_tag_required() {
            let mut app = valid_applicability();
            app.tag = "".to_string();
            let result = validate_applicability(&app);
            assert!(result.is_err());
        }

        #[test]
        fn applicability_tag_valid_format() {
            let mut app = valid_applicability();
            app.tag = "TAG_123".to_string();
            assert!(validate_applicability(&app).is_ok());
        }

        #[test]
        fn applicability_tag_invalid_format() {
            let mut app = valid_applicability();
            app.tag = "TAG-NAME".to_string();
            let result = validate_applicability(&app);
            assert!(result.is_err());
        }

        #[test]
        fn applicability_project_id_zero() {
            let mut app = valid_applicability();
            app.project_id = 0;
            let result = validate_applicability(&app);
            assert!(result.is_err());
        }
    }

    // ============================================================================
    // Tests for validate_user
    // ============================================================================

    mod validate_user_tests {
        use super::*;

        fn valid_user() -> NewUser {
            NewUser {
                id: None,
                username: "validuser".to_string(),
                name: "Valid User Name".to_string(),
                email: "user@example.com".to_string(),
                password_hash: "hash".to_string(),
                is_admin: false,
            }
        }

        #[test]
        fn valid_user_passes() {
            let user = valid_user();
            assert!(validate_user(&user).is_ok());
        }

        #[test]
        fn user_username_required() {
            let mut user = valid_user();
            user.username = "".to_string();
            let result = validate_user(&user);
            assert!(result.is_err());
        }

        #[test]
        fn user_username_too_short() {
            let mut user = valid_user();
            user.username = "ab".to_string(); // Less than 3
            let result = validate_user(&user);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::TooShort { field, min } => {
                    assert_eq!(field, "username");
                    assert_eq!(min, 3);
                }
                _ => panic!("Expected TooShort error"),
            }
        }

        #[test]
        fn user_username_exactly_min_length() {
            let mut user = valid_user();
            user.username = "abc".to_string();
            assert!(validate_user(&user).is_ok());
        }

        #[test]
        fn user_username_too_long() {
            let mut user = valid_user();
            user.username = "A".repeat(51);
            let result = validate_user(&user);
            assert!(result.is_err());
        }

        #[test]
        fn user_username_valid_alphanumeric() {
            let mut user = valid_user();
            user.username = "user123".to_string();
            assert!(validate_user(&user).is_ok());
        }

        #[test]
        fn user_username_valid_with_underscore() {
            let mut user = valid_user();
            user.username = "user_name".to_string();
            assert!(validate_user(&user).is_ok());
        }

        #[test]
        fn user_username_invalid_with_dash() {
            let mut user = valid_user();
            user.username = "user-name".to_string();
            let result = validate_user(&user);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::InvalidFormat { field, .. } => {
                    assert_eq!(field, "username");
                }
                _ => panic!("Expected InvalidFormat error"),
            }
        }

        #[test]
        fn user_username_invalid_with_space() {
            let mut user = valid_user();
            user.username = "user name".to_string();
            let result = validate_user(&user);
            assert!(result.is_err());
        }

        #[test]
        fn user_username_invalid_with_special_chars() {
            let mut user = valid_user();
            user.username = "user@name".to_string();
            let result = validate_user(&user);
            assert!(result.is_err());
        }

        #[test]
        fn user_name_required() {
            let mut user = valid_user();
            user.name = "".to_string();
            let result = validate_user(&user);
            assert!(result.is_err());
        }

        #[test]
        fn user_name_too_short() {
            let mut user = valid_user();
            user.name = "A".to_string(); // Less than 2
            let result = validate_user(&user);
            assert!(result.is_err());
        }

        #[test]
        fn user_name_too_long() {
            let mut user = valid_user();
            user.name = "A".repeat(101);
            let result = validate_user(&user);
            assert!(result.is_err());
        }

        #[test]
        fn user_email_valid_format() {
            let mut user = valid_user();
            user.email = "test@example.com".to_string();
            assert!(validate_user(&user).is_ok());
        }

        #[test]
        fn user_email_valid_with_subdomain() {
            let mut user = valid_user();
            user.email = "test@mail.example.com".to_string();
            assert!(validate_user(&user).is_ok());
        }

        #[test]
        fn user_email_valid_with_plus() {
            let mut user = valid_user();
            user.email = "test+tag@example.com".to_string();
            assert!(validate_user(&user).is_ok());
        }

        #[test]
        fn user_email_empty_allowed() {
            let mut user = valid_user();
            user.email = "".to_string();
            assert!(validate_user(&user).is_ok());
        }

        #[test]
        fn user_email_whitespace_only_allowed() {
            let mut user = valid_user();
            user.email = "   ".to_string();
            assert!(validate_user(&user).is_ok()); // Empty after trim
        }

        #[test]
        fn user_email_invalid_no_at() {
            let mut user = valid_user();
            user.email = "invalidemail.com".to_string();
            let result = validate_user(&user);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::InvalidFormat { field, .. } => {
                    assert_eq!(field, "email");
                }
                _ => panic!("Expected InvalidFormat error"),
            }
        }

        #[test]
        fn user_email_invalid_no_domain() {
            let mut user = valid_user();
            user.email = "test@".to_string();
            let result = validate_user(&user);
            assert!(result.is_err());
        }

        #[test]
        fn user_email_invalid_no_tld() {
            let mut user = valid_user();
            user.email = "test@example".to_string();
            let result = validate_user(&user);
            assert!(result.is_err());
        }
    }

    // ============================================================================
    // Tests for validate_project
    // ============================================================================

    mod validate_project_tests {
        use super::*;

        fn valid_project() -> NewProject {
            NewProject {
                name: "Valid Project".to_string(),
                description: Some("Valid description".to_string()),
                owner_id: Some(1),
                status: ProjectStatus::Active,
                group_id: None,
            }
        }

        #[test]
        fn valid_project_passes() {
            let project = valid_project();
            assert!(validate_project(&project).is_ok());
        }

        #[test]
        fn project_name_required() {
            let mut project = valid_project();
            project.name = "".to_string();
            let result = validate_project(&project);
            assert!(result.is_err());
        }

        #[test]
        fn project_name_too_short() {
            let mut project = valid_project();
            project.name = "A".to_string();
            let result = validate_project(&project);
            assert!(result.is_err());
        }

        #[test]
        fn project_name_too_long() {
            let mut project = valid_project();
            project.name = "A".repeat(101);
            let result = validate_project(&project);
            assert!(result.is_err());
        }

        #[test]
        fn project_description_none_allowed() {
            let mut project = valid_project();
            project.description = None;
            assert!(validate_project(&project).is_ok());
        }

        #[test]
        fn project_description_empty_allowed() {
            let mut project = valid_project();
            project.description = Some("".to_string());
            assert!(validate_project(&project).is_ok());
        }

        #[test]
        fn project_description_whitespace_only_allowed() {
            let mut project = valid_project();
            project.description = Some("   ".to_string());
            assert!(validate_project(&project).is_ok()); // Empty after trim
        }

        #[test]
        fn project_description_too_long() {
            let mut project = valid_project();
            project.description = Some("A".repeat(1001));
            let result = validate_project(&project);
            assert!(result.is_err());
        }

        #[test]
        fn project_description_exactly_max_length() {
            let mut project = valid_project();
            project.description = Some("A".repeat(1000));
            assert!(validate_project(&project).is_ok());
        }

        #[test]
        fn project_owner_id_required() {
            let mut project = valid_project();
            project.owner_id = None;
            let result = validate_project(&project);
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::Required { field } => {
                    assert_eq!(field, "owner_id");
                }
                _ => panic!("Expected Required error"),
            }
        }
    }

    // ============================================================================
    // Tests for validate_requirement_status
    // ============================================================================

    mod validate_requirement_status_tests {
        use super::*;

        fn valid_requirement_status() -> NewRequirementStatus {
            NewRequirementStatus {
                id: None,
                title: "Valid Status".to_string(),
                description: "Valid description".to_string(),
                tag: "VALID".to_string(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            }
        }

        #[test]
        fn valid_requirement_status_passes() {
            let status = valid_requirement_status();
            assert!(validate_requirement_status(&status).is_ok());
        }

        #[test]
        fn requirement_status_title_required() {
            let mut status = valid_requirement_status();
            status.title = "".to_string();
            let result = validate_requirement_status(&status);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_status_title_too_short() {
            let mut status = valid_requirement_status();
            status.title = "A".to_string();
            let result = validate_requirement_status(&status);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_status_title_too_long() {
            let mut status = valid_requirement_status();
            status.title = "A".repeat(51);
            let result = validate_requirement_status(&status);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_status_description_empty_allowed() {
            let mut status = valid_requirement_status();
            status.description = "".to_string();
            assert!(validate_requirement_status(&status).is_ok());
        }

        #[test]
        fn requirement_status_description_too_long() {
            let mut status = valid_requirement_status();
            status.description = "A".repeat(201);
            let result = validate_requirement_status(&status);
            assert!(result.is_err());
        }

        #[test]
        fn requirement_status_description_exactly_max_length() {
            let mut status = valid_requirement_status();
            status.description = "A".repeat(200);
            assert!(validate_requirement_status(&status).is_ok());
        }
    }

    // ============================================================================
    // Tests for validate_verification_status
    // ============================================================================

    mod validate_verification_status_tests {
        use super::*;

        fn valid_test_status() -> VerificationStatus {
            VerificationStatus {
                id: 1,
                title: "Valid Status".to_string(),
                description: "Valid description".to_string(),
                tag: "VALID".to_string(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            }
        }

        #[test]
        fn valid_test_status_passes() {
            let status = valid_test_status();
            assert!(validate_verification_status(&status).is_ok());
        }

        #[test]
        fn test_status_title_required() {
            let mut status = valid_test_status();
            status.title = "".to_string();
            let result = validate_verification_status(&status);
            assert!(result.is_err());
        }

        #[test]
        fn test_status_title_too_short() {
            let mut status = valid_test_status();
            status.title = "A".to_string();
            let result = validate_verification_status(&status);
            assert!(result.is_err());
        }

        #[test]
        fn test_status_title_too_long() {
            let mut status = valid_test_status();
            status.title = "A".repeat(51);
            let result = validate_verification_status(&status);
            assert!(result.is_err());
        }

        #[test]
        fn test_status_description_empty_allowed() {
            let mut status = valid_test_status();
            status.description = "".to_string();
            assert!(validate_verification_status(&status).is_ok());
        }

        #[test]
        fn test_status_description_too_long() {
            let mut status = valid_test_status();
            status.description = "A".repeat(201);
            let result = validate_verification_status(&status);
            assert!(result.is_err());
        }
    }

    // ============================================================================
    // Tests for validate_id
    // ============================================================================

    mod validate_id_tests {
        use super::*;

        #[test]
        fn validate_id_positive() {
            assert!(validate_id(1, "Test").is_ok());
        }

        #[test]
        fn validate_id_large_positive() {
            assert!(validate_id(i32::MAX, "Test").is_ok());
        }

        #[test]
        fn validate_id_zero() {
            let result = validate_id(0, "Test");
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::Custom(msg) => {
                    assert!(msg.contains("Test ID"));
                    assert!(msg.contains("positive"));
                }
                _ => panic!("Expected Custom error"),
            }
        }

        #[test]
        fn validate_id_negative() {
            let result = validate_id(-1, "Requirement");
            assert!(result.is_err());
            match result.unwrap_err() {
                ValidationError::Custom(msg) => {
                    assert!(msg.contains("Requirement ID"));
                }
                _ => panic!("Expected Custom error"),
            }
        }

        #[test]
        fn validate_id_different_entity_names() {
            let entities = vec!["Requirement", "Test", "Category", "User", "Project"];
            for entity in entities {
                let result = validate_id(0, entity);
                assert!(result.is_err());
                match result.unwrap_err() {
                    ValidationError::Custom(msg) => {
                        assert!(msg.contains(entity));
                    }
                    _ => panic!("Expected Custom error"),
                }
            }
        }
    }

    // ============================================================================
    // Tests for sanitize_string
    // ============================================================================

    mod sanitize_string_tests {
        use super::*;

        #[test]
        fn sanitize_string_no_whitespace() {
            let mut input = "TestString".to_string();
            sanitize_string(&mut input);
            assert_eq!(input, "TestString");
        }

        #[test]
        fn sanitize_string_leading_whitespace() {
            let mut input = "  TestString".to_string();
            sanitize_string(&mut input);
            assert_eq!(input, "TestString");
        }

        #[test]
        fn sanitize_string_trailing_whitespace() {
            let mut input = "TestString  ".to_string();
            sanitize_string(&mut input);
            assert_eq!(input, "TestString");
        }

        #[test]
        fn sanitize_string_both_ends() {
            let mut input = "  TestString  ".to_string();
            sanitize_string(&mut input);
            assert_eq!(input, "TestString");
        }

        #[test]
        fn sanitize_string_only_whitespace() {
            let mut input = "   ".to_string();
            sanitize_string(&mut input);
            assert_eq!(input, "");
        }

        #[test]
        fn sanitize_string_empty() {
            let mut input = "".to_string();
            sanitize_string(&mut input);
            assert_eq!(input, "");
        }

        #[test]
        fn sanitize_string_with_tabs() {
            let mut input = "\tTestString\t".to_string();
            sanitize_string(&mut input);
            assert_eq!(input, "TestString");
        }

        #[test]
        fn sanitize_string_with_newlines() {
            let mut input = "\nTestString\n".to_string();
            sanitize_string(&mut input);
            assert_eq!(input, "TestString");
        }

        #[test]
        fn sanitize_string_mixed_whitespace() {
            let mut input = " \t\n TestString \t\n ".to_string();
            sanitize_string(&mut input);
            assert_eq!(input, "TestString");
        }
    }

    // ============================================================================
    // Tests for sanitize_optional_string
    // ============================================================================

    mod sanitize_optional_string_tests {
        use super::*;

        #[test]
        fn sanitize_optional_string_some_no_whitespace() {
            let mut input = Some("TestString".to_string());
            sanitize_optional_string(&mut input);
            assert_eq!(input, Some("TestString".to_string()));
        }

        #[test]
        fn sanitize_optional_string_some_with_whitespace() {
            let mut input = Some("  TestString  ".to_string());
            sanitize_optional_string(&mut input);
            assert_eq!(input, Some("TestString".to_string()));
        }

        #[test]
        fn sanitize_optional_string_some_only_whitespace() {
            let mut input = Some("   ".to_string());
            sanitize_optional_string(&mut input);
            assert_eq!(input, None);
        }

        #[test]
        fn sanitize_optional_string_some_empty() {
            let mut input = Some("".to_string());
            sanitize_optional_string(&mut input);
            assert_eq!(input, None);
        }

        #[test]
        fn sanitize_optional_string_none() {
            let mut input: Option<String> = None;
            sanitize_optional_string(&mut input);
            assert_eq!(input, None);
        }

        #[test]
        fn sanitize_optional_string_whitespace_becomes_none() {
            let mut input = Some(" \t\n ".to_string());
            sanitize_optional_string(&mut input);
            assert_eq!(input, None);
        }
    }

    // ============================================================================
    // Tests for regex patterns
    // ============================================================================

    mod regex_pattern_tests {
        use regex::Regex;

        #[test]
        fn requirement_reference_regex_valid_patterns() {
            let regex = Regex::new(r"^[A-Z]{2,4}(?:-[A-Z0-9]{1,6})+$").unwrap();
            assert!(regex.is_match("REQ-001"));
            assert!(regex.is_match("REQ-ABC-001"));
            assert!(regex.is_match("AB-001"));
            assert!(regex.is_match("ABCD-001"));
            assert!(regex.is_match("REQ-123456"));
        }

        #[test]
        fn requirement_reference_regex_invalid_patterns() {
            let regex = Regex::new(r"^[A-Z]{2,4}(?:-[A-Z0-9]{1,6})+$").unwrap();
            assert!(!regex.is_match("req-001")); // lowercase
            assert!(!regex.is_match("REQ001")); // no dash
            assert!(!regex.is_match("REQ-001!")); // special char
            assert!(!regex.is_match("R-001")); // too short prefix
            assert!(!regex.is_match("ABCDE-001")); // too long prefix
        }

        #[test]
        fn test_reference_regex_valid_patterns() {
            let regex = Regex::new(r"^TEST-\d+$").unwrap();
            assert!(regex.is_match("TEST-1"));
            assert!(regex.is_match("TEST-123"));
            assert!(regex.is_match("TEST-12345"));
        }

        #[test]
        fn test_reference_regex_invalid_patterns() {
            let regex = Regex::new(r"^TEST-\d+$").unwrap();
            assert!(!regex.is_match("test-1")); // lowercase
            assert!(!regex.is_match("TEST1")); // no dash
            assert!(!regex.is_match("TEST-ABC")); // letters
            assert!(!regex.is_match("TEST-1-2")); // multiple dashes
        }

        #[test]
        fn tag_regex_valid_patterns() {
            let regex = Regex::new(r"^[A-Za-z0-9_]+$").unwrap();
            assert!(regex.is_match("TAG"));
            assert!(regex.is_match("TAG123"));
            assert!(regex.is_match("tag_name"));
            assert!(regex.is_match("TAG_NAME_123"));
            assert!(regex.is_match("_tag"));
        }

        #[test]
        fn tag_regex_invalid_patterns() {
            let regex = Regex::new(r"^[A-Za-z0-9_]+$").unwrap();
            assert!(!regex.is_match("TAG-NAME")); // dash
            assert!(!regex.is_match("TAG NAME")); // space
            assert!(!regex.is_match("TAG@NAME")); // special char
        }

        #[test]
        fn username_regex_valid_patterns() {
            let regex = Regex::new(r"^[A-Za-z0-9_]+$").unwrap();
            assert!(regex.is_match("username"));
            assert!(regex.is_match("user123"));
            assert!(regex.is_match("user_name"));
            assert!(regex.is_match("User123"));
        }

        #[test]
        fn username_regex_invalid_patterns() {
            let regex = Regex::new(r"^[A-Za-z0-9_]+$").unwrap();
            assert!(!regex.is_match("user-name")); // dash
            assert!(!regex.is_match("user name")); // space
            assert!(!regex.is_match("user@name")); // special char
        }

        #[test]
        fn email_regex_valid_patterns() {
            let regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
            assert!(regex.is_match("test@example.com"));
            assert!(regex.is_match("test@mail.example.com"));
            assert!(regex.is_match("test+tag@example.com"));
            assert!(regex.is_match("test.name@example.co.uk"));
        }

        #[test]
        fn email_regex_invalid_patterns() {
            let regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
            assert!(!regex.is_match("invalidemail.com")); // no @
            assert!(!regex.is_match("test@")); // no domain
            assert!(!regex.is_match("test@example")); // no TLD
            assert!(!regex.is_match("@example.com")); // no local part
        }
    }

    // ============================================================================
    // Edge case tests
    // ============================================================================

    mod edge_case_tests {
        use super::*;

        #[test]
        fn requirement_with_all_fields_valid() {
            let req = NewRequirement {
                id: Some(1),
                title: "Complete Requirement".to_string(),
                description: "Full description".to_string(),
                author_id: 1,
                category_id: 1,
                status_id: 1,
                reference_code: "REQ-ABC-001".to_string(),
                reviewer_id: 1,
                applicability_id: 1,
                justification: Some("Important".to_string()),
                project_id: 1,
            };
            assert!(validate_requirement(&req).is_ok());
        }

        #[test]
        fn test_with_all_fields_valid() {
            let test = NewVerification {
                id: Some(1),
                reference_code: "TEST-123".to_string(),
                name: "Complete Test".to_string(),
                description: "Full description".to_string(),
                source: "test.rs".to_string(),
                status_id: 1,
                parent_id: Some(10),
                project_id: 1,
                verification_method_id: None,
            };
            assert!(validate_verification(&test).is_ok());
        }

        #[test]
        fn boundary_values_title_min() {
            let req = NewRequirement {
                id: None,
                title: "ABC".to_string(), // Exactly 3
                description: "Desc".to_string(),
                author_id: 1,
                category_id: 1,
                status_id: 1,
                reference_code: "REQ-001".to_string(),
                reviewer_id: 1,
                applicability_id: 1,
                justification: None,
                project_id: 1,
            };
            assert!(validate_requirement(&req).is_ok());
        }

        #[test]
        fn boundary_values_title_max() {
            let req = NewRequirement {
                id: None,
                title: "A".repeat(255),
                description: "Desc".to_string(),
                author_id: 1,
                category_id: 1,
                status_id: 1,
                reference_code: "REQ-001".to_string(),
                reviewer_id: 1,
                applicability_id: 1,
                justification: None,
                project_id: 1,
            };
            assert!(validate_requirement(&req).is_ok());
        }

        #[test]
        fn boundary_values_description_max() {
            let req = NewRequirement {
                id: None,
                title: "Title".to_string(),
                description: "A".repeat(2000),
                author_id: 1,
                category_id: 1,
                status_id: 1,
                reference_code: "REQ-001".to_string(),
                reviewer_id: 1,
                applicability_id: 1,
                justification: None,
                project_id: 1,
            };
            assert!(validate_requirement(&req).is_ok());
        }

        #[test]
        fn multiple_validation_errors_first_one_returned() {
            let req = NewRequirement {
                id: None,
                title: "".to_string(),       // Error 1
                description: "".to_string(), // Error 2
                author_id: 1,
                category_id: 1,
                status_id: 1,
                reference_code: "REQ-001".to_string(),
                reviewer_id: 1,
                applicability_id: 1,
                justification: None,
                project_id: 1,
            };
            let result = validate_requirement(&req);
            assert!(result.is_err());
            // Should return first error (title required)
            if let ValidationError::Required { field } = result.unwrap_err() {
                assert_eq!(field, "title");
            }
        }

        #[test]
        fn unicode_characters_in_strings() {
            let req = NewRequirement {
                id: None,
                title: "Título con ñ".to_string(),
                description: "Descripción".to_string(),
                author_id: 1,
                category_id: 1,
                status_id: 1,
                reference_code: "REQ-001".to_string(),
                reviewer_id: 1,
                applicability_id: 1,
                justification: None,
                project_id: 1,
            };
            // Unicode should be handled (length is in bytes/chars)
            assert!(validate_requirement(&req).is_ok());
        }
    }
}

//! Input validation utilities for the ReqMan application.
//!
//! This module provides validation functions for all input data types,
//! ensuring data integrity and providing clear error messages.

use crate::errors::ValidationError;
use crate::models::*;
use regex::Regex;

/// Validate a requirement before creation or update
pub fn validate_requirement(req: &NewRequirement) -> Result<(), ValidationError> {
    // Validate title
    if req.req_title.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "req_title".to_string(),
        });
    }

    if req.req_title.len() > 255 {
        return Err(ValidationError::TooLong {
            field: "req_title".to_string(),
            max: 255,
        });
    }

    if req.req_title.len() < 3 {
        return Err(ValidationError::TooShort {
            field: "req_title".to_string(),
            min: 3,
        });
    }

    // Validate description
    if req.req_description.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "req_description".to_string(),
        });
    }

    if req.req_description.len() > 2000 {
        return Err(ValidationError::TooLong {
            field: "req_description".to_string(),
            max: 2000,
        });
    }

    // Validate reference format (should be like REQ-001, REQ-ABC-001, etc.)
    if !req.req_reference.trim().is_empty() {
        let ref_regex = Regex::new(r"^[A-Z]{2,4}(?:-[A-Z0-9]{1,6})+$").unwrap();
            if !ref_regex.is_match(&req.req_reference) {
            return Err(ValidationError::InvalidFormat {
                field: "req_reference".to_string(),
                message: "Reference should be in format like REQ-001 or REQ-ABC-001".to_string(),
            });
        }
    }

    // Validate link format if provided
    if !req.req_link.trim().is_empty() {
        let url_regex = Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap();
        if !url_regex.is_match(&req.req_link) {
            return Err(ValidationError::InvalidFormat {
                field: "req_link".to_string(),
                message: "Link must be a valid HTTP/HTTPS URL".to_string(),
            });
        }
    }

    // Validate IDs are positive
    if req.req_verification <= 0 {
        return Err(ValidationError::Custom(
            "Verification method ID must be positive".to_string(),
        ));
    }

    if req.req_current_status <= 0 {
        return Err(ValidationError::Custom(
            "Status ID must be positive".to_string(),
        ));
    }

    if req.req_author <= 0 {
        return Err(ValidationError::Custom(
            "Author ID must be positive".to_string(),
        ));
    }

    if req.req_reviewer <= 0 {
        return Err(ValidationError::Custom(
            "Reviewer ID must be positive".to_string(),
        ));
    }

    if req.req_category <= 0 {
        return Err(ValidationError::Custom(
            "Category ID must be positive".to_string(),
        ));
    }

    if req.project_id <= 0 {
        return Err(ValidationError::Custom(
            "Project ID must be positive".to_string(),
        ));
    }

    Ok(())
}

/// Validate a test before creation or update
pub fn validate_test(test: &NewTest) -> Result<(), ValidationError> {
    // Validate test name
    if test.test_name.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "test_name".to_string(),
        });
    }

    if test.test_name.len() > 255 {
        return Err(ValidationError::TooLong {
            field: "test_name".to_string(),
            max: 255,
        });
    }

    if test.test_name.len() < 3 {
        return Err(ValidationError::TooShort {
            field: "test_name".to_string(),
            min: 3,
        });
    }

    // Validate test reference
    if test.test_reference.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "test_reference".to_string(),
        });
    }

    if test.test_reference.len() > 50 {
        return Err(ValidationError::TooLong {
            field: "test_reference".to_string(),
            max: 50,
        });
    }

    // Validate test reference format (TEST-NUMBER)
    let test_ref_regex = Regex::new(r"^TEST-\d+$").unwrap();
    if !test_ref_regex.is_match(&test.test_reference) {
        return Err(ValidationError::Custom(
            "Test reference must follow format TEST-NUMBER (e.g., TEST-1, TEST-2)".to_string(),
        ));
    }

    // Validate description
    if test.test_description.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "test_description".to_string(),
        });
    }

    if test.test_description.len() > 2000 {
        return Err(ValidationError::TooLong {
            field: "test_description".to_string(),
            max: 2000,
        });
    }

    // Validate IDs are positive
    if test.test_status <= 0 {
        return Err(ValidationError::Custom(
            "Test status ID must be positive".to_string(),
        ));
    }

    if test.test_parent <= 0 {
        return Err(ValidationError::Custom(
            "Test parent ID must be positive".to_string(),
        ));
    }

    if test.project_id <= 0 {
        return Err(ValidationError::Custom(
            "Project ID must be positive".to_string(),
        ));
    }

    Ok(())
}

/// Validate a category before creation or update
pub fn validate_category(category: &NewCategory) -> Result<(), ValidationError> {
    // Validate title
    if category.cat_title.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "cat_title".to_string(),
        });
    }

    if category.cat_title.len() > 100 {
        return Err(ValidationError::TooLong {
            field: "cat_title".to_string(),
            max: 100,
        });
    }

    if category.cat_title.len() < 2 {
        return Err(ValidationError::TooShort {
            field: "cat_title".to_string(),
            min: 2,
        });
    }

    // Validate description
    if category.cat_description.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "cat_description".to_string(),
        });
    }

    if category.cat_description.len() > 500 {
        return Err(ValidationError::TooLong {
            field: "cat_description".to_string(),
            max: 500,
        });
    }

    // Validate tag
    if category.cat_tag.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "cat_tag".to_string(),
        });
    }

    if category.cat_tag.len() > 50 {
        return Err(ValidationError::TooLong {
            field: "cat_tag".to_string(),
            max: 50,
        });
    }

    // Validate tag format (should be alphanumeric with underscores)
    let tag_regex = Regex::new(r"^[A-Za-z0-9_]+$").unwrap();
    if !tag_regex.is_match(&category.cat_tag) {
        return Err(ValidationError::InvalidFormat {
            field: "cat_tag".to_string(),
            message: "Tag should contain only letters, numbers, and underscores".to_string(),
        });
    }

    if category.project_id <= 0 {
        return Err(ValidationError::Custom(
            "Project ID must be positive".to_string(),
        ));
    }

    Ok(())
}

/// Validate an applicability before creation or update
pub fn validate_applicability(applicability: &NewApplicability) -> Result<(), ValidationError> {
    // Validate title
    if applicability.app_title.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "app_title".to_string(),
        });
    }

    if applicability.app_title.len() > 100 {
        return Err(ValidationError::TooLong {
            field: "app_title".to_string(),
            max: 100,
        });
    }

    if applicability.app_title.len() < 2 {
        return Err(ValidationError::TooShort {
            field: "app_title".to_string(),
            min: 2,
        });
    }

    // Validate description
    if applicability.app_description.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "app_description".to_string(),
        });
    }

    if applicability.app_description.len() > 500 {
        return Err(ValidationError::TooLong {
            field: "app_description".to_string(),
            max: 500,
        });
    }

    // Validate tag
    if applicability.app_tag.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "app_tag".to_string(),
        });
    }

    if applicability.app_tag.len() > 50 {
        return Err(ValidationError::TooLong {
            field: "app_tag".to_string(),
            max: 50,
        });
    }

    // Validate tag format
    let tag_regex = Regex::new(r"^[A-Za-z0-9_]+$").unwrap();
    if !tag_regex.is_match(&applicability.app_tag) {
        return Err(ValidationError::InvalidFormat {
            field: "app_tag".to_string(),
            message: "Tag should contain only letters, numbers, and underscores".to_string(),
        });
    }

    if applicability.project_id <= 0 {
        return Err(ValidationError::Custom(
            "Project ID must be positive".to_string(),
        ));
    }

    Ok(())
}

/// Validate a user before creation or update
pub fn validate_user(user: &NewUser) -> Result<(), ValidationError> {
    // Validate username
    if user.user_username.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "user_username".to_string(),
        });
    }

    if user.user_username.len() > 50 {
        return Err(ValidationError::TooLong {
            field: "user_username".to_string(),
            max: 50,
        });
    }

    if user.user_username.len() < 3 {
        return Err(ValidationError::TooShort {
            field: "user_username".to_string(),
            min: 3,
        });
    }

    // Validate username format (alphanumeric and underscores only)
    let username_regex = Regex::new(r"^[A-Za-z0-9_]+$").unwrap();
    if !username_regex.is_match(&user.user_username) {
        return Err(ValidationError::InvalidFormat {
            field: "user_username".to_string(),
            message: "Username should contain only letters, numbers, and underscores".to_string(),
        });
    }

    // Validate name
    if user.user_name.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "user_name".to_string(),
        });
    }

    if user.user_name.len() > 100 {
        return Err(ValidationError::TooLong {
            field: "user_name".to_string(),
            max: 100,
        });
    }

    if user.user_name.len() < 2 {
        return Err(ValidationError::TooShort {
            field: "user_name".to_string(),
            min: 2,
        });
    }

    // Validate email format if provided
    if !user.user_email.trim().is_empty() {
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        if !email_regex.is_match(&user.user_email) {
            return Err(ValidationError::InvalidFormat {
                field: "user_email".to_string(),
                message: "Email must be in valid format".to_string(),
            });
        }
    }

    Ok(())
}

/// Validate a project before creation or update
pub fn validate_project(project: &NewProject) -> Result<(), ValidationError> {
    // Validate project name
    if project.project_name.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "project_name".to_string(),
        });
    }

    if project.project_name.len() > 100 {
        return Err(ValidationError::TooLong {
            field: "project_name".to_string(),
            max: 100,
        });
    }

    if project.project_name.len() < 2 {
        return Err(ValidationError::TooShort {
            field: "project_name".to_string(),
            min: 2,
        });
    }

    // Validate description
    if let Some(description) = &project.project_description {
        if !description.trim().is_empty() && description.len() > 1000 {
            return Err(ValidationError::TooLong {
                field: "project_description".to_string(),
                max: 1000,
            });
        }
    }

    if project.project_owner_id.is_none() {
        return Err(ValidationError::Required {
            field: "project_owner_id".to_string(),
        });
    }

    Ok(())
}

/// Validate a requirement status before creation
pub fn validate_requirement_status(status: &NewStatus) -> Result<(), ValidationError> {
    // Validate status name
    if status.req_st_title.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "req_st_title".to_string(),
        });
    }

    if status.req_st_title.len() > 50 {
        return Err(ValidationError::TooLong {
            field: "req_st_title".to_string(),
            max: 50,
        });
    }

    if status.req_st_title.len() < 2 {
        return Err(ValidationError::TooShort {
            field: "req_st_title".to_string(),
            min: 2,
        });
    }

    // Validate description
    if !status.req_st_description.trim().is_empty() && status.req_st_description.len() > 200 {
        return Err(ValidationError::TooLong {
            field: "req_st_description".to_string(),
            max: 200,
        });
    }

    Ok(())
}

/// Validate a test status before creation
pub fn validate_test_status(status: &TestStatus) -> Result<(), ValidationError> {
    // Validate status name
    if status.test_st_title.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "test_st_title".to_string(),
        });
    }

    if status.test_st_title.len() > 50 {
        return Err(ValidationError::TooLong {
            field: "test_st_title".to_string(),
            max: 50,
        });
    }

    if status.test_st_title.len() < 2 {
        return Err(ValidationError::TooShort {
            field: "test_st_title".to_string(),
            min: 2,
        });
    }

    // Validate description
    if !status.test_st_description.trim().is_empty() && status.test_st_description.len() > 200 {
        return Err(ValidationError::TooLong {
            field: "test_st_description".to_string(),
            max: 200,
        });
    }

    Ok(())
}

/// Validate ID parameter
pub fn validate_id(id: i32, entity_name: &str) -> Result<(), ValidationError> {
    if id <= 0 {
        return Err(ValidationError::Custom(format!(
            "{} ID must be positive",
            entity_name
        )));
    }
    Ok(())
}

/// Sanitize string input by trimming whitespace
pub fn sanitize_string(input: &mut String) {
    *input = input.trim().to_string();
}

/// Sanitize optional string input
pub fn sanitize_optional_string(input: &mut Option<String>) {
    if let Some(ref mut s) = input {
        *s = s.trim().to_string();
        if s.is_empty() {
            *input = None;
        }
    }
}

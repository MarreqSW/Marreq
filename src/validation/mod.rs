// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

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
    if req.title.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "title".to_string(),
        });
    }

    if req.title.trim().len() < 3 {
        return Err(ValidationError::TooShort {
            field: "title".to_string(),
            min: 3,
        });
    }

    if req.title.len() > 255 {
        return Err(ValidationError::TooLong {
            field: "title".to_string(),
            max: 255,
        });
    }

    // Validate description
    if req.description.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "description".to_string(),
        });
    }

    if req.description.len() > 2000 {
        return Err(ValidationError::TooLong {
            field: "description".to_string(),
            max: 2000,
        });
    }

    // Validate reference format (should be like REQ-001, REQ-ABC-001, etc.)
    if !req.reference_code.trim().is_empty() {
        let ref_regex = Regex::new(r"^[A-Z]{2,4}(?:-[A-Z0-9]{1,6})+$").unwrap();
        if !ref_regex.is_match(&req.reference_code) {
            return Err(ValidationError::InvalidFormat {
                field: "reference_code".to_string(),
                message: "Reference should be in format like REQ-001 or REQ-ABC-001".to_string(),
            });
        }
    }

    // Validate IDs are positive (verification methods are validated separately as a list)
    if req.status_id <= 0 {
        return Err(ValidationError::Custom(
            "Status ID must be positive".to_string(),
        ));
    }

    if req.author_id <= 0 {
        return Err(ValidationError::Custom(
            "Author ID must be positive".to_string(),
        ));
    }

    if req.reviewer_id <= 0 {
        return Err(ValidationError::Custom(
            "Reviewer ID must be positive".to_string(),
        ));
    }

    if req.category_id <= 0 {
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
pub fn validate_test(test: &NewTestCase) -> Result<(), ValidationError> {
    // Validate test name
    if test.name.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "name".to_string(),
        });
    }

    if test.name.len() > 255 {
        return Err(ValidationError::TooLong {
            field: "name".to_string(),
            max: 255,
        });
    }

    if test.name.len() < 3 {
        return Err(ValidationError::TooShort {
            field: "name".to_string(),
            min: 3,
        });
    }

    // Validate test reference
    if test.reference_code.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "reference_code".to_string(),
        });
    }

    if test.reference_code.len() > 50 {
        return Err(ValidationError::TooLong {
            field: "reference_code".to_string(),
            max: 50,
        });
    }

    // Validate test reference format (TEST-NUMBER)
    let test_ref_regex = Regex::new(r"^TEST-\d+$").unwrap();
    if !test_ref_regex.is_match(&test.reference_code) {
        return Err(ValidationError::Custom(
            "Test reference must follow format TEST-NUMBER (e.g., TEST-1, TEST-2)".to_string(),
        ));
    }

    // Validate description
    if test.description.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "description".to_string(),
        });
    }

    if test.description.len() > 2000 {
        return Err(ValidationError::TooLong {
            field: "description".to_string(),
            max: 2000,
        });
    }

    // Validate IDs are positive
    if test.status_id <= 0 {
        return Err(ValidationError::Custom(
            "Test status ID must be positive".to_string(),
        ));
    }

    if let Some(parent_id) = test.parent_id {
        if parent_id <= 0 {
            return Err(ValidationError::Custom(
                "Test parent ID must be positive".to_string(),
            ));
        }
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
    if category.title.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "title".to_string(),
        });
    }

    if category.title.len() > 100 {
        return Err(ValidationError::TooLong {
            field: "title".to_string(),
            max: 100,
        });
    }

    if category.title.len() < 2 {
        return Err(ValidationError::TooShort {
            field: "title".to_string(),
            min: 2,
        });
    }

    // Validate description
    if category.description.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "description".to_string(),
        });
    }

    if category.description.len() > 500 {
        return Err(ValidationError::TooLong {
            field: "description".to_string(),
            max: 500,
        });
    }

    // Validate tag
    if category.tag.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "tag".to_string(),
        });
    }

    if category.tag.len() > 50 {
        return Err(ValidationError::TooLong {
            field: "tag".to_string(),
            max: 50,
        });
    }

    // Validate tag format (should be alphanumeric with underscores)
    let tag_regex = Regex::new(r"^[A-Za-z0-9_]+$").unwrap();
    if !tag_regex.is_match(&category.tag) {
        return Err(ValidationError::InvalidFormat {
            field: "tag".to_string(),
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
    if applicability.title.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "title".to_string(),
        });
    }

    if applicability.title.len() > 100 {
        return Err(ValidationError::TooLong {
            field: "title".to_string(),
            max: 100,
        });
    }

    if applicability.title.len() < 2 {
        return Err(ValidationError::TooShort {
            field: "title".to_string(),
            min: 2,
        });
    }

    // Validate description
    if applicability.description.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "description".to_string(),
        });
    }

    if applicability.description.len() > 500 {
        return Err(ValidationError::TooLong {
            field: "description".to_string(),
            max: 500,
        });
    }

    // Validate tag
    if applicability.tag.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "tag".to_string(),
        });
    }

    if applicability.tag.len() > 50 {
        return Err(ValidationError::TooLong {
            field: "tag".to_string(),
            max: 50,
        });
    }

    // Validate tag format
    let tag_regex = Regex::new(r"^[A-Za-z0-9_]+$").unwrap();
    if !tag_regex.is_match(&applicability.tag) {
        return Err(ValidationError::InvalidFormat {
            field: "tag".to_string(),
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
    if user.username.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "username".to_string(),
        });
    }

    if user.username.len() > 50 {
        return Err(ValidationError::TooLong {
            field: "username".to_string(),
            max: 50,
        });
    }

    if user.username.len() < 3 {
        return Err(ValidationError::TooShort {
            field: "username".to_string(),
            min: 3,
        });
    }

    // Validate username format (alphanumeric and underscores only)
    let username_regex = Regex::new(r"^[A-Za-z0-9_]+$").unwrap();
    if !username_regex.is_match(&user.username) {
        return Err(ValidationError::InvalidFormat {
            field: "username".to_string(),
            message: "Username should contain only letters, numbers, and underscores".to_string(),
        });
    }

    // Validate name
    if user.name.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "name".to_string(),
        });
    }

    if user.name.len() > 100 {
        return Err(ValidationError::TooLong {
            field: "name".to_string(),
            max: 100,
        });
    }

    if user.name.len() < 2 {
        return Err(ValidationError::TooShort {
            field: "name".to_string(),
            min: 2,
        });
    }

    // Validate email format if provided
    if !user.email.trim().is_empty() {
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        if !email_regex.is_match(&user.email) {
            return Err(ValidationError::InvalidFormat {
                field: "email".to_string(),
                message: "Email must be in valid format".to_string(),
            });
        }
    }

    Ok(())
}

/// Validate a project before creation or update
pub fn validate_project(project: &NewProject) -> Result<(), ValidationError> {
    // Validate project name
    if project.name.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "name".to_string(),
        });
    }

    if project.name.len() > 100 {
        return Err(ValidationError::TooLong {
            field: "name".to_string(),
            max: 100,
        });
    }

    if project.name.len() < 2 {
        return Err(ValidationError::TooShort {
            field: "name".to_string(),
            min: 2,
        });
    }

    // Validate description
    if let Some(description) = &project.description {
        if !description.trim().is_empty() && description.len() > 1000 {
            return Err(ValidationError::TooLong {
                field: "description".to_string(),
                max: 1000,
            });
        }
    }

    if project.owner_id.is_none() {
        return Err(ValidationError::Required {
            field: "owner_id".to_string(),
        });
    }

    Ok(())
}

/// Validate a requirement status before creation
pub fn validate_requirement_status(status: &NewRequirementStatus) -> Result<(), ValidationError> {
    // Validate status name
    if status.title.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "title".to_string(),
        });
    }

    if status.title.len() > 50 {
        return Err(ValidationError::TooLong {
            field: "title".to_string(),
            max: 50,
        });
    }

    if status.title.len() < 2 {
        return Err(ValidationError::TooShort {
            field: "title".to_string(),
            min: 2,
        });
    }

    // Validate description
    if !status.description.trim().is_empty() && status.description.len() > 200 {
        return Err(ValidationError::TooLong {
            field: "description".to_string(),
            max: 200,
        });
    }

    Ok(())
}

/// Validate a test status before creation
pub fn validate_test_status(status: &TestStatus) -> Result<(), ValidationError> {
    // Validate status name
    if status.title.trim().is_empty() {
        return Err(ValidationError::Required {
            field: "title".to_string(),
        });
    }

    if status.title.len() > 50 {
        return Err(ValidationError::TooLong {
            field: "title".to_string(),
            max: 50,
        });
    }

    if status.title.len() < 2 {
        return Err(ValidationError::TooShort {
            field: "title".to_string(),
            min: 2,
        });
    }

    // Validate description
    if !status.description.trim().is_empty() && status.description.len() > 200 {
        return Err(ValidationError::TooLong {
            field: "description".to_string(),
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

#[cfg(test)]
mod tests;

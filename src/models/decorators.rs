//! Decorated models with human-readable values.
//!
//! These structures extend the base database entities by replacing foreign key
//! IDs with their corresponding human-readable names and formatted values,
//! making them ideal for presentation in templates and UI.

use serde::{Deserialize, Serialize};

/// Requirement enriched with human readable values for presentation.
///
/// Foreign key fields are replaced by their corresponding names, simplifying
/// template rendering.
#[derive(Serialize, Deserialize)]
pub struct DecoratedRequirement {
    pub req_id: i32,
    pub req_title: String,
    pub req_description: String,
    pub req_verification: String,
    pub req_verification_id: i32,
    pub req_current_status: String,
    pub req_current_status_id: i32, // Add numeric status ID for access control
    pub req_author: String,
    pub req_author_id: i32,
    pub req_reviewer: String,
    pub req_reviewer_id: i32,
    pub req_reference: String,
    pub req_category: String,
    pub req_category_id: i32,
    pub req_applicability: String,
    pub req_applicability_id: i32,
    pub req_parent_id: i32,
    pub req_parent_title: String,
    pub req_creation_date: String,
    pub req_update_date: String,
    pub req_deadline_date: String,
    pub req_justification: Option<String>,
    pub project_id: i32,
}

/// Test information with resolved foreign keys for presentation.
#[derive(Serialize, Deserialize, Debug)]
pub struct DecoratedTest {
    pub test_id: i32,
    pub test_reference: String,
    pub test_name: String,
    pub test_description: String,
    pub test_source: String,
    pub test_status: String,
    pub test_status_id: i32, // Add numeric status ID for access control
    pub test_parent_id: i32,
    pub test_parent_title: String,
    pub project_id: i32,
}

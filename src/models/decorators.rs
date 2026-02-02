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
#[derive(Serialize, Deserialize, Clone)]
pub struct DecoratedRequirement {
    pub id: i32,
    pub title: String,
    pub description: String,
    /// Comma-separated verification method names for display
    pub verification_method_id: String,
    /// Verification method IDs for filtering and form pre-selection
    pub req_verification_ids: Vec<i32>,
    pub status_id: String,
    pub req_current_status_id: i32, // Add numeric status ID for access control
    pub author_id: String,
    pub req_author_id: i32,
    pub reviewer_id: String,
    pub req_reviewer_id: i32,
    pub reference_code: String,
    pub category_id: String,
    pub req_category_id: i32,
    pub applicability_id: String,
    pub req_applicability_id: i32,
    pub req_parent_id: Option<i32>,
    pub req_parent_title: String,
    pub creation_date: String,
    pub update_date: String,
    pub deadline_date: String,
    pub justification: Option<String>,
    pub project_id: i32,
}

/// Test case information with resolved foreign keys for presentation.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DecoratedTestCase {
    pub id: i32,
    pub reference_code: String,
    pub name: String,
    pub description: String,
    pub source: String,
    pub status_id: String,
    pub test_status_id: i32, // Add numeric status ID for access control
    pub test_parent_id: Option<i32>,
    pub test_parent_title: String,
    pub project_id: i32,
}

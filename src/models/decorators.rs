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
    /// ID of the current requirement_version (for version history UI).
    pub current_version_id: Option<i32>,
    pub title: String,
    pub description: String,
    /// Comma-separated verification method names for display
    pub verification_method_id: String,
    /// Verification method IDs for filtering and form pre-selection
    pub req_verification_ids: Vec<i32>,
    pub status_id: String,
    pub req_current_status_id: i32, // Add numeric status ID for access control
    /// Optional tag color (hex) for status badge background.
    pub status_tag_color: Option<String>,
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
    /// Parent's reference code (for hover preview when parent link is shown).
    pub req_parent_reference_code: String,
    /// Parent's description (for hover preview).
    pub req_parent_description: String,
    /// Parent's status title (for hover preview).
    pub req_parent_status_id: String,
    /// Parent's status tag color (hex) for badge, if set.
    pub req_parent_status_tag_color: Option<String>,
    /// Parent's category title (for hover preview).
    pub req_parent_category_id: String,
    pub creation_date: String,
    pub update_date: String,
    pub deadline_date: String,
    pub justification: Option<String>,
    pub project_id: i32,
    /// Approval workflow: draft | reviewed | approved (current version).
    pub approval_state: String,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::NaiveDateTime>,
    /// Project-scoped custom metadata (from requirement or version).
    pub custom_fields: Option<Vec<crate::models::CustomFieldValueDisplay>>,
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
    /// CSS variant for status badge: passed, failed, proposal, draft, default
    pub status_variant: String,
    pub test_status_id: i32, // Add numeric status ID for access control
    /// Optional tag color (hex) for status badge background.
    pub status_tag_color: Option<String>,
    pub test_parent_id: Option<i32>,
    pub test_parent_title: String,
    /// Parent test fields for hover preview card (same shape as main test).
    pub test_parent_reference_code: String,
    pub test_parent_description: String,
    pub test_parent_status_id: String,
    pub test_parent_status_variant: String,
    /// Parent's status tag color (hex) for badge, if set.
    pub test_parent_status_tag_color: Option<String>,
    pub test_parent_source: String,
    pub project_id: i32,
}

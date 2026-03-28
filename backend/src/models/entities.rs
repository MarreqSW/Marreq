// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Core database entities.
//!
//! These structures directly map to database tables and represent the persistent
//! state of the application.

use crate::logger::Loggable;
use crate::schema::*;
use crate::status_enums::ProjectStatus;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Logical requirement container (id, project_id, stable_code, current_version_id).
/// Content lives in [`RequirementVersion`]; the "current" requirement view is built from this + current version.
#[derive(Serialize, Deserialize, Queryable, Selectable, Clone, Debug)]
#[diesel(table_name = crate::schema::requirements)]
pub struct RequirementContainer {
    pub id: i32,
    pub project_id: i32,
    pub stable_code: String,
    pub current_version_id: Option<i32>,
    pub first_created_at: chrono::NaiveDateTime,
}

/// A single immutable requirement version (content snapshot).
#[derive(Serialize, Deserialize, Queryable, Selectable, Clone, Debug)]
#[diesel(table_name = crate::schema::requirement_versions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RequirementVersion {
    pub id: i32,
    pub requirement_id: i32,
    pub title: String,
    pub description: String,
    pub status_id: i32,
    pub author_id: i32,
    pub reviewer_id: i32,
    pub category_id: i32,
    pub applicability_id: i32,
    pub justification: Option<String>,
    pub deadline_date: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    /// Approval workflow: draft | reviewed | approved
    pub approval_state: String,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::NaiveDateTime>,
    /// User who transitioned this version to reviewed (if applicable).
    pub reviewed_by: Option<i32>,
    pub reviewed_at: Option<chrono::NaiveDateTime>,
}

/// One custom field value as returned in requirement payloads (field_id, label, value).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CustomFieldValueDisplay {
    pub field_id: i32,
    pub label: String,
    pub value: Option<String>,
}

/// Current view of a requirement (logical id + current version content).
/// Built from [`RequirementContainer`] + current [`RequirementVersion`] for API/UI compatibility.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Requirement {
    pub id: i32,
    /// ID of the current requirement_version row (for version history UI).
    pub current_version_id: Option<i32>,
    /// When true (baseline view only), the baseline version is the same as current; hide "Diff vs current".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_as_current: Option<bool>,
    pub title: String,
    pub description: String,
    pub status_id: i32,
    pub author_id: i32,
    pub reviewer_id: i32,
    pub reference_code: String,
    pub category_id: i32,
    pub parent_id: Option<i32>,
    pub creation_date: chrono::NaiveDateTime,
    pub update_date: chrono::NaiveDateTime,
    pub deadline_date: Option<chrono::NaiveDateTime>,
    pub applicability_id: i32,
    pub justification: Option<String>,
    pub project_id: i32,
    /// Approval state of the current version (draft | reviewed | approved).
    pub approval_state: String,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::NaiveDateTime>,
    /// Project-scoped custom metadata (empty if none defined or no values).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_fields: Option<Vec<CustomFieldValueDisplay>>,
}

impl Requirement {
    /// Set same_as_current (baseline context only). Chainable for builder-style.
    #[allow(dead_code)]
    pub fn with_same_as_current(mut self, same: bool) -> Self {
        self.same_as_current = Some(same);
        self
    }
}

/// Immutable comment on a requirement (general) or a specific requirement version.
#[derive(Serialize, Deserialize, Queryable, Clone, Debug)]
#[diesel(table_name = crate::schema::requirement_comments)]
pub struct RequirementComment {
    pub id: i32,
    pub requirement_id: i32,
    pub requirement_version_id: Option<i32>,
    pub author_id: i32,
    pub body: String,
    pub created_at: chrono::NaiveDateTime,
}

/// New comment to insert (no id; created_at from DB default).
#[derive(Serialize, Deserialize, Insertable, Clone, Debug)]
#[diesel(table_name = crate::schema::requirement_comments)]
pub struct NewRequirementComment {
    pub requirement_id: i32,
    pub requirement_version_id: Option<i32>,
    pub author_id: i32,
    pub body: String,
}

/// Link between a requirement version and a verification method (many-to-many).
#[derive(Serialize, Deserialize, Queryable, Insertable, Clone, Debug)]
#[diesel(table_name = crate::schema::requirement_version_verification_methods)]
pub struct RequirementVersionVerificationMethod {
    pub requirement_version_id: i32,
    pub verification_method_id: i32,
}

/// Typed link between two requirement versions (e.g. DERIVES_FROM, REFINES).
/// source_version_id = child, target_version_id = parent.
#[derive(Serialize, Deserialize, Queryable, Clone, Debug)]
#[diesel(table_name = crate::schema::requirement_version_links)]
pub struct RequirementVersionLink {
    pub id: i32,
    pub source_version_id: i32,
    pub target_version_id: i32,
    pub link_type: String,
    pub rationale: Option<String>,
    pub project_id: i32,
    pub created_at: chrono::NaiveDateTime,
    pub metadata: Option<serde_json::Value>,
}

/// Insertable row for a new requirement version link.
#[derive(Serialize, Deserialize, Insertable, Clone, Debug)]
#[diesel(table_name = crate::schema::requirement_version_links)]
pub struct NewRequirementVersionLink {
    pub source_version_id: i32,
    pub target_version_id: i32,
    pub link_type: String,
    pub rationale: Option<String>,
    pub project_id: i32,
    pub metadata: Option<serde_json::Value>,
}

/// Link between a requirement and a test in the traceability matrix.
/// Suspect state: when a requirement changes, links are marked suspect until reviewed.
#[derive(Serialize, Deserialize, Queryable, Clone, Debug)]
#[diesel(table_name = crate::schema::matrix)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MatrixLink {
    pub req_id: i32,
    pub verification_id: i32,
    pub creation_date: chrono::NaiveDateTime,
    pub project_id: i32,
    pub suspect: bool,
    pub suspect_at: Option<chrono::NaiveDateTime>,
    pub suspect_reason: Option<String>,
    pub cleared_by: Option<i32>,
    pub cleared_at: Option<chrono::NaiveDateTime>,
    pub triggering_version_id: Option<i32>,
    pub triggering_user_id: Option<i32>,
}

/// Immutable project baseline (snapshot of requirement versions and traceability at creation time).
#[derive(Serialize, Deserialize, Queryable, Clone, Debug)]
#[diesel(table_name = crate::schema::baselines)]
pub struct Baseline {
    pub id: i32,
    pub project_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub created_by: i32,
}

/// Snapshot row: which requirement_version was in the baseline for each requirement.
#[derive(Serialize, Deserialize, Queryable, Clone, Debug)]
#[diesel(table_name = crate::schema::baseline_requirements)]
pub struct BaselineRequirement {
    pub baseline_id: i32,
    pub requirement_id: i32,
    pub version_id: i32,
}

/// Snapshot row: requirement–verification traceability at baseline time.
#[derive(Serialize, Deserialize, Queryable, Clone, Debug)]
#[diesel(table_name = crate::schema::baseline_traceability)]
pub struct BaselineTraceability {
    pub baseline_id: i32,
    pub requirement_id: i32,
    pub verification_id: i32,
    pub suspect: bool,
    pub suspect_at: Option<chrono::NaiveDateTime>,
    pub suspect_reason: Option<String>,
}

/// Snapshot row: verification as at baseline time (denormalized copy).
#[derive(Serialize, Deserialize, Queryable, Clone, Debug)]
#[diesel(table_name = crate::schema::baseline_verifications)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct BaselineVerification {
    pub baseline_id: i32,
    pub verification_id: i32,
    pub name: String,
    pub reference_code: String,
    pub description: String,
    pub source: String,
    pub status_id: i32,
    pub parent_id: Option<i32>,
    pub project_id: i32,
    pub verification_method_id: Option<i32>,
    pub author_id: i32,
    pub reviewer_id: i32,
}

/// A system user that can access projects and manage requirements.
///
/// # Security Note
/// The `password_hash` field is protected with `#[serde(skip_serializing)]` to prevent
/// accidental exposure in API responses. Never remove this attribute without creating a
/// dedicated public DTO type that excludes sensitive fields.
///
/// When creating users via API, use [`UserCreateRequest`](crate::models::UserCreateRequest)
/// which accepts plain passwords and hashes them server-side.
#[derive(Serialize, Deserialize, Queryable, AsChangeset, Debug, Clone)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub name: String,
    pub email: String,
    pub creation_date: chrono::NaiveDateTime,
    pub last_login: chrono::NaiveDateTime,
    #[serde(skip_serializing, default)]
    pub password_hash: String,
    pub is_admin: bool,
}

/// A verification (formerly test case) that can verify one or more requirements.
#[derive(Serialize, Deserialize, Queryable, Clone, Debug)]
#[diesel(table_name = crate::schema::verifications)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Verification {
    pub id: i32,
    pub name: String,
    pub reference_code: String,
    pub description: String,
    pub source: String,
    pub status_id: i32,
    pub parent_id: Option<i32>,
    pub project_id: i32,
    /// Project-scoped verification method (e.g. Test, Analysis, Review).
    pub verification_method_id: Option<i32>,
    pub author_id: i32,
    pub reviewer_id: i32,
    /// Last user (project reviewer) who changed verification status.
    pub status_set_by: Option<i32>,
    pub status_set_at: Option<chrono::NaiveDateTime>,
}

/// A group organizes multiple related projects under a shared container.
#[derive(Queryable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::schema::groups)]
pub struct Group {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub owner_id: Option<i32>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// Membership that links a user to a group with a specific role.
/// Roles: 1 = Owner, 2 = Maintainer, 3 = Contributor, 4 = Viewer
#[derive(Queryable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::schema::group_members)]
#[diesel(primary_key(group_id, user_id))]
pub struct GroupMember {
    pub group_id: i32,
    pub user_id: i32,
    pub role: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// A project groups a collection of requirements and tests.
#[derive(Queryable, Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub creation_date: Option<chrono::NaiveDateTime>,
    pub update_date: Option<chrono::NaiveDateTime>,
    pub status: ProjectStatus,
    pub owner_id: Option<i32>,
    pub slug: String,
    pub group_id: Option<i32>,
}

/// Membership that links a user to a project with a specific role.
#[derive(Queryable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::schema::project_members)]
#[diesel(primary_key(project_id, user_id))]
pub struct ProjectMember {
    pub project_id: i32,
    pub user_id: i32,
    pub role: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

/// A single audit log entry describing a user action.
#[derive(Queryable, Serialize, Deserialize, Debug, Clone)]
pub struct Log {
    pub log_id: i32,
    #[serde(rename = "id")]
    #[diesel(column_name = id)]
    pub user_id: i32,
    pub action_type: String,
    pub entity_type: String,
    pub entity_id: Option<i32>,
    pub project_id: Option<i32>,
    pub old_values: Option<String>,
    pub new_values: Option<String>,
    pub description: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

/// Macro to define tagged entities with common structure.
macro_rules! define_tagged_entity {
    (
        $name:ident
    ) => {
        #[derive(Serialize, Deserialize, Queryable, Clone)]
        #[diesel(check_for_backend(diesel::pg::Pg))]
        pub struct $name {
            pub id: i32,
            pub title: String,
            pub description: String,
            pub tag: String,
            pub project_id: i32,
        }
    };
}

define_tagged_entity!(Category);
define_tagged_entity!(Applicability);

/// Requirement status with optional system flag (system statuses are immutable).
#[derive(Debug, Serialize, Deserialize, Queryable, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RequirementStatus {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub tag: String,
    pub project_id: i32,
    pub is_system: bool,
    pub tag_color: Option<String>,
}

/// Verification status with optional system flag (system statuses are immutable).
#[derive(Debug, Serialize, Deserialize, Queryable, Clone)]
#[diesel(table_name = crate::schema::verification_status)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct VerificationStatus {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub tag: String,
    pub project_id: i32,
    pub is_system: bool,
    pub tag_color: Option<String>,
}

#[derive(Serialize, Deserialize, Queryable, Clone)]
#[diesel(table_name = crate::schema::verification_methods)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct VerificationMethod {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub tag: String,
    pub project_id: i32,
}

/// Project-scoped custom field definition (label, type, optional enum values).
#[derive(Serialize, Deserialize, Queryable, Clone, Debug)]
#[diesel(table_name = crate::schema::custom_field_definitions)]
pub struct CustomFieldDefinition {
    pub id: i32,
    pub project_id: i32,
    pub label: String,
    pub field_type: String,
    pub enum_values: Option<serde_json::Value>,
    pub sort_order: i32,
    pub created_at: chrono::NaiveDateTime,
}

/// One stored custom field value per requirement version.
#[derive(Serialize, Deserialize, Queryable, Insertable, Clone, Debug)]
#[diesel(table_name = crate::schema::custom_field_values)]
pub struct CustomFieldValue {
    pub requirement_version_id: i32,
    pub custom_field_definition_id: i32,
    pub value: Option<String>,
}

/// Different categories of actions that can appear in the audit log.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActionType {
    Create,
    Update,
    Delete,
    Login,
    Logout,
    Export,
    Import,
    StatusChange,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionType::Create => write!(f, "CREATE"),
            ActionType::Update => write!(f, "UPDATE"),
            ActionType::Delete => write!(f, "DELETE"),
            ActionType::Login => write!(f, "LOGIN"),
            ActionType::Logout => write!(f, "LOGOUT"),
            ActionType::Export => write!(f, "EXPORT"),
            ActionType::Import => write!(f, "IMPORT"),
            ActionType::StatusChange => write!(f, "STATUS_CHANGE"),
        }
    }
}

impl ActionType {
    /// Human-friendly past tense description used in audit log messages.
    pub fn past_tense(self) -> &'static str {
        match self {
            ActionType::Create => "Created",
            ActionType::Update => "Updated",
            ActionType::Delete => "Deleted",
            ActionType::Login => "Logged in",
            ActionType::Logout => "Logged out",
            ActionType::Export => "Exported",
            ActionType::Import => "Imported",
            ActionType::StatusChange => "Changed status", // catch-all phrasing
        }
    }
}

/// Entities that can be referenced by a [`Log`] entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    Group,
    Project,
    Requirement,
    Category,
    Applicability,
    User,
    MatrixLink,
    /// Verification (formerly test case) that verifies requirements.
    Verification,
    /// Verification method (e.g. Test, Analysis, Review) attached to requirement versions.
    VerificationMethod,
    Comment,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::Group => write!(f, "GROUP"),
            EntityType::Project => write!(f, "PROJECT"),
            EntityType::Requirement => write!(f, "REQUIREMENT"),
            EntityType::Category => write!(f, "CATEGORY"),
            EntityType::Applicability => write!(f, "APPLICABILITY"),
            EntityType::User => write!(f, "USER"),
            EntityType::MatrixLink => write!(f, "MATRIX"),
            EntityType::Verification => write!(f, "VERIFICATION"),
            EntityType::VerificationMethod => write!(f, "VERIFICATION_METHOD"),
            EntityType::Comment => write!(f, "COMMENT"),
        }
    }
}

impl EntityType {
    /// Lowercase, human-oriented label for log descriptions.
    pub fn human_name(self) -> &'static str {
        match self {
            EntityType::Group => "group",
            EntityType::Project => "project",
            EntityType::Requirement => "requirement",
            EntityType::Category => "category",
            EntityType::Applicability => "applicability",
            EntityType::User => "user",
            EntityType::MatrixLink => "matrix",
            EntityType::Verification => "verification",
            EntityType::VerificationMethod => "verification method",
            EntityType::Comment => "comment",
        }
    }
}

// Display implementations for entities.
// These provide minimal, text-based representations suitable for logging,
// debugging, or simple text output. HTML rendering should be handled by
// templates or view-layer helpers for better separation of concerns.
impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Requirement #{}: {}", self.id, self.title)
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Category: {}", self.title)
    }
}

impl fmt::Display for Applicability {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Applicability: {}", self.title)
    }
}

impl fmt::Display for RequirementStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Status: {}", self.title)
    }
}

impl fmt::Display for MatrixLink {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Matrix: Req {} <-> Verification {}",
            self.req_id, self.verification_id
        )
    }
}

impl fmt::Display for Verification {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Verification #{}: {}", self.id, self.name)
    }
}

// Loggable implementations
// This macro provides Loggable trait implementations for entity types.
// It handles two cases: entities with project_id (most common) and entities
// without project_id (only User and NewUser). The macro is kept simple since
// there are only these two patterns and they're unlikely to grow.
macro_rules! impl_loggable {
    // Special case: no project_id (used only for User entities)
    ($ty:ty, $entity:expr, $name:ident, no_project) => {
        impl Loggable for $ty {
            fn entity_type() -> EntityType {
                $entity
            }
            fn id(&self) -> i32 {
                self.id
            }
            fn project_id(&self) -> Option<i32> {
                None
            }
            fn display_name(&self) -> String {
                self.$name.clone()
            }
        }
    };

    // For types with a distinct `id` and `project_id` field
    ($ty:ty, $entity:expr, $project:ident, $name:ident) => {
        impl Loggable for $ty {
            fn entity_type() -> EntityType {
                $entity
            }
            fn id(&self) -> i32 {
                self.id
            }
            fn project_id(&self) -> Option<i32> {
                Some(self.$project)
            }
            fn display_name(&self) -> String {
                self.$name.clone()
            }
        }
    };
}

impl Loggable for Group {
    fn entity_type() -> EntityType {
        EntityType::Group
    }
    fn id(&self) -> i32 {
        self.id
    }
    fn project_id(&self) -> Option<i32> {
        None
    }
    fn display_name(&self) -> String {
        self.name.clone()
    }
}

impl_loggable!(Project, EntityType::Project, id, name);
impl_loggable!(Requirement, EntityType::Requirement, project_id, title);
impl_loggable!(Category, EntityType::Category, project_id, title);
impl_loggable!(Applicability, EntityType::Applicability, project_id, title);
impl_loggable!(Verification, EntityType::Verification, project_id, name);
impl_loggable!(
    VerificationMethod,
    EntityType::VerificationMethod,
    project_id,
    title
);
impl_loggable!(User, EntityType::User, username, no_project);

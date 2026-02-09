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
#[derive(Serialize, Deserialize, Queryable, Clone, Debug)]
#[diesel(table_name = crate::schema::requirements)]
pub struct RequirementContainer {
    pub id: i32,
    pub project_id: i32,
    pub stable_code: String,
    pub current_version_id: Option<i32>,
}

/// A single immutable requirement version (content snapshot).
#[derive(Serialize, Deserialize, Queryable, Clone, Debug)]
#[diesel(table_name = crate::schema::requirement_versions)]
pub struct RequirementVersion {
    pub id: i32,
    pub requirement_id: i32,
    pub title: String,
    pub description: String,
    pub status_id: i32,
    pub author_id: i32,
    pub reviewer_id: i32,
    pub category_id: i32,
    pub parent_id: Option<i32>,
    pub applicability_id: i32,
    pub justification: Option<String>,
    pub deadline_date: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    /// Approval workflow: draft | reviewed | approved
    pub approval_state: String,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::NaiveDateTime>,
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
}

impl Requirement {
    /// Set same_as_current (baseline context only). Chainable for builder-style.
    #[allow(dead_code)]
    pub fn with_same_as_current(mut self, same: bool) -> Self {
        self.same_as_current = Some(same);
        self
    }
}

/// Link between a requirement version and a verification method (many-to-many).
#[derive(Serialize, Deserialize, Queryable, Insertable, Clone, Debug)]
#[diesel(table_name = crate::schema::requirement_version_verification_methods)]
pub struct RequirementVersionVerificationMethod {
    pub requirement_version_id: i32,
    pub verification_method_id: i32,
}

/// Link between a requirement and a test in the traceability matrix.
/// Suspect state: when a requirement changes, links are marked suspect until reviewed.
#[derive(Serialize, Deserialize, Queryable, Clone, Debug)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MatrixLink {
    pub req_id: i32,
    pub test_id: i32,
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

/// Snapshot row: requirement–test traceability at baseline time.
#[derive(Serialize, Deserialize, Queryable, Clone, Debug)]
#[diesel(table_name = crate::schema::baseline_traceability)]
pub struct BaselineTraceability {
    pub baseline_id: i32,
    pub requirement_id: i32,
    pub test_id: i32,
    pub suspect: bool,
    pub suspect_at: Option<chrono::NaiveDateTime>,
    pub suspect_reason: Option<String>,
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

/// A test case that can verify one or more requirements.
#[derive(Serialize, Deserialize, Queryable, Clone, Debug)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TestCase {
    pub id: i32,
    pub name: String,
    pub reference_code: String,
    pub description: String,
    pub source: String,
    pub status_id: i32,
    pub parent_id: Option<i32>,
    pub project_id: i32,
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
define_tagged_entity!(RequirementStatus);
define_tagged_entity!(TestStatus);
define_tagged_entity!(VerificationMethod);

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
    Project,
    Requirement,
    Test,
    Category,
    Applicability,
    User,
    MatrixLink,
    Verification,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::Project => write!(f, "PROJECT"),
            EntityType::Requirement => write!(f, "REQUIREMENT"),
            EntityType::Test => write!(f, "TEST"),
            EntityType::Category => write!(f, "CATEGORY"),
            EntityType::Applicability => write!(f, "APPLICABILITY"),
            EntityType::User => write!(f, "USER"),
            EntityType::MatrixLink => write!(f, "MATRIX"),
            EntityType::Verification => write!(f, "VERIFICATION"),
        }
    }
}

impl EntityType {
    /// Lowercase, human-oriented label for log descriptions.
    pub fn human_name(self) -> &'static str {
        match self {
            EntityType::Project => "project",
            EntityType::Requirement => "requirement",
            EntityType::Test => "test",
            EntityType::Category => "category",
            EntityType::Applicability => "applicability",
            EntityType::User => "user",
            EntityType::MatrixLink => "matrix",
            EntityType::Verification => "verification",
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
        write!(f, "Matrix: Req {} <-> Test {}", self.req_id, self.test_id)
    }
}

impl fmt::Display for TestCase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Test #{}: {}", self.id, self.name)
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

impl_loggable!(Project, EntityType::Project, id, name);
impl_loggable!(Requirement, EntityType::Requirement, project_id, title);
impl_loggable!(Category, EntityType::Category, project_id, title);
impl_loggable!(Applicability, EntityType::Applicability, project_id, title);
impl_loggable!(TestCase, EntityType::Test, project_id, name);
impl_loggable!(User, EntityType::User, username, no_project);

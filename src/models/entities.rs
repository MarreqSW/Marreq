//! Core database entities.
//!
//! These structures directly map to database tables and represent the persistent
//! state of the application.

use crate::logger::Loggable;
use crate::schema::*;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/// A single requirement stored in the `requirements` table.
///
/// Mirrors the database representation and is used when fetching or updating
/// existing requirements.
#[derive(Serialize, Deserialize, Queryable, AsChangeset, Clone)]
pub struct Requirement {
    pub req_id: i32,
    pub req_title: String,
    pub req_description: String,
    pub req_verification: i32,
    pub req_current_status: i32,
    pub req_author: i32,
    pub req_reviewer: i32,
    pub req_reference: String,
    pub req_category: i32,
    pub req_parent: i32,
    pub req_creation_date: chrono::NaiveDateTime,
    pub req_update_date: chrono::NaiveDateTime,
    pub req_deadline_date: chrono::NaiveDateTime,
    pub req_applicability: i32,
    pub req_justification: Option<String>,
    pub project_id: i32,
}

/// A grouping category for requirements.
#[derive(Serialize, Deserialize, Queryable, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Category {
    pub cat_id: i32,
    pub cat_title: String,
    pub cat_description: String,
    pub cat_tag: String,
    pub project_id: i32,
}

/// Applicability tags limit where a requirement applies.
#[derive(Serialize, Deserialize, Queryable, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Applicability {
    pub app_id: i32,
    pub app_title: String,
    pub app_description: String,
    pub app_tag: String,
    pub project_id: i32,
}

/// Possible status values for requirements.
#[derive(Serialize, Deserialize, Queryable, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = requirement_status)]
pub struct RequirementStatus {
    pub req_st_id: i32,
    pub req_st_title: String,
    pub req_st_description: String,
    pub req_st_short_name: String,
}

/// Possible status values for tests.
#[derive(Serialize, Deserialize, Queryable, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(table_name = test_status)]
pub struct TestStatus {
    pub test_st_id: i32,
    pub test_st_title: String,
    pub test_st_description: String,
    pub test_st_short_name: String,
}

// Keep the old Status struct for backward compatibility
#[derive(Serialize, Deserialize, Clone)]
pub struct Status {
    pub st_id: i32,
    pub st_title: String,
    pub st_description: String,
    pub st_short_name: String,
}

/// Verification methods available for requirements.
#[derive(Serialize, Deserialize, Queryable, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Verification {
    pub verification_id: i32,
    pub verification_name: String,
    pub verification_description: String,
    pub project_id: i32,
}

/// Link between a requirement and a test in the traceability matrix.
#[derive(Serialize, Deserialize, Queryable, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Matrix {
    pub matrix_req_id: i32,
    pub matrix_test_id: i32,
    pub matrix_creation_date: chrono::NaiveDateTime,
    pub project_id: i32,
}

/// A system user that can access projects and manage requirements.
#[derive(Serialize, Deserialize, Queryable, AsChangeset, Debug, Clone)]
pub struct User {
    pub user_id: i32,
    pub user_username: String,
    pub user_name: String,
    pub user_email: String,
    pub user_creation_date: chrono::NaiveDateTime,
    pub user_last_login: chrono::NaiveDateTime,
    pub user_password: String,
    pub is_admin: bool,
}

/// A test case that can verify one or more requirements.
#[derive(Serialize, Deserialize, Queryable, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Test {
    pub test_id: i32,
    pub test_name: String,
    pub test_reference: String,
    pub test_description: String,
    pub test_source: String,
    pub test_status: i32,
    pub test_parent: i32,
    pub project_id: i32,
}

/// A project groups a collection of requirements and tests.
#[derive(Queryable, Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub project_id: i32,
    pub project_name: String,
    pub project_description: Option<String>,
    pub project_creation_date: Option<chrono::NaiveDateTime>,
    pub project_update_date: Option<chrono::NaiveDateTime>,
    pub project_status: Option<String>,
    pub project_owner_id: Option<i32>,
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
#[derive(Queryable, Serialize, Deserialize, Debug)]
pub struct Log {
    pub log_id: i32,
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
#[derive(Debug, Clone, Copy)]
pub enum EntityType {
    Project,
    Requirement,
    Test,
    Category,
    Applicability,
    User,
    Matrix,
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
            EntityType::Matrix => write!(f, "MATRIX"),
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
            EntityType::Matrix => "matrix",
            EntityType::Verification => "verification",
        }
    }
}

// Display implementations for entities
impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "
        <div class='requirement'>
            <div class='ReqNum'>Num: <a href='http://localhost:8000/p/{}/requirements/show/{}'>{}</a></div>
            <div class='ReqTitle'>Title: {}</div>
            <div class='ReqDesc'>Description: {}</div>
            <div class='ReqAuthor'>Author: {}</div>
            <div class='ReqRef'>Reference {}</div>
            <div class='ReqDate'>Date: {}</div>
            <div class='ReqParent'>Parent: {}</div>
        </div>",
            self.project_id,
            self.req_id,
            self.req_id,
            self.req_title,
            self.req_description,
            self.req_author,
            self.req_reference,
            self.req_creation_date,
            self.req_parent
        )
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<div class='category'>Category: {}</div>",
            self.cat_title
        )
    }
}

impl fmt::Display for Applicability {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<div class='applicability'>Applicability: {}</div>",
            self.app_title
        )
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<div class='status'>Status: {}</div>", self.st_title)
    }
}

impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "
        <div class='matrixID'>Req ID: {}</div>
        <div class='matrixID'>Test ID: {}</div>",
            self.matrix_req_id, self.matrix_test_id
        )
    }
}

impl fmt::Display for Test {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "
        <div class='TestDiv'>
        <div class='testID'>Test ID: <a href='http://localhost:8000/tests/{}'>{}</a></div>
        <div class='testName'>Name: {}</div>
        <div class='testDescription'>Description: {}</div>
        <div class='testSource'>Source: {}</div>
        <div class='testParent'>Parent: {}</div>
        </div>
        ",
            self.test_id,
            self.test_id,
            self.test_name,
            self.test_description,
            self.test_source,
            self.test_parent
        )
    }
}

// Loggable implementations
macro_rules! impl_loggable {
    // For types with direct `id`
    ($ty:ty, $entity:expr, $id:ident, $name:ident) => {
        impl Loggable for $ty {
            fn entity_type() -> EntityType {
                $entity
            }
            fn id(&self) -> i32 {
                self.$id
            }
            fn project_id(&self) -> Option<i32> {
                Some(self.project_id)
            }
            fn display_name(&self) -> String {
                self.$name.clone()
            }
        }
    };

    // Special case: no project_id
    ($ty:ty, $entity:expr, $id:ident, $name:ident, no_project) => {
        impl Loggable for $ty {
            fn entity_type() -> EntityType {
                $entity
            }
            fn id(&self) -> i32 {
                self.$id
            }
            fn project_id(&self) -> Option<i32> {
                None
            }
            fn display_name(&self) -> String {
                self.$name.clone()
            }
        }
    };
}

impl_loggable!(Project, EntityType::Project, project_id, project_name);
impl_loggable!(Requirement, EntityType::Requirement, req_id, req_title);
impl_loggable!(Category, EntityType::Category, cat_id, cat_title);
impl_loggable!(Applicability, EntityType::Applicability, app_id, app_title);
impl_loggable!(Test, EntityType::Test, test_id, test_name);
impl_loggable!(User, EntityType::User, user_id, user_username, no_project);

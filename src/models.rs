//! Data models for the requirement management application.
//!
//! These structures describe the core entities stored in the database and the
//! auxiliary forms used to create or update them. Most of the types derive
//! Diesel traits so they can be mapped directly to the PostgreSQL database.

use crate::logger::Loggable;
use crate::schema::*;
use diesel::prelude::*;
use std::fmt;

use serde::{Deserialize, Serialize};

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
    pub req_link: String,
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

/// Data required to insert or update a requirement.
///
/// Typically populated from HTTP forms when creating or editing requirements.
#[derive(Serialize, Deserialize, Insertable, AsChangeset, FromForm)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = requirements)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(req_id))]
pub struct NewRequirement {
    pub req_id: Option<i32>,
    pub req_title: String,
    pub req_description: String,
    pub req_verification: i32,
    pub req_author: i32,
    pub req_link: String,
    pub req_category: i32,
    pub req_current_status: i32,
    pub req_parent: i32,
    pub req_reference: String,
    pub req_reviewer: i32,
    pub req_applicability: i32,
    pub req_justification: Option<String>,
    pub project_id: i32,
}

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
    pub req_link: String,
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

/// Form used to insert or update a [`Category`].
#[derive(Serialize, Deserialize, Insertable, FromForm, AsChangeset, Clone)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = categories)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(cat_id))]
pub struct NewCategory {
    pub cat_id: Option<i32>,
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

/// Form used to insert or update an [`Applicability`].
#[derive(Serialize, Deserialize, Insertable, FromForm, AsChangeset, Clone)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = applicability)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(app_id))]
pub struct NewApplicability {
    pub app_id: Option<i32>,
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

/// Form used to create a new [`Status`].
#[derive(Serialize, Deserialize, Insertable, FromForm)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = requirement_status)]
pub struct NewStatus {
    pub req_st_title: String,
    pub req_st_description: String,
    pub req_st_short_name: String,
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

/// Form used to create a new [`Matrix`] entry tying a requirement to a test.
#[derive(Serialize, Deserialize, Insertable)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = matrix)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewMatrix {
    pub matrix_req_id: i32,
    pub matrix_test_id: i32,
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

/// Form used to insert or update [`User`] records.
#[derive(Serialize, Deserialize, Queryable, Insertable, AsChangeset, FromForm)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(user_id))]
pub struct NewUser {
    pub user_id: Option<i32>,
    pub user_username: String,
    pub user_name: String,
    pub user_email: String,
    pub user_password: String,
    pub is_admin: bool,
}

/// Partial user information used when editing an existing user.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct UpdateUser {
    pub user_id: Option<i32>,
    pub user_username: String,
    pub user_name: String,
    pub user_email: String,
    pub is_admin: bool,
}

/// A test case that can verify one or more requirements.
#[derive(Serialize, Deserialize, Queryable, Clone)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Test {
    pub test_id: i32,
    pub test_name: String,
    pub test_description: String,
    pub test_source: String,
    pub test_status: i32,
    pub test_reference: String,
    pub test_parent: i32,
    pub project_id: i32,
}

/// Test information with resolved foreign keys for presentation.
#[derive(Serialize, Deserialize, Debug)]
pub struct DecoratedTest {
    pub test_id: i32,
    pub test_name: String,
    pub test_description: String,
    pub test_source: String,
    pub test_status: String,
    pub test_status_id: i32, // Add numeric status ID for access control
    pub test_reference: String,
    pub test_parent_id: i32,
    pub test_parent_title: String,
    pub project_id: i32,
}

/// Form used to create or update a [`Test`].
#[derive(Serialize, Deserialize, Insertable, FromForm, AsChangeset)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = tests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewTest {
    pub test_id: Option<i32>,
    pub test_name: String,
    pub test_description: String,
    pub test_source: String,
    pub test_status: i32,
    pub test_reference: String,
    pub test_parent: i32,
    pub project_id: i32,
}

/// Form data submitted when creating a new test along with linked
/// requirements.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct NewTestForm {
    pub test_name: String,
    pub test_description: String,
    pub test_source: String,
    pub test_status: i32,
    pub test_parent: i32,
    pub test_req: Vec<i32>,
    pub project_id: i32,
}

/// Form used for editing an existing test and updating its requirement links.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct EditTestForm {
    pub test_id: i32,
    pub test_name: String,
    pub test_description: String,
    pub test_source: String,
    pub test_status: i32,
    pub test_parent: i32,
    pub linked_requirements: Vec<i32>,
    pub project_id: i32,
}

/// Credentials submitted during user login.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

/// Form used when a user requests to change their password.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct ChangePasswordForm {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "
        <div class='requirement'>
            <div class='ReqNum'>Num: <a href='http://localhost:8000/requirements/{}'>{}</a></div>
            <div class='ReqTitle'>Title: {}</div>
            <div class='ReqDesc'>Description: {}</div>
            <div class='ReqAuthor'>Author: {}</div>
            <div class='ReqRef'>Reference {}</div>
            <div class='ReqDate'>Date: {}</div>
            <div class='ReqParent'>Parent: {}</div>
        </div>",
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

impl fmt::Display for NewRequirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "
        <div class='requirement'>
            <div class='ReqTitle'>Title: {}</div><div class='ReqDesc'>Description: {}</div>
            <div class='ReqAuthor'>Author: {}</div>
        </div>",
            self.req_title, self.req_description, self.req_author
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

/// Data required to create or update a project membership entry.
#[derive(Insertable, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::project_members)]
pub struct NewProjectMember {
    pub project_id: i32,
    pub user_id: i32,
    pub role: i32,
}

/// Data required to create a new [`Project`].
#[derive(Insertable, Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::projects)]
pub struct NewProject {
    pub project_name: String,
    pub project_description: Option<String>,
    pub project_status: String,
    pub project_owner_id: Option<i32>,
}

/// Form used to update a project's metadata.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct UpdateProject {
    pub project_name: String,
    pub project_description: Option<String>,
    pub project_status: String,
    pub project_owner_id: Option<i32>,
}

/// Form that stores temporary column mappings for data imports.
#[derive(FromForm)]
pub struct ImportMappingForm {
    pub column_mappings: String,
    pub import_type: String,
    pub temp_file: String,
}

// Logging models
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

/// Form used to insert a new [`Log`] entry.
#[derive(Insertable, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::logs)]
pub struct NewLog {
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

macro_rules! impl_loggable {
    // For types with direct `id`
    ($ty:ty, $entity:expr, $id:ident, $name:ident) => {
        impl Loggable for $ty {
            fn entity_type() -> EntityType { $entity }
            fn id(&self) -> i32 { self.$id }
            fn project_id(&self) -> Option<i32> { Some(self.project_id) }
            fn display_name(&self) -> String { self.$name.clone() }
        }
    };

    // For types with `Option` id
    ($ty:ty, $entity:expr, $id:ident?, $name:ident) => {
        impl Loggable for $ty {
            fn entity_type() -> EntityType { $entity }
            fn id(&self) -> i32 { self.$id.unwrap_or_default() }
            fn project_id(&self) -> Option<i32> { Some(self.project_id) }
            fn display_name(&self) -> String { self.$name.clone() }
        }
    };

    // Special case: no project_id
    ($ty:ty, $entity:expr, $id:ident, $name:ident, no_project) => {
        impl Loggable for $ty {
            fn entity_type() -> EntityType { $entity }
            fn id(&self) -> i32 { self.$id }
            fn project_id(&self) -> Option<i32> { None }
            fn display_name(&self) -> String { self.$name.clone() }
        }
    };

    ($ty:ty, $entity:expr, $id:ident?, $name:ident, no_project) => {
        impl Loggable for $ty {
            fn entity_type() -> EntityType { $entity }
            fn id(&self) -> i32 { self.$id.unwrap_or_default() }
            fn project_id(&self) -> Option<i32> { None }
            fn display_name(&self) -> String { self.$name.clone() }
        }
    };
}

impl_loggable!(Project,          EntityType::Project,       project_id, project_name);
impl_loggable!(Requirement,      EntityType::Requirement,   req_id,     req_title);
impl_loggable!(NewRequirement,   EntityType::Requirement,   req_id?,    req_title);
impl_loggable!(Category,         EntityType::Category,      cat_id,     cat_title);
impl_loggable!(NewCategory,      EntityType::Category,      cat_id?,    cat_title);
impl_loggable!(Applicability,    EntityType::Applicability, app_id,     app_title);
impl_loggable!(NewApplicability, EntityType::Applicability, app_id?,    app_title);
impl_loggable!(Test,             EntityType::Test,          test_id,    test_name);
impl_loggable!(NewTest,          EntityType::Test,          test_id?,   test_name);
impl_loggable!(User,             EntityType::User,          user_id,    user_username, no_project);
impl_loggable!(NewUser,          EntityType::User,          user_id?,   user_username, no_project);

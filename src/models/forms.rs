//! Form structures for creating and updating database entities.
//!
//! These types are typically used with HTTP forms and represent the data
//! needed to insert or update records in the database.

use crate::logger::Loggable;
use crate::models::entities::EntityType;
use crate::schema::*;
use crate::status_enums::ProjectStatus;
use diesel::prelude::*;
use rocket::form::FromForm;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Data required to insert or update a requirement.
///
/// Typically populated from HTTP forms when creating or editing requirements.
#[derive(Serialize, Deserialize, Insertable, AsChangeset, FromForm)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = requirements)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(id))]
pub struct NewRequirement {
    pub id: Option<i32>,
    pub title: String,
    pub description: String,
    pub author_id: i32,
    pub category_id: i32,
    pub status_id: i32,
    pub parent_id: Option<i32>,
    pub reference_code: String,
    pub reviewer_id: i32,
    pub applicability_id: i32,
    pub justification: Option<String>,
    pub project_id: i32,
}

/// Macro to define tagged form entities with common structure.
macro_rules! define_tagged_form {
    ($name:ident, $table:ident) => {
        #[derive(Serialize, Deserialize, Insertable, FromForm, AsChangeset, Clone)]
        #[serde(crate = "rocket::serde")]
        #[diesel(table_name = $table)]
        #[diesel(check_for_backend(diesel::pg::Pg))]
        #[diesel(primary_key(id))]
        pub struct $name {
            pub id: Option<i32>,
            pub title: String,
            pub description: String,
            pub tag: String,
            pub project_id: i32,
        }
    };
}

define_tagged_form!(NewCategory, categories);
define_tagged_form!(NewApplicability, applicability);
define_tagged_form!(NewRequirementStatus, requirement_status);
define_tagged_form!(NewTestStatus, test_status);
define_tagged_form!(NewVerificationMethod, verification);

/// Form used to create a new [`MatrixLink`] entry tying a requirement to a test.
#[derive(Serialize, Deserialize, Insertable)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = matrix)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewMatrixLink {
    pub req_id: i32,
    pub test_id: i32,
    pub project_id: i32,
}

/// Form used to insert or update [`User`] records.
///
/// # Security Note
/// The `password_hash` field stores the bcrypt hash and is protected with
/// `#[serde(skip_serializing)]` to prevent accidental exposure in responses.
/// This struct is for internal use after password hashing. API endpoints should
/// use [`UserCreateRequest`] which accepts plain passwords.
#[derive(Serialize, Deserialize, Queryable, Insertable, AsChangeset, FromForm)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(id))]
pub struct NewUser {
    pub id: Option<i32>,
    pub username: String,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing, default)]
    pub password_hash: String,
    pub is_admin: bool,
}

/// Partial user information used when editing an existing user.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct UpdateUser {
    pub id: Option<i32>,
    pub username: String,
    pub name: String,
    pub email: String,
    pub is_admin: bool,
}

/// Form used to create or update a [`TestCase`].
#[derive(Serialize, Deserialize, Insertable, FromForm, AsChangeset)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = tests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewTestCase {
    pub id: Option<i32>,
    pub reference_code: String,
    pub name: String,
    pub description: String,
    pub source: String,
    pub status_id: i32,
    pub parent_id: Option<i32>,
    pub project_id: i32,
}

/// Form data submitted when creating a new test along with linked
/// requirements.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct NewTestForm {
    pub name: String,
    pub reference_code: String,
    pub description: String,
    pub source: String,
    pub status_id: i32,
    pub parent_id: Option<i32>,
    pub test_req: Vec<i32>,
    pub project_id: i32,
}

/// Form used for editing an existing test and updating its requirement links.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct EditTestForm {
    pub id: i32,
    pub reference_code: String,
    pub name: String,
    pub description: String,
    pub source: String,
    pub status_id: i32,
    pub parent_id: Option<i32>,
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

/// API request DTO for creating a new user with plain password.
///
/// Unlike [`NewUser`], this accepts a plain `password` field which is hashed
/// server-side before being stored. This is the secure way to handle user
/// creation via API endpoints.
#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UserCreateRequest {
    pub username: String,
    pub name: String,
    pub email: String,
    pub password: String,
    pub is_admin: bool,
}

/// Data required to create a new [`Project`].
#[derive(Insertable, Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::projects)]
pub struct NewProject {
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Option<i32>,
    #[serde(default)]
    #[field(default = ProjectStatus::Active)]
    pub status: ProjectStatus,
}

/// Form used to update a project's metadata.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct UpdateProject {
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Option<i32>,
    pub status: Option<ProjectStatus>,
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

/// Form that stores temporary column mappings for data imports.
#[derive(FromForm)]
pub struct ImportMappingForm {
    pub column_mappings: String,
    pub import_type: String,
    pub temp_file: String,
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

// Display implementations for form types
// These provide minimal, text-based representations suitable for logging
// and debugging. HTML rendering should be handled by templates.
impl fmt::Display for NewRequirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "New Requirement: {}", self.title)
    }
}

// Loggable implementations for form types
macro_rules! impl_loggable {
    // For types with `Option` id
    ($ty:ty, $entity:expr, $name:ident) => {
        impl Loggable for $ty {
            fn entity_type() -> EntityType {
                $entity
            }
            fn id(&self) -> i32 {
                self.id.unwrap_or_default()
            }
            fn project_id(&self) -> Option<i32> {
                Some(self.project_id)
            }
            fn display_name(&self) -> String {
                self.$name.clone()
            }
        }
    };

    ($ty:ty, $entity:expr, $name:ident, no_project) => {
        impl Loggable for $ty {
            fn entity_type() -> EntityType {
                $entity
            }
            fn id(&self) -> i32 {
                self.id.unwrap_or_default()
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

impl_loggable!(NewRequirement, EntityType::Requirement, title);
impl_loggable!(NewCategory, EntityType::Category, title);
impl_loggable!(NewApplicability, EntityType::Applicability, title);
impl_loggable!(NewVerificationMethod, EntityType::Verification, title);
impl_loggable!(NewTestCase, EntityType::Test, name);
impl_loggable!(NewUser, EntityType::User, username, no_project);
impl_loggable!(NewRequirementStatus, EntityType::Requirement, title);
impl_loggable!(NewTestStatus, EntityType::Test, title);

//! Form structures for creating and updating database entities.
//!
//! These types are typically used with HTTP forms and represent the data
//! needed to insert or update records in the database.

use crate::logger::Loggable;
use crate::models::entities::EntityType;
use crate::schema::*;
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
    pub verification_method_id: i32,
    pub author_id: i32,
    pub category_id: i32,
    pub current_status_id: i32,
    pub parent_id: i32,
    pub reference_code: String,
    pub reviewer_id: i32,
    pub applicability_id: i32,
    pub justification: Option<String>,
    pub project_id: i32,
}

/// Form used to insert or update a [`Category`].
#[derive(Serialize, Deserialize, Insertable, FromForm, AsChangeset, Clone)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = categories)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(id))]
pub struct NewCategory {
    pub id: Option<i32>,
    pub title: String,
    pub description: String,
    pub tag: String,
    pub project_id: i32,
}

/// Form used to insert or update an [`Applicability`].
#[derive(Serialize, Deserialize, Insertable, FromForm, AsChangeset, Clone)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = applicability)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(id))]
pub struct NewApplicability {
    pub id: Option<i32>,
    pub title: String,
    pub description: String,
    pub tag: String,
    pub project_id: i32,
}

/// Form used to create a new [`Status`].
#[derive(Serialize, Deserialize, Insertable, FromForm)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = requirement_status)]
pub struct NewStatus {
    pub title: String,
    pub description: String,
    pub short_name: String,
}

#[derive(Serialize, Deserialize, Insertable, AsChangeset, FromForm, Clone)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = verification)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(id))]
pub struct NewVerificationMethod {
    pub id: Option<i32>,
    pub name: String,
    pub description: String,
    pub project_id: i32,
}

/// Form used to create a new [`Matrix`] entry tying a requirement to a test.
#[derive(Serialize, Deserialize, Insertable)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = matrix)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewMatrix {
    pub req_id: i32,
    pub id: i32,
    pub project_id: i32,
}

/// Form used to insert or update [`User`] records.
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
    pub parent_id: i32,
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
    pub parent_id: i32,
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
    pub parent_id: i32,
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

/// Data required to create a new [`Project`].
#[derive(Insertable, Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::projects)]
pub struct NewProject {
    pub name: String,
    pub description: Option<String>,
    pub status_id: String,
    pub owner_id: Option<i32>,
}

/// Form used to update a project's metadata.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct UpdateProject {
    pub name: String,
    pub description: Option<String>,
    pub status_id: String,
    pub owner_id: Option<i32>,
}

/// Data required to create or update a project membership entry.
#[derive(Insertable, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::project_members)]
pub struct NewProjectMember {
    pub project_id: i32,
    pub id: i32,
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
    pub id: i32,
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
impl fmt::Display for NewRequirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "
        <div class='requirement'>
            <div class='ReqTitle'>Title: {}</div><div class='ReqDesc'>Description: {}</div>
            <div class='ReqAuthor'>Author: {}</div>
        </div>",
            self.title, self.description, self.author_id
        )
    }
}

// Loggable implementations for form types
macro_rules! impl_loggable {
    // For types with `Option` id
    ($ty:ty, $entity:expr, $id:ident?, $name:ident) => {
        impl Loggable for $ty {
            fn entity_type() -> EntityType {
                $entity
            }
            fn id(&self) -> i32 {
                self.$id.unwrap_or_default()
            }
            fn project_id(&self) -> Option<i32> {
                Some(self.project_id)
            }
            fn display_name(&self) -> String {
                self.$name.clone()
            }
        }
    };

    ($ty:ty, $entity:expr, $id:ident?, $name:ident, no_project) => {
        impl Loggable for $ty {
            fn entity_type() -> EntityType {
                $entity
            }
            fn id(&self) -> i32 {
                self.$id.unwrap_or_default()
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

impl_loggable!(NewRequirement, EntityType::Requirement, id?, title);
impl_loggable!(NewCategory, EntityType::Category, id?, title);
impl_loggable!(
    NewApplicability,
    EntityType::Applicability,
    id?,
    title
);
impl_loggable!(NewTestCase, EntityType::Test, id?, name);
impl_loggable!(
    NewUser,
    EntityType::User,
    id?,
    username,
    no_project
);

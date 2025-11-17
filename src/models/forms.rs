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
#[diesel(primary_key(req_id))]
pub struct NewRequirement {
    pub req_id: Option<i32>,
    pub req_title: String,
    pub req_description: String,
    pub req_verification: i32,
    pub req_author: i32,
    pub req_category: i32,
    pub req_current_status: i32,
    pub req_parent: i32,
    pub req_reference: String,
    pub req_reviewer: i32,
    pub req_applicability: i32,
    pub req_justification: Option<String>,
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

/// Form used to create a new [`Status`].
#[derive(Serialize, Deserialize, Insertable, FromForm)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = requirement_status)]
pub struct NewStatus {
    pub req_st_title: String,
    pub req_st_description: String,
    pub req_st_short_name: String,
}

#[derive(Serialize, Deserialize, Insertable, AsChangeset, FromForm, Clone)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = verification)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(verification_id))]
pub struct NewVerification {
    pub verification_id: Option<i32>,
    pub verification_name: String,
    pub verification_description: String,
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

/// Form used to create or update a [`Test`].
#[derive(Serialize, Deserialize, Insertable, FromForm, AsChangeset)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = tests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewTest {
    pub test_id: Option<i32>,
    pub test_reference: String,
    pub test_name: String,
    pub test_description: String,
    pub test_source: String,
    pub test_status: i32,
    pub test_parent: i32,
    pub project_id: i32,
}

/// Form data submitted when creating a new test along with linked
/// requirements.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct NewTestForm {
    pub test_name: String,
    pub test_reference: String,
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
    pub test_reference: String,
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

impl_loggable!(NewRequirement, EntityType::Requirement, req_id?, req_title);
impl_loggable!(NewCategory, EntityType::Category, cat_id?, cat_title);
impl_loggable!(
    NewApplicability,
    EntityType::Applicability,
    app_id?,
    app_title
);
impl_loggable!(NewTest, EntityType::Test, test_id?, test_name);
impl_loggable!(
    NewUser,
    EntityType::User,
    user_id?,
    user_username,
    no_project
);

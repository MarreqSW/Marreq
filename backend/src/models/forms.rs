// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

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

/// Data required to create or update a requirement (version content).
///
/// Used as API/form payload; converted to [`NewRequirementVersion`] when inserting a version.
#[derive(Serialize, Deserialize, FromForm, Clone, Debug)]
#[serde(crate = "rocket::serde")]
pub struct NewRequirement {
    pub id: Option<i32>,
    pub title: String,
    pub description: String,
    pub author_id: i32,
    pub category_id: i32,
    pub status_id: i32,
    pub reference_code: String,
    pub reviewer_id: i32,
    pub applicability_id: i32,
    pub justification: Option<String>,
    pub project_id: i32,
}

/// Insertable row for a new requirement version (immutable snapshot).
#[derive(Serialize, Deserialize, Insertable, Clone, Debug)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = requirement_versions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewRequirementVersion {
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
    /// New versions start as draft.
    pub approval_state: String,
}

/// Insertable row for the logical requirement container (no version content).
#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = requirements)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewRequirementContainer {
    pub project_id: i32,
    pub stable_code: String,
    pub current_version_id: Option<i32>,
}

impl NewRequirement {
    /// Build an insertable version row from this payload for the given logical requirement.
    pub fn to_new_version(&self, requirement_id: i32) -> NewRequirementVersion {
        NewRequirementVersion {
            requirement_id,
            title: self.title.clone(),
            description: self.description.clone(),
            status_id: self.status_id,
            author_id: self.author_id,
            reviewer_id: self.reviewer_id,
            category_id: self.category_id,
            applicability_id: self.applicability_id,
            justification: self.justification.clone(),
            deadline_date: None,
            approval_state: crate::status_enums::ApprovalState::Draft
                .to_db_string()
                .to_string(),
        }
    }
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

/// Insertable requirement status. `is_system` is set by the service (true for defaults, false for user-created).
#[derive(Serialize, Deserialize, Insertable, FromForm, AsChangeset, Clone)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = requirement_status)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(id))]
pub struct NewRequirementStatus {
    pub id: Option<i32>,
    pub title: String,
    pub description: String,
    pub tag: String,
    pub project_id: i32,
    #[serde(default)]
    pub is_system: bool,
    #[serde(default)]
    pub tag_color: Option<String>,
}

/// Insertable verification status. `is_system` is set by the service (true for defaults, false for user-created).
#[derive(Serialize, Deserialize, Insertable, FromForm, AsChangeset, Clone)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = verification_status)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(id))]
pub struct NewVerificationStatus {
    pub id: Option<i32>,
    pub title: String,
    pub description: String,
    pub tag: String,
    pub project_id: i32,
    #[serde(default)]
    pub is_system: bool,
    #[serde(default)]
    pub tag_color: Option<String>,
}

define_tagged_form!(NewVerificationMethod, verification_methods);

/// Payload to create or update a custom field definition (project-scoped).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "rocket::serde")]
pub struct CustomFieldDefinitionPayload {
    pub label: String,
    /// One of: text, enum, boolean, number
    pub field_type: String,
    /// Required for field_type "enum": array of allowed strings
    pub enum_values: Option<Vec<String>>,
    pub sort_order: Option<i32>,
}

/// Insertable row for custom_field_definitions (id and created_at are SERIAL/DEFAULT).
#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = custom_field_definitions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewCustomFieldDefinitionRow {
    pub project_id: i32,
    pub label: String,
    pub field_type: String,
    pub enum_values: Option<serde_json::Value>,
    pub sort_order: i32,
}

/// One custom field value when creating/updating a requirement (field_id, value).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "rocket::serde")]
pub struct CustomFieldValueInput {
    pub field_id: i32,
    pub value: Option<String>,
}

/// Form used to create a new [`MatrixLink`] entry tying a requirement to a verification.
#[derive(Serialize, Deserialize, Insertable)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = matrix)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewMatrixLink {
    pub req_id: i32,
    pub verification_id: i32,
    pub project_id: i32,
    pub triggering_version_id: Option<i32>,
    pub triggering_user_id: Option<i32>,
}

/// Payload to create an immutable baseline (name, description). created_at/created_by set at insert.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "rocket::serde")]
pub struct NewBaseline {
    pub name: String,
    pub description: Option<String>,
}

/// Insertable row for baselines table (id is SERIAL).
#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = baselines)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewBaselineRow {
    pub project_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub created_by: i32,
}

/// Insertable row for baseline_requirements.
#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = baseline_requirements)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewBaselineRequirement {
    pub baseline_id: i32,
    pub requirement_id: i32,
    pub version_id: i32,
}

/// Insertable row for baseline_traceability.
#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = baseline_traceability)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewBaselineTraceability {
    pub baseline_id: i32,
    pub requirement_id: i32,
    pub verification_id: i32,
    pub suspect: bool,
    pub suspect_at: Option<chrono::NaiveDateTime>,
    pub suspect_reason: Option<String>,
}

/// Insertable row for baseline_verifications.
#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = baseline_verifications)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewBaselineVerification {
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

/// Form used to insert or update [`User`] records.
///
/// # Security Note
/// The `password_hash` field stores the password hash and is protected with
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

/// Form used to create or update a [`Verification`].
#[derive(Serialize, Deserialize, Insertable, FromForm, AsChangeset)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = verifications)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewVerification {
    pub id: Option<i32>,
    pub reference_code: String,
    pub name: String,
    pub description: String,
    pub source: String,
    pub status_id: i32,
    pub parent_id: Option<i32>,
    pub project_id: i32,
    pub verification_method_id: Option<i32>,
    pub author_id: i32,
    pub reviewer_id: i32,
}

/// Form data submitted when creating a new verification along with linked
/// requirements.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct NewVerificationForm {
    pub name: String,
    pub reference_code: String,
    pub description: String,
    pub source: String,
    pub status_id: i32,
    pub parent_id: Option<i32>,
    pub verification_method_id: Option<i32>,
    pub verification_req: Vec<i32>,
    pub project_id: i32,
}

/// Form used for editing an existing verification and updating its requirement links.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct EditVerificationForm {
    pub id: i32,
    pub reference_code: String,
    pub name: String,
    pub description: String,
    pub source: String,
    pub status_id: i32,
    pub parent_id: Option<i32>,
    pub verification_method_id: Option<i32>,
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

/// Form used when an admin sets another user's password (no current password).
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct AdminSetPasswordForm {
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

/// Data required to create a new [`Group`].
#[derive(Serialize, Deserialize, FromForm, Clone, Debug)]
#[serde(crate = "rocket::serde")]
pub struct NewGroup {
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Option<i32>,
}

/// Internal insertable row for creating a group with a persisted slug.
#[derive(Insertable, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::groups)]
pub struct NewGroupRow {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub owner_id: Option<i32>,
}

/// Form used to update a group's metadata.
#[derive(Serialize, Deserialize, FromForm, Clone, Debug)]
#[serde(crate = "rocket::serde")]
pub struct UpdateGroup {
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Option<i32>,
}

/// Data required to create or update a group membership entry.
#[derive(Insertable, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::group_members)]
pub struct NewGroupMember {
    pub group_id: i32,
    pub user_id: i32,
    pub role: i32,
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
    pub group_id: Option<i32>,
}

/// Internal insertable row for creating a project with a persisted slug.
#[derive(Insertable, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::projects)]
pub struct NewProjectRow {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub owner_id: Option<i32>,
    pub status: ProjectStatus,
    pub group_id: Option<i32>,
}

/// Form used to update a project's metadata.
#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct UpdateProject {
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Option<i32>,
    pub status: Option<ProjectStatus>,
    pub slug: Option<String>,
    pub group_id: Option<i32>,
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
impl_loggable!(NewVerificationMethod, EntityType::VerificationMethod, title);
impl_loggable!(NewVerification, EntityType::Verification, name);
impl_loggable!(NewUser, EntityType::User, username, no_project);
impl_loggable!(NewRequirementStatus, EntityType::Requirement, title);
impl_loggable!(NewVerificationStatus, EntityType::Verification, title);

/// Payload for creating a notification.
#[derive(Insertable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::schema::notifications)]
pub struct NewNotification {
    pub user_id: i32,
    pub project_id: Option<i32>,
    pub notification_type: String,
    pub title: String,
    pub body: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<i32>,
    pub actor_id: Option<i32>,
}

/// Payload for creating or updating a notification preference.
#[derive(Insertable, AsChangeset, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = crate::schema::notification_preferences)]
pub struct NewNotificationPreference {
    pub user_id: i32,
    pub project_id: i32,
    pub notify_in_app: bool,
    pub notify_email: bool,
}

#[cfg(test)]
mod forms_tests {
    use super::*;
    use crate::models::entities::EntityType;

    #[test]
    fn new_requirement_display() {
        let req = NewRequirement {
            id: Some(1),
            title: "Safety requirement".into(),
            description: "Desc".into(),
            author_id: 1,
            category_id: 1,
            status_id: 1,
            reference_code: "REQ-001".into(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: None,
            project_id: 10,
        };
        assert_eq!(req.to_string(), "New Requirement: Safety requirement");
    }

    #[test]
    fn new_requirement_loggable() {
        let req = NewRequirement {
            id: Some(5),
            title: "Title".into(),
            description: "D".into(),
            author_id: 1,
            category_id: 1,
            status_id: 1,
            reference_code: "R".into(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: None,
            project_id: 7,
        };
        assert_eq!(NewRequirement::entity_type(), EntityType::Requirement);
        assert_eq!(req.id(), 5);
        assert_eq!(req.project_id(), Some(7));
        assert_eq!(req.display_name(), "Title");
    }

    #[test]
    fn new_category_loggable() {
        let cat = NewCategory {
            id: None,
            title: "Systems".into(),
            description: "D".into(),
            tag: "systems".into(),
            project_id: 1,
        };
        assert_eq!(NewCategory::entity_type(), EntityType::Category);
        assert_eq!(cat.id(), 0);
        assert_eq!(cat.project_id(), Some(1));
        assert_eq!(cat.display_name(), "Systems");
    }

    #[test]
    fn new_user_loggable_no_project() {
        let u = NewUser {
            id: Some(2),
            username: "alice".into(),
            name: "Alice".into(),
            email: "a@b.com".into(),
            password_hash: "hash".into(),
            is_admin: false,
        };
        assert_eq!(NewUser::entity_type(), EntityType::User);
        assert_eq!(u.id(), 2);
        assert_eq!(u.project_id(), None);
        assert_eq!(u.display_name(), "alice");
    }

    #[test]
    fn new_matrix_link_fields() {
        let link = NewMatrixLink {
            req_id: 1,
            verification_id: 2,
            project_id: 10,
            triggering_version_id: None,
            triggering_user_id: None,
        };
        assert_eq!(link.req_id, 1);
        assert_eq!(link.verification_id, 2);
        assert_eq!(link.project_id, 10);
    }

    #[test]
    fn login_form_fields() {
        let form = LoginForm {
            username: "user".into(),
            password: "secret".into(),
        };
        assert_eq!(form.username, "user");
        assert_eq!(form.password, "secret");
    }

    #[test]
    fn change_password_form_fields() {
        let form = ChangePasswordForm {
            current_password: "old".into(),
            new_password: "new".into(),
            confirm_password: "new".into(),
        };
        assert_eq!(form.current_password, "old");
        assert_eq!(form.new_password, "new");
    }

    #[test]
    fn user_create_request_has_plain_password() {
        let req = UserCreateRequest {
            username: "u".into(),
            name: "N".into(),
            email: "e@e.com".into(),
            password: "plain".into(),
            is_admin: false,
        };
        assert_eq!(req.password, "plain");
        assert!(!req.is_admin);
    }

    #[test]
    fn new_project_serialization_roundtrip() {
        let project = NewProject {
            name: "Proj".into(),
            description: Some("Desc".into()),
            owner_id: Some(1),
            status: ProjectStatus::Active,
            group_id: None,
        };
        let json = serde_json::to_string(&project).unwrap();
        let parsed: NewProject = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, project.name);
        assert_eq!(parsed.description, project.description);
    }

    #[test]
    fn update_project_optional_status() {
        let upd = UpdateProject {
            name: "N".into(),
            description: None,
            owner_id: None,
            status: Some(ProjectStatus::Completed),
            slug: None,
            group_id: None,
        };
        assert_eq!(upd.name, "N");
        assert_eq!(upd.status, Some(ProjectStatus::Completed));
    }

    #[test]
    fn new_project_member_debug_clone() {
        let m = NewProjectMember {
            project_id: 1,
            user_id: 2,
            role: 3,
        };
        let m2 = m.clone();
        assert_eq!(m2.project_id, m.project_id);
        assert_eq!(format!("{:?}", m), format!("{:?}", m2));
    }

    #[test]
    fn new_test_form_fields() {
        let form = NewVerificationForm {
            name: "T1".into(),
            reference_code: "T-001".into(),
            description: "D".into(),
            source: "Spec".into(),
            status_id: 1,
            parent_id: None,
            verification_method_id: None,
            verification_req: vec![1, 2],
            project_id: 10,
        };
        assert_eq!(form.verification_req.len(), 2);
        assert_eq!(form.project_id, 10);
    }

    #[test]
    fn edit_test_form_linked_requirements() {
        let form = EditVerificationForm {
            id: 5,
            reference_code: "T-005".into(),
            name: "Test".into(),
            description: "D".into(),
            source: "S".into(),
            status_id: 1,
            parent_id: None,
            verification_method_id: None,
            linked_requirements: vec![10, 20],
            project_id: 1,
        };
        assert_eq!(form.id, 5);
        assert_eq!(form.linked_requirements, vec![10, 20]);
    }

    #[test]
    fn new_requirement_loggable_id_none() {
        let req = NewRequirement {
            id: None,
            title: "T".into(),
            description: "D".into(),
            author_id: 1,
            category_id: 1,
            status_id: 1,
            reference_code: "R".into(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: None,
            project_id: 2,
        };
        assert_eq!(req.id(), 0);
        assert_eq!(req.display_name(), "T");
    }

    #[test]
    fn new_applicability_loggable() {
        let a = NewApplicability {
            id: Some(3),
            title: "App Title".into(),
            description: "D".into(),
            tag: "tag".into(),
            project_id: 1,
        };
        assert_eq!(NewApplicability::entity_type(), EntityType::Applicability);
        assert_eq!(a.id(), 3);
        assert_eq!(a.project_id(), Some(1));
        assert_eq!(a.display_name(), "App Title");
    }

    #[test]
    fn new_verification_method_loggable() {
        let v = NewVerificationMethod {
            id: None,
            title: "Verification".into(),
            description: "D".into(),
            tag: "V".into(),
            project_id: 1,
        };
        assert_eq!(
            NewVerificationMethod::entity_type(),
            EntityType::VerificationMethod
        );
        assert_eq!(v.id(), 0);
        assert_eq!(v.display_name(), "Verification");
    }

    #[test]
    fn new_requirement_status_loggable() {
        let s = NewRequirementStatus {
            id: Some(1),
            title: "Draft".into(),
            description: "D".into(),
            tag: "D".into(),
            project_id: 1,
            is_system: false,
            tag_color: None,
        };
        assert_eq!(NewRequirementStatus::entity_type(), EntityType::Requirement);
        assert_eq!(s.display_name(), "Draft");
    }

    #[test]
    fn new_verification_status_loggable() {
        let s = NewVerificationStatus {
            id: Some(1),
            title: "Pass".into(),
            description: "D".into(),
            tag: "P".into(),
            project_id: 1,
            is_system: false,
            tag_color: None,
        };
        assert_eq!(
            NewVerificationStatus::entity_type(),
            EntityType::Verification
        );
        assert_eq!(s.display_name(), "Pass");
    }

    #[test]
    fn new_verification_loggable() {
        let t = NewVerification {
            id: Some(7),
            reference_code: "V-007".into(),
            name: "Verification".into(),
            description: "D".into(),
            source: "S".into(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
            verification_method_id: None,
            author_id: 1,
            reviewer_id: 1,
        };
        assert_eq!(NewVerification::entity_type(), EntityType::Verification);
        assert_eq!(t.id(), 7);
        assert_eq!(t.project_id(), Some(1));
        assert_eq!(t.display_name(), "Verification");
    }

    #[test]
    fn update_user_fields() {
        let u = UpdateUser {
            id: Some(10),
            username: "u".into(),
            name: "Name".into(),
            email: "e@e.com".into(),
            is_admin: true,
        };
        assert_eq!(u.id, Some(10));
        assert_eq!(u.username, "u");
        assert!(u.is_admin);
    }

    #[test]
    fn new_log_fields() {
        let log = NewLog {
            user_id: 1,
            action_type: "CREATE".into(),
            entity_type: "Requirement".into(),
            entity_id: Some(5),
            project_id: Some(10),
            old_values: None,
            new_values: Some("{}".into()),
            description: Some("Created".into()),
            ip_address: Some("127.0.0.1".into()),
            user_agent: None,
        };
        assert_eq!(log.user_id, 1);
        assert_eq!(log.entity_id, Some(5));
        assert_eq!(log.new_values.as_deref(), Some("{}"));
    }
}

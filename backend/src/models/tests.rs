//! Comprehensive test suite for the models module.
//!
//! This module provides tests for all model types, ensuring:
//! - Display implementations work correctly
//! - Loggable trait implementations return correct values
//! - Enum methods (ActionType, EntityType) work as expected
//! - Serialization/deserialization works
//! - All struct fields are accessible

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use super::super::*;
    use crate::logger::Loggable;
    use crate::status_enums::ProjectStatus;
    use chrono::{NaiveDate, NaiveDateTime};

    // Helper function to create a test timestamp
    fn test_timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 15)
            .unwrap()
            .and_hms_opt(10, 30, 0)
            .unwrap()
    }

    // ============================================================================
    // Tests for entities.rs
    // ============================================================================

    mod entities_tests {
        use super::*;

        #[test]
        fn requirement_display() {
            let req = Requirement {
                id: 42,
                current_version_id: None,
                same_as_current: None,
                title: "Test Requirement".to_string(),
                description: "Test Description".to_string(),
                status_id: 2,
                author_id: 10,
                reviewer_id: 11,
                reference_code: "REQ-001".to_string(),
                category_id: 5,
                parent_id: None,
                creation_date: test_timestamp(),
                update_date: test_timestamp(),
                deadline_date: Some(test_timestamp()),
                applicability_id: 3,
                justification: Some("Important".to_string()),
                project_id: 1,
                approval_state: "draft".to_string(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            };

            let display = format!("{}", req);
            assert_eq!(display, "Requirement #42: Test Requirement");
        }

        #[test]
        fn requirement_display_with_parent() {
            let req = Requirement {
                id: 100,
                current_version_id: None,
                same_as_current: None,
                title: "Child Requirement".to_string(),
                description: "Child".to_string(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REQ-100".to_string(),
                category_id: 1,
                parent_id: Some(50),
                creation_date: test_timestamp(),
                update_date: test_timestamp(),
                deadline_date: None,
                applicability_id: 1,
                justification: None,
                project_id: 1,
                approval_state: "draft".to_string(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            };

            let display = format!("{}", req);
            assert_eq!(display, "Requirement #100: Child Requirement");
        }

        #[test]
        fn category_display() {
            let cat = Category {
                id: 1,
                title: "Security".to_string(),
                description: "Security requirements".to_string(),
                tag: "SEC".to_string(),
                project_id: 1,
            };

            let display = format!("{}", cat);
            assert_eq!(display, "Category: Security");
        }

        #[test]
        fn applicability_display() {
            let app = Applicability {
                id: 2,
                title: "All Systems".to_string(),
                description: "Applies to all systems".to_string(),
                tag: "ALL".to_string(),
                project_id: 1,
            };

            let display = format!("{}", app);
            assert_eq!(display, "Applicability: All Systems");
        }

        #[test]
        fn requirement_status_display() {
            let status = RequirementStatus {
                id: 3,
                title: "Accepted".to_string(),
                description: "Requirement accepted".to_string(),
                tag: "ACC".to_string(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            };

            let display = format!("{}", status);
            assert_eq!(display, "Status: Accepted");
        }

        #[test]
        fn matrix_link_display() {
            let link = MatrixLink {
                req_id: 10,
                verification_id: 20,
                creation_date: test_timestamp(),
                project_id: 1,
                suspect: false,
                suspect_at: None,
                suspect_reason: None,
                cleared_by: None,
                cleared_at: None,
                triggering_version_id: None,
                triggering_user_id: None,
            };

            let display = format!("{}", link);
            assert_eq!(display, "Matrix: Req 10 <-> Verification 20");
        }

        #[test]
        fn verification_display() {
            let ver = Verification {
                id: 5,
                name: "Unit Verification".to_string(),
                reference_code: "VER-005".to_string(),
                description: "Verification description".to_string(),
                source: "ver.rs".to_string(),
                status_id: 1,
                parent_id: None,
                project_id: 1,
                verification_method_id: None,
                author_id: 1,
                reviewer_id: 1,
                status_set_by: None,
                status_set_at: None,
            };

            let display = format!("{}", ver);
            assert_eq!(display, "Verification #5: Unit Verification");
        }

        #[test]
        fn verification_display_with_parent() {
            let ver = Verification {
                id: 15,
                name: "Child Verification".to_string(),
                reference_code: "VER-015".to_string(),
                description: "Child".to_string(),
                source: "child.rs".to_string(),
                status_id: 1,
                parent_id: Some(10),
                project_id: 1,
                verification_method_id: None,
                author_id: 1,
                reviewer_id: 1,
                status_set_by: None,
                status_set_at: None,
            };

            let display = format!("{}", ver);
            assert_eq!(display, "Verification #15: Child Verification");
        }

        // ActionType tests
        #[test]
        fn action_type_display() {
            assert_eq!(format!("{}", ActionType::Create), "CREATE");
            assert_eq!(format!("{}", ActionType::Update), "UPDATE");
            assert_eq!(format!("{}", ActionType::Delete), "DELETE");
            assert_eq!(format!("{}", ActionType::Login), "LOGIN");
            assert_eq!(format!("{}", ActionType::Logout), "LOGOUT");
            assert_eq!(format!("{}", ActionType::Export), "EXPORT");
            assert_eq!(format!("{}", ActionType::Import), "IMPORT");
            assert_eq!(format!("{}", ActionType::StatusChange), "STATUS_CHANGE");
        }

        #[test]
        fn action_type_past_tense() {
            assert_eq!(ActionType::Create.past_tense(), "Created");
            assert_eq!(ActionType::Update.past_tense(), "Updated");
            assert_eq!(ActionType::Delete.past_tense(), "Deleted");
            assert_eq!(ActionType::Login.past_tense(), "Logged in");
            assert_eq!(ActionType::Logout.past_tense(), "Logged out");
            assert_eq!(ActionType::Export.past_tense(), "Exported");
            assert_eq!(ActionType::Import.past_tense(), "Imported");
            assert_eq!(ActionType::StatusChange.past_tense(), "Changed status");
        }

        #[test]
        fn action_type_equality() {
            assert_eq!(ActionType::Create, ActionType::Create);
            assert_ne!(ActionType::Create, ActionType::Update);
        }

        // EntityType tests
        #[test]
        fn entity_type_display() {
            assert_eq!(format!("{}", EntityType::Project), "PROJECT");
            assert_eq!(format!("{}", EntityType::Requirement), "REQUIREMENT");
            assert_eq!(format!("{}", EntityType::Category), "CATEGORY");
            assert_eq!(format!("{}", EntityType::Applicability), "APPLICABILITY");
            assert_eq!(format!("{}", EntityType::User), "USER");
            assert_eq!(format!("{}", EntityType::MatrixLink), "MATRIX");
            assert_eq!(format!("{}", EntityType::Verification), "VERIFICATION");
            assert_eq!(
                format!("{}", EntityType::VerificationMethod),
                "VERIFICATION_METHOD"
            );
        }

        #[test]
        fn entity_type_human_name() {
            assert_eq!(EntityType::Project.human_name(), "project");
            assert_eq!(EntityType::Requirement.human_name(), "requirement");
            assert_eq!(EntityType::Category.human_name(), "category");
            assert_eq!(EntityType::Applicability.human_name(), "applicability");
            assert_eq!(EntityType::User.human_name(), "user");
            assert_eq!(EntityType::MatrixLink.human_name(), "matrix");
            assert_eq!(EntityType::Verification.human_name(), "verification");
            assert_eq!(
                EntityType::VerificationMethod.human_name(),
                "verification method"
            );
        }

        // Loggable trait tests for entities
        #[test]
        fn requirement_loggable() {
            let req = Requirement {
                id: 42,
                current_version_id: None,
                same_as_current: None,
                title: "Test Req".to_string(),
                description: "Desc".to_string(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REQ-001".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: test_timestamp(),
                update_date: test_timestamp(),
                deadline_date: None,
                applicability_id: 1,
                justification: None,
                project_id: 5,
                approval_state: "draft".to_string(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            };

            assert_eq!(Requirement::entity_type(), EntityType::Requirement);
            assert_eq!(req.id(), 42);
            assert_eq!(req.project_id(), Some(5));
            assert_eq!(req.display_name(), "Test Req");
        }

        #[test]
        fn category_loggable() {
            let cat = Category {
                id: 10,
                title: "Security".to_string(),
                description: "Desc".to_string(),
                tag: "SEC".to_string(),
                project_id: 3,
            };

            assert_eq!(Category::entity_type(), EntityType::Category);
            assert_eq!(cat.id(), 10);
            assert_eq!(cat.project_id(), Some(3));
            assert_eq!(cat.display_name(), "Security");
        }

        #[test]
        fn applicability_loggable() {
            let app = Applicability {
                id: 20,
                title: "All Systems".to_string(),
                description: "Desc".to_string(),
                tag: "ALL".to_string(),
                project_id: 2,
            };

            assert_eq!(Applicability::entity_type(), EntityType::Applicability);
            assert_eq!(app.id(), 20);
            assert_eq!(app.project_id(), Some(2));
            assert_eq!(app.display_name(), "All Systems");
        }

        #[test]
        fn verification_loggable() {
            let ver = Verification {
                id: 30,
                name: "Unit Verification".to_string(),
                reference_code: "VER-001".to_string(),
                description: "Desc".to_string(),
                source: "ver.rs".to_string(),
                status_id: 1,
                parent_id: None,
                project_id: 4,
                verification_method_id: None,
                author_id: 1,
                reviewer_id: 1,
                status_set_by: None,
                status_set_at: None,
            };

            assert_eq!(Verification::entity_type(), EntityType::Verification);
            assert_eq!(ver.id(), 30);
            assert_eq!(ver.project_id(), Some(4));
            assert_eq!(ver.display_name(), "Unit Verification");
        }

        #[test]
        fn user_loggable() {
            let user = User {
                id: 1,
                username: "testuser".to_string(),
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                creation_date: test_timestamp(),
                last_login: test_timestamp(),
                password_hash: "hash".to_string(),
                is_admin: false,
            };

            assert_eq!(User::entity_type(), EntityType::User);
            assert_eq!(user.id(), 1);
            assert_eq!(user.project_id(), None); // User has no project_id
            assert_eq!(user.display_name(), "testuser");
        }

        #[test]
        fn project_loggable() {
            let project = Project {
                id: 7,
                name: "Test Project".to_string(),
                description: Some("Description".to_string()),
                creation_date: Some(test_timestamp()),
                update_date: Some(test_timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(1),
                slug: "test-project".to_string(),
                group_id: None,
            };

            assert_eq!(Project::entity_type(), EntityType::Project);
            assert_eq!(project.id(), 7);
            assert_eq!(project.project_id(), Some(7)); // Uses id field
            assert_eq!(project.display_name(), "Test Project");
        }

        // Test struct field access
        #[test]
        fn requirement_field_access() {
            let req = Requirement {
                id: 1,
                current_version_id: None,
                same_as_current: None,
                title: "Title".to_string(),
                description: "Desc".to_string(),
                status_id: 3,
                author_id: 4,
                reviewer_id: 5,
                reference_code: "REF".to_string(),
                category_id: 6,
                parent_id: Some(7),
                creation_date: test_timestamp(),
                update_date: test_timestamp(),
                deadline_date: Some(test_timestamp()),
                applicability_id: 8,
                justification: Some("Just".to_string()),
                project_id: 9,
                approval_state: "draft".to_string(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            };

            assert_eq!(req.id, 1);
            assert_eq!(req.title, "Title");
            assert_eq!(req.status_id, 3);
            assert_eq!(req.parent_id, Some(7));
            assert_eq!(req.justification, Some("Just".to_string()));
        }

        #[test]
        fn user_field_access() {
            let user = User {
                id: 1,
                username: "user".to_string(),
                name: "Name".to_string(),
                email: "email@test.com".to_string(),
                creation_date: test_timestamp(),
                last_login: test_timestamp(),
                password_hash: "hash123".to_string(),
                is_admin: true,
            };

            assert_eq!(user.id, 1);
            assert_eq!(user.username, "user");
            assert!(user.is_admin);
        }

        #[test]
        fn project_field_access() {
            let project = Project {
                id: 1,
                name: "Project".to_string(),
                description: Some("Desc".to_string()),
                creation_date: Some(test_timestamp()),
                update_date: Some(test_timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(10),
                slug: "project".to_string(),
                group_id: None,
            };

            assert_eq!(project.id, 1);
            assert_eq!(project.name, "Project");
            assert_eq!(project.status, ProjectStatus::Active);
            assert_eq!(project.owner_id, Some(10));
        }

        #[test]
        fn matrix_link_field_access() {
            let link = MatrixLink {
                req_id: 1,
                verification_id: 2,
                creation_date: test_timestamp(),
                project_id: 3,
                suspect: false,
                suspect_at: None,
                suspect_reason: None,
                cleared_by: None,
                cleared_at: None,
                triggering_version_id: None,
                triggering_user_id: None,
            };

            assert_eq!(link.req_id, 1);
            assert_eq!(link.verification_id, 2);
            assert_eq!(link.project_id, 3);
        }

        #[test]
        fn project_member_field_access() {
            let member = ProjectMember {
                project_id: 1,
                user_id: 2,
                role: 3,
                created_at: test_timestamp(),
                updated_at: test_timestamp(),
            };

            assert_eq!(member.project_id, 1);
            assert_eq!(member.user_id, 2);
            assert_eq!(member.role, 3);
        }

        #[test]
        fn log_field_access() {
            let log = Log {
                log_id: 1,
                user_id: 2,
                action_type: "CREATE".to_string(),
                entity_type: "REQUIREMENT".to_string(),
                entity_id: Some(3),
                project_id: Some(4),
                old_values: Some("old".to_string()),
                new_values: Some("new".to_string()),
                description: Some("Desc".to_string()),
                ip_address: Some("127.0.0.1".to_string()),
                user_agent: Some("Mozilla".to_string()),
                created_at: test_timestamp(),
            };

            assert_eq!(log.log_id, 1);
            assert_eq!(log.user_id, 2);
            assert_eq!(log.action_type, "CREATE");
            assert_eq!(log.entity_id, Some(3));
        }

        // Test serialization/deserialization
        #[test]
        fn requirement_serialization() {
            let req = Requirement {
                id: 1,
                current_version_id: None,
                same_as_current: None,
                title: "Test".to_string(),
                description: "Desc".to_string(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: test_timestamp(),
                update_date: test_timestamp(),
                deadline_date: None,
                applicability_id: 1,
                justification: None,
                project_id: 1,
                approval_state: "draft".to_string(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            };

            let json = serde_json::to_string(&req).unwrap();
            let deserialized: Requirement = serde_json::from_str(&json).unwrap();

            assert_eq!(req.id, deserialized.id);
            assert_eq!(req.title, deserialized.title);
        }

        #[test]
        fn user_serialization_skips_password() {
            let user = User {
                id: 1,
                username: "user".to_string(),
                name: "Name".to_string(),
                email: "email@test.com".to_string(),
                creation_date: test_timestamp(),
                last_login: test_timestamp(),
                password_hash: "secret_hash".to_string(),
                is_admin: false,
            };

            let json = serde_json::to_string(&user).unwrap();
            // Password hash should not be in serialized output
            assert!(!json.contains("secret_hash"));
            assert!(!json.contains("password_hash"));
        }

        #[test]
        fn project_serialization() {
            let project = Project {
                id: 1,
                name: "Project".to_string(),
                description: Some("Desc".to_string()),
                creation_date: Some(test_timestamp()),
                update_date: Some(test_timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(1),
                slug: "project".to_string(),
                group_id: None,
            };

            let json = serde_json::to_string(&project).unwrap();
            let deserialized: Project = serde_json::from_str(&json).unwrap();

            assert_eq!(project.id, deserialized.id);
            assert_eq!(project.name, deserialized.name);
            assert_eq!(project.status, deserialized.status);
        }
    }

    // ============================================================================
    // Tests for forms.rs
    // ============================================================================

    mod forms_tests {
        use super::*;

        #[test]
        fn new_requirement_display() {
            let new_req = NewRequirement {
                id: Some(1),
                title: "New Requirement".to_string(),
                description: "Description".to_string(),
                author_id: 1,
                category_id: 1,
                status_id: 1,
                reference_code: "REF-001".to_string(),
                reviewer_id: 1,
                applicability_id: 1,
                justification: None,
                project_id: 1,
            };

            let display = format!("{}", new_req);
            assert_eq!(display, "New Requirement: New Requirement");
        }

        #[test]
        fn new_requirement_loggable() {
            let new_req = NewRequirement {
                id: Some(42),
                title: "Test Req".to_string(),
                description: "Desc".to_string(),
                author_id: 1,
                category_id: 1,
                status_id: 1,
                reference_code: "REF".to_string(),
                reviewer_id: 1,
                applicability_id: 1,
                justification: None,
                project_id: 5,
            };

            assert_eq!(NewRequirement::entity_type(), EntityType::Requirement);
            assert_eq!(new_req.id(), 42);
            assert_eq!(new_req.project_id(), Some(5));
            assert_eq!(new_req.display_name(), "Test Req");
        }

        #[test]
        fn new_requirement_loggable_without_id() {
            let new_req = NewRequirement {
                id: None,
                title: "New Req".to_string(),
                description: "Desc".to_string(),
                author_id: 1,
                category_id: 1,
                status_id: 1,
                reference_code: "REF".to_string(),
                reviewer_id: 1,
                applicability_id: 1,
                justification: None,
                project_id: 1,
            };

            assert_eq!(new_req.id(), 0); // None becomes 0
            assert_eq!(new_req.project_id(), Some(1));
        }

        #[test]
        fn new_category_loggable() {
            let new_cat = NewCategory {
                id: Some(10),
                title: "Security".to_string(),
                description: "Desc".to_string(),
                tag: "SEC".to_string(),
                project_id: 2,
            };

            assert_eq!(NewCategory::entity_type(), EntityType::Category);
            assert_eq!(new_cat.id(), 10);
            assert_eq!(new_cat.project_id(), Some(2));
            assert_eq!(new_cat.display_name(), "Security");
        }

        #[test]
        fn new_applicability_loggable() {
            let new_app = NewApplicability {
                id: Some(20),
                title: "All Systems".to_string(),
                description: "Desc".to_string(),
                tag: "ALL".to_string(),
                project_id: 3,
            };

            assert_eq!(NewApplicability::entity_type(), EntityType::Applicability);
            assert_eq!(new_app.id(), 20);
            assert_eq!(new_app.project_id(), Some(3));
            assert_eq!(new_app.display_name(), "All Systems");
        }

        #[test]
        fn new_verification_loggable() {
            let new_ver = NewVerification {
                id: Some(30),
                reference_code: "VER-001".to_string(),
                name: "Unit Verification".to_string(),
                description: "Desc".to_string(),
                source: "ver.rs".to_string(),
                status_id: 1,
                parent_id: None,
                project_id: 4,
                verification_method_id: None,
                author_id: 1,
                reviewer_id: 1,
            };

            assert_eq!(NewVerification::entity_type(), EntityType::Verification);
            assert_eq!(new_ver.id(), 30);
            assert_eq!(new_ver.project_id(), Some(4));
            assert_eq!(new_ver.display_name(), "Unit Verification");
        }

        #[test]
        fn new_user_loggable() {
            let new_user = NewUser {
                id: Some(1),
                username: "testuser".to_string(),
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                password_hash: "hash".to_string(),
                is_admin: false,
            };

            assert_eq!(NewUser::entity_type(), EntityType::User);
            assert_eq!(new_user.id(), 1);
            assert_eq!(new_user.project_id(), None); // User has no project_id
            assert_eq!(new_user.display_name(), "testuser");
        }

        #[test]
        fn new_verification_method_loggable() {
            let new_ver = NewVerificationMethod {
                id: Some(1),
                title: "Test".to_string(),
                description: "Desc".to_string(),
                tag: "TEST".to_string(),
                project_id: 1,
            };

            assert_eq!(
                NewVerificationMethod::entity_type(),
                EntityType::VerificationMethod
            );
            assert_eq!(new_ver.id(), 1);
            assert_eq!(new_ver.project_id(), Some(1));
            assert_eq!(new_ver.display_name(), "Test");
        }

        #[test]
        fn new_requirement_status_loggable() {
            let new_status = NewRequirementStatus {
                id: Some(1),
                title: "Accepted".to_string(),
                description: "Desc".to_string(),
                tag: "ACC".to_string(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            };

            assert_eq!(NewRequirementStatus::entity_type(), EntityType::Requirement);
            assert_eq!(new_status.id(), 1);
            assert_eq!(new_status.project_id(), Some(1));
            assert_eq!(new_status.display_name(), "Accepted");
        }

        #[test]
        fn new_verification_status_loggable() {
            let new_status = NewVerificationStatus {
                id: Some(1),
                title: "Passed".to_string(),
                description: "Desc".to_string(),
                tag: "PASS".to_string(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            };

            assert_eq!(
                NewVerificationStatus::entity_type(),
                EntityType::Verification
            );
            assert_eq!(new_status.id(), 1);
            assert_eq!(new_status.project_id(), Some(1));
            assert_eq!(new_status.display_name(), "Passed");
        }

        #[test]
        fn new_matrix_link_field_access() {
            let new_link = NewMatrixLink {
                req_id: 1,
                verification_id: 2,
                project_id: 3,
                triggering_version_id: None,
                triggering_user_id: None,
            };

            assert_eq!(new_link.req_id, 1);
            assert_eq!(new_link.verification_id, 2);
            assert_eq!(new_link.project_id, 3);
        }

        #[test]
        fn new_project_field_access() {
            let new_project = NewProject {
                name: "Project".to_string(),
                description: Some("Desc".to_string()),
                owner_id: Some(1),
                status: ProjectStatus::Active,
                group_id: None,
            };

            assert_eq!(new_project.name, "Project");
            assert_eq!(new_project.owner_id, Some(1));
            assert_eq!(new_project.status, ProjectStatus::Active);
        }

        #[test]
        fn update_project_field_access() {
            let update = UpdateProject {
                name: "Updated".to_string(),
                description: Some("New Desc".to_string()),
                owner_id: Some(2),
                status: Some(ProjectStatus::Cancelled),
                slug: None,
                group_id: None,
            };

            assert_eq!(update.name, "Updated");
            assert_eq!(update.status, Some(ProjectStatus::Cancelled));
        }

        #[test]
        fn new_project_member_field_access() {
            let member = NewProjectMember {
                project_id: 1,
                user_id: 2,
                role: 3,
            };

            assert_eq!(member.project_id, 1);
            assert_eq!(member.user_id, 2);
            assert_eq!(member.role, 3);
        }

        #[test]
        fn user_create_request_field_access() {
            let req = UserCreateRequest {
                username: "user".to_string(),
                name: "Name".to_string(),
                email: "email@test.com".to_string(),
                password: "plaintext".to_string(),
                is_admin: true,
            };

            assert_eq!(req.username, "user");
            assert_eq!(req.password, "plaintext");
            assert!(req.is_admin);
        }

        #[test]
        fn login_form_field_access() {
            let form = LoginForm {
                username: "user".to_string(),
                password: "pass".to_string(),
            };

            assert_eq!(form.username, "user");
            assert_eq!(form.password, "pass");
        }

        #[test]
        fn change_password_form_field_access() {
            let form = ChangePasswordForm {
                current_password: "old".to_string(),
                new_password: "new".to_string(),
                confirm_password: "new".to_string(),
            };

            assert_eq!(form.current_password, "old");
            assert_eq!(form.new_password, "new");
            assert_eq!(form.confirm_password, "new");
        }

        #[test]
        fn new_test_form_field_access() {
            let form = NewVerificationForm {
                name: "Test".to_string(),
                reference_code: "REF".to_string(),
                description: "Desc".to_string(),
                source: "source.rs".to_string(),
                status_id: 1,
                parent_id: None,
                verification_method_id: None,
                verification_req: vec![1, 2, 3],
                project_id: 1,
            };

            assert_eq!(form.name, "Test");
            assert_eq!(form.verification_req, vec![1, 2, 3]);
        }

        #[test]
        fn edit_test_form_field_access() {
            let form = EditVerificationForm {
                id: 10,
                reference_code: "REF".to_string(),
                name: "Test".to_string(),
                description: "Desc".to_string(),
                source: "source.rs".to_string(),
                status_id: 1,
                parent_id: None,
                verification_method_id: None,
                linked_requirements: vec![1, 2],
                project_id: 1,
            };

            assert_eq!(form.id, 10);
            assert_eq!(form.linked_requirements, vec![1, 2]);
        }

        #[test]
        fn update_user_field_access() {
            let update = UpdateUser {
                id: Some(1),
                username: "user".to_string(),
                name: "Name".to_string(),
                email: "email@test.com".to_string(),
                is_admin: true,
            };

            assert_eq!(update.id, Some(1));
            assert_eq!(update.username, "user");
            assert!(update.is_admin);
        }

        #[test]
        fn import_mapping_form_field_access() {
            let form = ImportMappingForm {
                column_mappings: "{}".to_string(),
                import_type: "requirements".to_string(),
                temp_file: "file.xls".to_string(),
            };

            assert_eq!(form.column_mappings, "{}");
            assert_eq!(form.import_type, "requirements");
            assert_eq!(form.temp_file, "file.xls");
        }

        #[test]
        fn new_log_field_access() {
            let log = NewLog {
                user_id: 1,
                action_type: "CREATE".to_string(),
                entity_type: "REQUIREMENT".to_string(),
                entity_id: Some(2),
                project_id: Some(3),
                old_values: Some("old".to_string()),
                new_values: Some("new".to_string()),
                description: Some("Desc".to_string()),
                ip_address: Some("127.0.0.1".to_string()),
                user_agent: Some("Mozilla".to_string()),
            };

            assert_eq!(log.user_id, 1);
            assert_eq!(log.action_type, "CREATE");
            assert_eq!(log.entity_id, Some(2));
        }

        // Test serialization
        #[test]
        fn new_requirement_serialization() {
            let new_req = NewRequirement {
                id: Some(1),
                title: "Test".to_string(),
                description: "Desc".to_string(),
                author_id: 1,
                category_id: 1,
                status_id: 1,
                reference_code: "REF".to_string(),
                reviewer_id: 1,
                applicability_id: 1,
                justification: None,
                project_id: 1,
            };

            let json = serde_json::to_string(&new_req).unwrap();
            let deserialized: NewRequirement = serde_json::from_str(&json).unwrap();

            assert_eq!(new_req.title, deserialized.title);
            assert_eq!(new_req.project_id, deserialized.project_id);
        }

        #[test]
        fn new_user_serialization_skips_password() {
            let new_user = NewUser {
                id: Some(1),
                username: "user".to_string(),
                name: "Name".to_string(),
                email: "email@test.com".to_string(),
                password_hash: "secret_hash".to_string(),
                is_admin: false,
            };

            let json = serde_json::to_string(&new_user).unwrap();
            // Password hash should not be in serialized output
            assert!(!json.contains("secret_hash"));
            assert!(!json.contains("password_hash"));
        }
    }

    // ============================================================================
    // Tests for decorators.rs
    // ============================================================================

    mod decorators_tests {
        use super::*;

        #[test]
        fn decorated_requirement_field_access() {
            let decorated = DecoratedRequirement {
                id: 1,
                current_version_id: None,
                title: "Test".to_string(),
                description: "Desc".to_string(),
                verification_method_id: "Test".to_string(),
                req_verification_ids: vec![1],
                status_id: "Accepted".to_string(),
                req_current_status_id: 3,
                status_tag_color: None,
                author_id: "John Doe".to_string(),
                req_author_id: 1,
                reviewer_id: "Jane Doe".to_string(),
                req_reviewer_id: 2,
                reference_code: "REF-001".to_string(),
                category_id: "Security".to_string(),
                req_category_id: 5,
                applicability_id: "All Systems".to_string(),
                req_applicability_id: 2,
                req_parent_id: None,
                req_parent_title: "".to_string(),
                req_parents: vec![],
                req_parent_reference_code: "".to_string(),
                req_parent_description: "".to_string(),
                req_parent_status_id: "".to_string(),
                req_parent_status_tag_color: None,
                req_parent_category_id: "".to_string(),
                creation_date: "2024-01-15".to_string(),
                update_date: "2024-01-15".to_string(),
                deadline_date: "2024-12-31".to_string(),
                justification: Some("Important".to_string()),
                project_id: 1,
                approval_state: "draft".to_string(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            };

            assert_eq!(decorated.id, 1);
            assert_eq!(decorated.title, "Test");
            assert_eq!(decorated.status_id, "Accepted");
            assert_eq!(decorated.req_current_status_id, 3);
            assert_eq!(decorated.author_id, "John Doe");
            assert_eq!(decorated.req_author_id, 1);
        }

        #[test]
        fn decorated_requirement_with_parent() {
            let decorated = DecoratedRequirement {
                id: 2,
                current_version_id: None,
                title: "Child".to_string(),
                description: "Desc".to_string(),
                verification_method_id: "Test".to_string(),
                req_verification_ids: vec![1],
                status_id: "Draft".to_string(),
                req_current_status_id: 1,
                status_tag_color: None,
                author_id: "User".to_string(),
                req_author_id: 1,
                reviewer_id: "Reviewer".to_string(),
                req_reviewer_id: 2,
                reference_code: "REF-002".to_string(),
                category_id: "Category".to_string(),
                req_category_id: 1,
                applicability_id: "All".to_string(),
                req_applicability_id: 1,
                req_parent_id: Some(1),
                req_parent_title: "Parent Requirement".to_string(),
                req_parents: vec![],
                req_parent_reference_code: "".to_string(),
                req_parent_description: "".to_string(),
                req_parent_status_id: "".to_string(),
                req_parent_status_tag_color: None,
                req_parent_category_id: "".to_string(),
                creation_date: "2024-01-15".to_string(),
                update_date: "2024-01-15".to_string(),
                deadline_date: "".to_string(),
                justification: None,
                project_id: 1,
                approval_state: "draft".to_string(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            };

            assert_eq!(decorated.req_parent_id, Some(1));
            assert_eq!(decorated.req_parent_title, "Parent Requirement");
        }

        #[test]
        fn decorated_verification_field_access() {
            let decorated = DecoratedVerification {
                id: 1,
                reference_code: "VER-001".to_string(),
                name: "Unit Verification".to_string(),
                description: "Desc".to_string(),
                source: "ver.rs".to_string(),
                status_id: "Passed".to_string(),
                status_variant: "passed".to_string(),
                verification_status_id: 1,
                status_tag_color: None,
                verification_parent_id: None,
                verification_parent_title: "".to_string(),
                verification_parent_reference_code: "".to_string(),
                verification_parent_description: "".to_string(),
                verification_parent_status_id: "".to_string(),
                verification_parent_status_variant: "".to_string(),
                verification_parent_status_tag_color: None,
                verification_parent_source: "".to_string(),
                project_id: 1,
                verification_method_id: None,
                verification_method_title: None,
            };

            assert_eq!(decorated.id, 1);
            assert_eq!(decorated.name, "Unit Verification");
            assert_eq!(decorated.status_id, "Passed");
            assert_eq!(decorated.verification_status_id, 1);
        }

        #[test]
        fn decorated_verification_with_parent() {
            let decorated = DecoratedVerification {
                id: 2,
                reference_code: "VER-002".to_string(),
                name: "Child Verification".to_string(),
                description: "Desc".to_string(),
                source: "child.rs".to_string(),
                status_id: "Pending".to_string(),
                status_variant: "proposal".to_string(),
                verification_status_id: 3,
                status_tag_color: None,
                verification_parent_id: Some(1),
                verification_parent_title: "Parent Verification".to_string(),
                verification_parent_reference_code: "".to_string(),
                verification_parent_description: "".to_string(),
                verification_parent_status_id: "".to_string(),
                verification_parent_status_variant: "".to_string(),
                verification_parent_status_tag_color: None,
                verification_parent_source: "".to_string(),
                project_id: 1,
                verification_method_id: None,
                verification_method_title: None,
            };

            assert_eq!(decorated.verification_parent_id, Some(1));
            assert_eq!(decorated.verification_parent_title, "Parent Verification");
        }

        #[test]
        fn decorated_requirement_serialization() {
            let decorated = DecoratedRequirement {
                id: 1,
                current_version_id: None,
                title: "Test".to_string(),
                description: "Desc".to_string(),
                verification_method_id: "Test".to_string(),
                req_verification_ids: vec![1],
                status_id: "Accepted".to_string(),
                req_current_status_id: 3,
                status_tag_color: None,
                author_id: "Author".to_string(),
                req_author_id: 1,
                reviewer_id: "Reviewer".to_string(),
                req_reviewer_id: 2,
                reference_code: "REF".to_string(),
                category_id: "Category".to_string(),
                req_category_id: 1,
                applicability_id: "All".to_string(),
                req_applicability_id: 1,
                req_parent_id: None,
                req_parent_title: "".to_string(),
                req_parents: vec![],
                req_parent_reference_code: "".to_string(),
                req_parent_description: "".to_string(),
                req_parent_status_id: "".to_string(),
                req_parent_status_tag_color: None,
                req_parent_category_id: "".to_string(),
                creation_date: "2024-01-15".to_string(),
                update_date: "2024-01-15".to_string(),
                deadline_date: "".to_string(),
                justification: None,
                project_id: 1,
                approval_state: "draft".to_string(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            };

            let json = serde_json::to_string(&decorated).unwrap();
            let deserialized: DecoratedRequirement = serde_json::from_str(&json).unwrap();

            assert_eq!(decorated.id, deserialized.id);
            assert_eq!(decorated.title, deserialized.title);
        }

        #[test]
        fn decorated_verification_serialization() {
            let decorated = DecoratedVerification {
                id: 1,
                reference_code: "VER-001".to_string(),
                name: "Verification".to_string(),
                description: "Desc".to_string(),
                source: "ver.rs".to_string(),
                status_id: "Passed".to_string(),
                status_variant: "passed".to_string(),
                verification_status_id: 1,
                status_tag_color: None,
                verification_parent_id: None,
                verification_parent_title: "".to_string(),
                verification_parent_reference_code: "".to_string(),
                verification_parent_description: "".to_string(),
                verification_parent_status_id: "".to_string(),
                verification_parent_status_variant: "".to_string(),
                verification_parent_status_tag_color: None,
                verification_parent_source: "".to_string(),
                project_id: 1,
                verification_method_id: None,
                verification_method_title: None,
            };

            let json = serde_json::to_string(&decorated).unwrap();
            let deserialized: DecoratedVerification = serde_json::from_str(&json).unwrap();

            assert_eq!(decorated.id, deserialized.id);
            assert_eq!(decorated.name, deserialized.name);
        }
    }

    // ============================================================================
    // Additional edge case tests
    // ============================================================================

    mod edge_cases_tests {
        use super::*;

        #[test]
        fn entity_type_equality() {
            assert_eq!(EntityType::Project, EntityType::Project);
            assert_eq!(EntityType::Requirement, EntityType::Requirement);
            assert_ne!(EntityType::Project, EntityType::Requirement);
        }

        #[test]
        fn action_type_all_variants() {
            // Test all variants are distinct
            let variants = [
                ActionType::Create,
                ActionType::Update,
                ActionType::Delete,
                ActionType::Login,
                ActionType::Logout,
                ActionType::Export,
                ActionType::Import,
                ActionType::StatusChange,
            ];

            for (i, v1) in variants.iter().enumerate() {
                for (j, v2) in variants.iter().enumerate() {
                    if i == j {
                        assert_eq!(v1, v2);
                    } else {
                        assert_ne!(v1, v2);
                    }
                }
            }
        }

        #[test]
        fn requirement_clone() {
            let req = Requirement {
                id: 1,
                current_version_id: None,
                same_as_current: None,
                title: "Test".to_string(),
                description: "Desc".to_string(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: test_timestamp(),
                update_date: test_timestamp(),
                deadline_date: None,
                applicability_id: 1,
                justification: None,
                project_id: 1,
                approval_state: "draft".to_string(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            };

            let cloned = req.clone();
            assert_eq!(req.id, cloned.id);
            assert_eq!(req.title, cloned.title);
        }

        #[test]
        fn user_clone() {
            let user = User {
                id: 1,
                username: "user".to_string(),
                name: "Name".to_string(),
                email: "email@test.com".to_string(),
                creation_date: test_timestamp(),
                last_login: test_timestamp(),
                password_hash: "hash".to_string(),
                is_admin: true,
            };

            let cloned = user.clone();
            assert_eq!(user.id, cloned.id);
            assert_eq!(user.username, cloned.username);
            assert_eq!(user.is_admin, cloned.is_admin);
        }

        #[test]
        fn matrix_link_clone() {
            let link = MatrixLink {
                req_id: 1,
                verification_id: 2,
                creation_date: test_timestamp(),
                project_id: 3,
                suspect: false,
                suspect_at: None,
                suspect_reason: None,
                cleared_by: None,
                cleared_at: None,
                triggering_version_id: None,
                triggering_user_id: None,
            };

            let cloned = link.clone();
            assert_eq!(link.req_id, cloned.req_id);
            assert_eq!(link.verification_id, cloned.verification_id);
        }

        #[test]
        fn requirement_with_all_option_fields() {
            let req = Requirement {
                id: 1,
                current_version_id: None,
                same_as_current: None,
                title: "Test".to_string(),
                description: "Desc".to_string(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF".to_string(),
                category_id: 1,
                parent_id: Some(10),
                creation_date: test_timestamp(),
                update_date: test_timestamp(),
                deadline_date: Some(test_timestamp()),
                applicability_id: 1,
                justification: Some("Justification text".to_string()),
                project_id: 1,
                approval_state: "draft".to_string(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            };

            assert_eq!(req.parent_id, Some(10));
            assert_eq!(req.deadline_date, Some(test_timestamp()));
            assert_eq!(req.justification, Some("Justification text".to_string()));
        }

        #[test]
        fn verification_with_all_option_fields() {
            let ver = Verification {
                id: 1,
                name: "Verification".to_string(),
                reference_code: "REF".to_string(),
                description: "Desc".to_string(),
                source: "source.rs".to_string(),
                status_id: 1,
                parent_id: Some(5),
                project_id: 1,
                verification_method_id: None,
                author_id: 1,
                reviewer_id: 1,
                status_set_by: None,
                status_set_at: None,
            };

            assert_eq!(ver.parent_id, Some(5));
        }

        #[test]
        fn project_with_all_option_fields() {
            let project = Project {
                id: 1,
                name: "Project".to_string(),
                description: Some("Description".to_string()),
                creation_date: Some(test_timestamp()),
                update_date: Some(test_timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(1),
                slug: "project".to_string(),
                group_id: None,
            };

            assert_eq!(project.description, Some("Description".to_string()));
            assert_eq!(project.owner_id, Some(1));
        }

        #[test]
        fn project_with_none_fields() {
            let project = Project {
                id: 1,
                name: "Project".to_string(),
                description: None,
                creation_date: None,
                update_date: None,
                status: ProjectStatus::OnHold,
                owner_id: None,
                slug: "project".to_string(),
                group_id: None,
            };

            assert_eq!(project.description, None);
            assert_eq!(project.owner_id, None);
        }

        #[test]
        fn new_requirement_with_all_fields() {
            let new_req = NewRequirement {
                id: Some(1),
                title: "Title".to_string(),
                description: "Desc".to_string(),
                author_id: 1,
                category_id: 1,
                status_id: 1,
                reference_code: "REF".to_string(),
                reviewer_id: 1,
                applicability_id: 1,
                justification: Some("Just".to_string()),
                project_id: 1,
            };

            assert_eq!(new_req.id, Some(1));
            assert_eq!(new_req.justification, Some("Just".to_string()));
        }

        #[test]
        fn new_verification_with_all_fields() {
            let new_ver = NewVerification {
                id: Some(1),
                reference_code: "REF".to_string(),
                name: "Verification".to_string(),
                description: "Desc".to_string(),
                source: "source.rs".to_string(),
                status_id: 1,
                parent_id: Some(5),
                project_id: 1,
                verification_method_id: None,
                author_id: 1,
                reviewer_id: 1,
            };

            assert_eq!(new_ver.id, Some(1));
            assert_eq!(new_ver.parent_id, Some(5));
        }

        #[test]
        fn tagged_entities_serialization() {
            let cat = Category {
                id: 1,
                title: "Security".to_string(),
                description: "Desc".to_string(),
                tag: "SEC".to_string(),
                project_id: 1,
            };

            let json = serde_json::to_string(&cat).unwrap();
            let deserialized: Category = serde_json::from_str(&json).unwrap();

            assert_eq!(cat.id, deserialized.id);
            assert_eq!(cat.tag, deserialized.tag);
        }

        #[test]
        fn verification_method_serialization() {
            let ver = VerificationMethod {
                id: 1,
                title: "Test".to_string(),
                description: "Desc".to_string(),
                tag: "TEST".to_string(),
                project_id: 1,
            };

            let json = serde_json::to_string(&ver).unwrap();
            let deserialized: VerificationMethod = serde_json::from_str(&json).unwrap();

            assert_eq!(ver.id, deserialized.id);
            assert_eq!(ver.title, deserialized.title);
        }

        #[test]
        fn verification_status_serialization() {
            let status = VerificationStatus {
                id: 1,
                title: "Passed".to_string(),
                description: "Desc".to_string(),
                tag: "PASS".to_string(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            };

            let json = serde_json::to_string(&status).unwrap();
            let deserialized: VerificationStatus = serde_json::from_str(&json).unwrap();

            assert_eq!(status.id, deserialized.id);
            assert_eq!(status.title, deserialized.title);
        }
    }
}

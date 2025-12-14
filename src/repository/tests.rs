#[cfg(test)]
mod tests {
    use crate::models::*;
    use crate::repository::errors::RepoError;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::repository::{
        UserRepository, RequirementsRepository, TestsCaseRepository, LookupRepository,
        ProjectsRepository, MatrixRepository
    };
    use crate::status_enums::ProjectStatus;
    use chrono::{NaiveDate, NaiveDateTime};

    fn test_datetime() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    fn create_test_user() -> User {
        User {
            id: 1,
            username: "testuser".to_string(),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            user_level: 1,
            creation_date: test_datetime(),
            last_login: test_datetime(),
            password_hash: "hashed_password".to_string(),
            project_id: Some(1),
            is_admin: false,
        }
    }

    fn create_test_requirement() -> Requirement {
        Requirement {
            id: 1,
            title: "Test Requirement".to_string(),
            description: "Test Description".to_string(),
            verification_method_id: 1,
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            req_link: "http://example.com".to_string(),
            reference_code: "REF-001".to_string(),
            category_id: 1,
            parent_id: None,
            creation_date: test_datetime(),
            update_date: test_datetime(),
            deadline_date: test_datetime(),
            applicability_id: 1,
            justification: None,
            project_id: 1,
        }
    }

    fn create_test_test() -> TestCase {
        TestCase {
            id: 1,
            name: "Test Test".to_string(),
            description: "Test Description".to_string(),
            source: "Manual".to_string(),
            reference_code: "TEST-1".to_string(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
        }
    }

    fn create_test_project() -> Project {
        Project {
            id: 1,
            name: "Test Project".to_string(),
            description: Some("Test Project Description".to_string()),
            creation_date: Some(test_datetime()),
            update_date: Some(test_datetime()),
            status: ProjectStatus::Active,
            owner_id: Some(1),
        }
    }

    fn create_test_requirement_status() -> RequirementStatus {
        RequirementStatus {
            id: 1,
            title: "Draft".to_string(),
            description: "Draft status".to_string(),
            tag: "DRAFT".to_string(),
            project_id: 1,
        }
    }

    fn create_test_test_status() -> TestStatus {
        TestStatus {
            id: 1,
            title: "Draft".to_string(),
            description: "Draft status".to_string(),
            tag: "DRAFT".to_string(),
            project_id: 1,
        }
    }

    fn create_test_category() -> Category {
        Category {
            id: 1,
            title: "Test Category".to_string(),
            description: "Test Category Description".to_string(),
            tag: "TEST".to_string(),
            project_id: 1,
        }
    }

    fn create_test_applicability() -> Applicability {
        Applicability {
            id: 1,
            title: "Test Applicability".to_string(),
            description: "Test Applicability Description".to_string(),
            tag: "TEST".to_string(),
            project_id: 1,
        }
    }

    fn create_test_verification() -> VerificationMethod {
        VerificationMethod {
            id: 1,
            title: "Test Verification".to_string(),
            description: "Test Verification Description".to_string(),
            tag: "TEST_VERIFICATION".to_string(),
            project_id: 1,
        }
    }

    fn create_test_matrix() -> MatrixLink {
        MatrixLink {
            req_id: 1,
            id: 1,
            creation_date: test_datetime(),
            project_id: 1,
        }
    }

    // UserRepository Tests
    #[test]
    fn test_user_repository_get_users_all() {
        let user = create_test_user();
        let repo = DieselRepoMock::with_users(vec![user.clone()]);
        
        let result = repo.get_users_all();
        assert!(result.is_ok());
        let users = result.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].username, "testuser");
    }

    #[test]
    fn test_user_repository_get_user_by_id() {
        let user = create_test_user();
        let repo = DieselRepoMock::with_users(vec![user.clone()]);
        
        let result = repo.get_user_by_id(1);
        assert!(result.is_ok());
        let found_user = result.unwrap();
        assert_eq!(found_user.username, "testuser");
    }

    #[test]
    fn test_user_repository_get_user_by_id_not_found() {
        let repo = DieselRepoMock::default();
        
        let result = repo.get_user_by_id(999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::NotFound));
    }

    #[test]
    fn test_user_repository_get_user_by_username() {
        let user = create_test_user();
        let repo = DieselRepoMock::with_users(vec![user.clone()]);
        
        let result = repo.get_user_by_username("testuser");
        assert!(result.is_ok());
        let found_user = result.unwrap();
        assert!(found_user.is_some());
        assert_eq!(found_user.unwrap().username, "testuser");
    }

    #[test]
    fn test_user_repository_get_user_by_username_not_found() {
        let repo = DieselRepoMock::default();
        
        let result = repo.get_user_by_username("nonexistent");
        assert!(result.is_ok());
        let found_user = result.unwrap();
        assert!(found_user.is_none());
    }

    #[test]
    fn test_user_repository_insert_user() {
        let mut repo = DieselRepoMock::default();
        let new_user = NewUser {
            id: None,
            username: "newuser".to_string(),
            name: "New User".to_string(),
            email: "new@example.com".to_string(),
            user_level: 1,
            password_hash: "hash".to_string(),
            project_id: Some(1),
            is_admin: false,
        };
        
        let result = repo.insert_user(&new_user);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // DieselRepoMock returns 0
    }

    #[test]
    fn test_user_repository_update_user_password() {
        let mut repo = DieselRepoMock::default();
        
        let result = repo.update_user_password(1, "new_hash");
        assert!(result.is_err()); // DieselRepoMock returns NotFound for non-existent user
    }

    #[test]
    fn test_user_repository_update_user() {
        let mut repo = DieselRepoMock::default();
        let user_data = NewUser {
            id: Some(1),
            username: "updated".to_string(),
            name: "Updated User".to_string(),
            email: "updated@example.com".to_string(),
            user_level: 1,
            password_hash: "hash".to_string(),
            project_id: Some(1),
            is_admin: false,
        };
        
        let result = repo.update_user(&user_data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true); // DieselRepoMock returns true
    }

    #[test]
    fn test_user_repository_update_user_without_password() {
        let mut repo = DieselRepoMock::default();
        let user_data = UpdateUser {
            id: Some(1),
            username: "updated".to_string(),
            name: "Updated User".to_string(),
            email: "updated@example.com".to_string(),
            user_level: 1,
            is_admin: false,
        };
        
        let result = repo.update_user_without_password(&user_data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true); // DieselRepoMock returns true
    }

    #[test]
    fn test_user_repository_delete_user() {
        let mut repo = DieselRepoMock::default();
        
        let result = repo.delete_user(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true); // DieselRepoMock returns true
    }

    // RequirementsRepository Tests
    #[test]
    fn test_requirements_repository_get_requirement_by_id() {
        let mut repo = DieselRepoMock::default();
        let requirement = create_test_requirement();
        repo.requirements.insert(1, requirement.clone());
        
        let result = repo.get_requirement_by_id(1);
        assert!(result.is_ok());
        let found_req = result.unwrap();
        assert_eq!(found_req.title, "Test Requirement");
    }

    #[test]
    fn test_requirements_repository_get_requirement_by_id_not_found() {
        let repo = DieselRepoMock::default();
        
        let result = repo.get_requirement_by_id(999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::NotFound));
    }

    #[test]
    fn test_requirements_repository_get_requirements_all() {
        let mut repo = DieselRepoMock::default();
        let requirement = create_test_requirement();
        repo.requirements.insert(1, requirement.clone());
        
        let result = repo.get_requirements_all();
        assert!(result.is_ok());
        let requirements = result.unwrap();
        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].title, "Test Requirement");
    }

    #[test]
    fn test_requirements_repository_get_requirements_by_project() {
        let mut repo = DieselRepoMock::default();
        let requirement = create_test_requirement();
        repo.requirements.insert(1, requirement.clone());
        
        let result = repo.get_requirements_by_project(1);
        assert!(result.is_ok());
        let requirements = result.unwrap();
        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].project_id, 1);
    }

    #[test]
    fn test_requirements_repository_get_requirements_by_category() {
        let mut repo = DieselRepoMock::default();
        let requirement = create_test_requirement();
        repo.requirements.insert(1, requirement.clone());
        
        let result = repo.get_requirements_by_category(1);
        assert!(result.is_ok());
        let requirements = result.unwrap();
        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].category_id, 1);
    }

    #[test]
    fn test_requirements_repository_get_requirements_by_status() {
        let mut repo = DieselRepoMock::default();
        let requirement = create_test_requirement();
        repo.requirements.insert(1, requirement.clone());
        
        let result = repo.get_requirements_by_status(1);
        assert!(result.is_ok());
        let requirements = result.unwrap();
        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].status_id, 1);
    }

    #[test]
    fn test_requirements_repository_insert_new_requirement() {
        let mut repo = DieselRepoMock::default();
        let new_req = NewRequirement {
            id: None,
            title: "New Requirement".to_string(),
            description: "New Description".to_string(),
            verification_method_id: 1,
            author_id: 1,
            req_link: "http://example.com".to_string(),
            category_id: 1,
            status_id: 1,
            parent_id: None,
            reference_code: "REQ-NEW".to_string(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: None,
            project_id: 1,
        };
        
        let result = repo.insert_new_requirement(&new_req);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // DieselRepoMock returns 0
    }

    #[test]
    fn test_requirements_repository_edit_requirement() {
        let mut repo = DieselRepoMock::default();
        let new_req = NewRequirement {
            id: Some(1),
            title: "Updated Requirement".to_string(),
            description: "Updated Description".to_string(),
            verification_method_id: 1,
            author_id: 1,
            req_link: "http://example.com".to_string(),
            category_id: 1,
            status_id: 1,
            parent_id: None,
            reference_code: "REQ-UPDATED".to_string(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: None,
            project_id: 1,
        };
        
        let result = repo.edit_requirement(&new_req);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // DieselRepoMock returns false
    }

    #[test]
    fn test_requirements_repository_delete_requirement() {
        let mut repo = DieselRepoMock::default();
        
        let result = repo.delete_requirement(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // DieselRepoMock returns false
    }

    #[test]
    fn test_requirements_repository_update_requirement() {
        let mut repo = DieselRepoMock::default();
        
        let result = repo.update_requirement(1);
        assert!(result.is_ok());
    }

    // TestsCaseRepository Tests
    #[test]
    fn test_tests_repository_get_test_by_id() {
        let mut repo = DieselRepoMock::default();
        let test = create_test_test();
        repo.tests.insert(1, test.clone());
        
        let result = repo.get_test_by_id(1);
        assert!(result.is_ok());
        let found_test = result.unwrap();
        assert_eq!(found_test.name, "Test Test");
    }

    #[test]
    fn test_tests_repository_get_test_by_id_not_found() {
        let repo = DieselRepoMock::default();
        
        let result = repo.get_test_by_id(999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::NotFound));
    }

    #[test]
    fn test_tests_repository_get_tests_all() {
        let mut repo = DieselRepoMock::default();
        let test = create_test_test();
        repo.tests.insert(1, test.clone());
        
        let result = repo.get_tests_all();
        assert!(result.is_ok());
        let tests = result.unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "Test Test");
    }

    #[test]
    fn test_tests_repository_get_tests_by_project() {
        let mut repo = DieselRepoMock::default();
        let test = create_test_test();
        repo.tests.insert(1, test.clone());
        
        let result = repo.get_tests_by_project(1);
        assert!(result.is_ok());
        let tests = result.unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].project_id, 1);
    }

    #[test]
    fn test_tests_repository_get_tests_by_status() {
        let mut repo = DieselRepoMock::default();
        let test = create_test_test();
        repo.tests.insert(1, test.clone());
        
        let result = repo.get_tests_by_status(1);
        assert!(result.is_ok());
        let tests = result.unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].status_id, 1);
    }

    #[test]
    fn test_tests_repository_get_tests_by_parent() {
        let mut repo = DieselRepoMock::default();
        let test = create_test_test();
        repo.tests.insert(1, test.clone());
        
        let result = repo.get_tests_by_parent(0);
        assert!(result.is_ok());
        let tests = result.unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].parent_id, 0);
    }

    #[test]
    fn test_tests_repository_get_requirements_for_test() {
        let mut repo = DieselRepoMock::default();
        let requirement = create_test_requirement();
        let matrix = create_test_matrix();
        repo.requirements.insert(1, requirement.clone());
        repo.matrices.push(matrix);
        
        let result = repo.get_requirements_for_test(1);
        assert!(result.is_ok());
        let requirements = result.unwrap();
        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].id, 1);
    }

    #[test]
    fn test_tests_repository_get_tests_for_requirement() {
        let mut repo = DieselRepoMock::default();
        let test = create_test_test();
        let matrix = create_test_matrix();
        repo.tests.insert(1, test.clone());
        repo.matrices.push(matrix);
        
        let result = repo.get_tests_for_requirement(1);
        assert!(result.is_ok());
        let tests = result.unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].id, 1);
    }

    #[test]
    fn test_tests_repository_insert_test() {
        let mut repo = DieselRepoMock::default();
        let new_test = NewTestCase {
            id: None,
            name: "New Test".to_string(),
            description: "New Description".to_string(),
            source: "Manual".to_string(),
            reference_code: "TEST-NEW".to_string(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
        };
        
        let result = repo.insert_test(&new_test);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // DieselRepoMock returns 0
    }

    #[test]
    fn test_tests_repository_edit_test() {
        let mut repo = DieselRepoMock::default();
        let new_test = NewTestCase {
            id: Some(1),
            name: "Updated Test".to_string(),
            description: "Updated Description".to_string(),
            source: "Manual".to_string(),
            reference_code: "TEST-UPDATED".to_string(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
        };
        
        let result = repo.edit_test(&new_test);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // DieselRepoMock returns false
    }

    #[test]
    fn test_tests_repository_delete_test() {
        let mut repo = DieselRepoMock::default();
        
        let result = repo.delete_test(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // DieselRepoMock returns false
    }

    #[test]
    fn test_tests_repository_update_test_requirement_links() {
        let mut repo = DieselRepoMock::default();
        
        let result = repo.update_test_requirement_links(1, &[1, 2, 3]);
        assert!(result.is_ok());
    }

    // LookupRepository Tests
    #[test]
    fn test_lookup_repository_get_requirement_status_all() {
        let mut repo = DieselRepoMock::default();
        let status = create_test_requirement_status();
        repo.requirement_statuses.insert(1, status.clone());
        
        let result = repo.get_requirement_status_all();
        assert!(result.is_ok());
        let statuses = result.unwrap();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].title, "Draft");
    }

    #[test]
    fn test_lookup_repository_get_requirement_status_by_id() {
        let mut repo = DieselRepoMock::default();
        let status = create_test_requirement_status();
        repo.requirement_statuses.insert(1, status.clone());
        
        let result = repo.get_requirement_status_by_id(1);
        assert!(result.is_ok());
        let found_status = result.unwrap();
        assert_eq!(found_status.title, "Draft");
    }

    #[test]
    fn test_lookup_repository_get_requirement_status_by_id_not_found() {
        let repo = DieselRepoMock::default();
        
        let result = repo.get_requirement_status_by_id(999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::NotFound));
    }

    #[test]
    fn test_lookup_repository_get_test_status_all() {
        let mut repo = DieselRepoMock::default();
        let status = create_test_test_status();
        repo.test_statuses.insert(1, status.clone());
        
        let result = repo.get_test_status_all();
        assert!(result.is_ok());
        let statuses = result.unwrap();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].title, "Draft");
    }

    #[test]
    fn test_lookup_repository_get_test_status_by_id() {
        let mut repo = DieselRepoMock::default();
        let status = create_test_test_status();
        repo.test_statuses.insert(1, status.clone());
        
        let result = repo.get_test_status_by_id(1);
        assert!(result.is_ok());
        let found_status = result.unwrap();
        assert_eq!(found_status.title, "Draft");
    }

    #[test]
    fn test_lookup_repository_get_test_status_by_id_not_found() {
        let repo = DieselRepoMock::default();
        
        let result = repo.get_test_status_by_id(999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::NotFound));
    }

    #[test]
    fn test_lookup_repository_get_categories_all() {
        let mut repo = DieselRepoMock::default();
        let category = create_test_category();
        repo.categories.insert(1, category.clone());
        
        let result = repo.get_categories_all();
        assert!(result.is_ok());
        let categories = result.unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].title, "Test Category");
    }

    #[test]
    fn test_lookup_repository_get_categories_by_project() {
        let mut repo = DieselRepoMock::default();
        let category = create_test_category();
        repo.categories.insert(1, category.clone());
        
        let result = repo.get_categories_by_project(1);
        assert!(result.is_ok());
        let categories = result.unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].project_id, 1);
    }

    #[test]
    fn test_lookup_repository_get_category_by_id() {
        let mut repo = DieselRepoMock::default();
        let category = create_test_category();
        repo.categories.insert(1, category.clone());
        
        let result = repo.get_category_by_id(1);
        assert!(result.is_ok());
        let found_category = result.unwrap();
        assert_eq!(found_category.title, "Test Category");
    }

    #[test]
    fn test_lookup_repository_get_category_by_id_not_found() {
        let repo = DieselRepoMock::default();
        
        let result = repo.get_category_by_id(999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::NotFound));
    }

    #[test]
    fn test_lookup_repository_get_applicability_all() {
        let mut repo = DieselRepoMock::default();
        let applicability = create_test_applicability();
        repo.applicability.insert(1, applicability.clone());
        
        let result = repo.get_applicability_all();
        assert!(result.is_ok());
        let applicability_list = result.unwrap();
        assert_eq!(applicability_list.len(), 1);
        assert_eq!(applicability_list[0].title, "Test Applicability");
    }

    #[test]
    fn test_lookup_repository_get_applicability_by_id() {
        let mut repo = DieselRepoMock::default();
        let applicability = create_test_applicability();
        repo.applicability.insert(1, applicability.clone());
        
        let result = repo.get_applicability_by_id(1);
        assert!(result.is_ok());
        let found_applicability = result.unwrap();
        assert_eq!(found_applicability.title, "Test Applicability");
    }

    #[test]
    fn test_lookup_repository_get_applicability_by_id_not_found() {
        let repo = DieselRepoMock::default();
        
        let result = repo.get_applicability_by_id(999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::NotFound));
    }

    #[test]
    fn test_lookup_repository_get_applicability_by_project() {
        let mut repo = DieselRepoMock::default();
        let applicability = create_test_applicability();
        repo.applicability.insert(1, applicability.clone());
        
        let result = repo.get_applicability_by_project(1);
        assert!(result.is_ok());
        let applicability_list = result.unwrap();
        assert_eq!(applicability_list.len(), 1);
        assert_eq!(applicability_list[0].project_id, 1);
    }

    #[test]
    fn test_lookup_repository_get_verification_all() {
        let mut repo = DieselRepoMock::default();
        let verification = create_test_verification();
        repo.verifications.insert(1, verification.clone());
        
        let result = repo.get_verification_all();
        assert!(result.is_ok());
        let verifications = result.unwrap();
        assert_eq!(verifications.len(), 1);
        assert_eq!(verifications[0].title, "Test Verification");
    }

    #[test]
    fn test_lookup_repository_get_verification_by_id() {
        let mut repo = DieselRepoMock::default();
        let verification = create_test_verification();
        repo.verifications.insert(1, verification.clone());
        
        let result = repo.get_verification_by_id(1);
        assert!(result.is_ok());
        let found_verification = result.unwrap();
        assert_eq!(found_verification.title, "Test Verification");
    }

    #[test]
    fn test_lookup_repository_get_verification_by_id_not_found() {
        let repo = DieselRepoMock::default();
        
        let result = repo.get_verification_by_id(999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::NotFound));
    }

    #[test]
    fn test_lookup_repository_get_verification_by_project() {
        let mut repo = DieselRepoMock::default();
        let verification = create_test_verification();
        repo.verifications.insert(1, verification.clone());
        
        let result = repo.get_verification_by_project(1);
        assert!(result.is_ok());
        let verifications = result.unwrap();
        assert_eq!(verifications.len(), 1);
        assert_eq!(verifications[0].project_id, 1);
    }

    #[test]
    fn test_lookup_repository_create_requirement_status() {
        let mut repo = DieselRepoMock::default();
        let new_status = NewRequirementStatus {
            id: None,
            title: "New Status".to_string(),
            description: "New Status Description".to_string(),
            tag: "NEW".to_string(),
            project_id: 1,
        };
        
        let result = repo.create_requirement_status(&new_status);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // DieselRepoMock returns next available ID
    }

    #[test]
    fn test_lookup_repository_create_test_status() {
        let mut repo = DieselRepoMock::default();
        let new_status = NewTestStatus {
            id: None,
            title: "New Test Status".to_string(),
            description: "Test Status Description".to_string(),
            tag: "TST".to_string(),
            project_id: 1,
        };
        
        let result = repo.create_test_status(&new_status);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // DieselRepoMock returns next available ID
    }

    #[test]
    fn test_lookup_repository_insert_new_category() {
        let mut repo = DieselRepoMock::default();
        let new_category = NewCategory {
            id: None,
            title: "New Category".to_string(),
            description: "New Category Description".to_string(),
            tag: "NEW".to_string(),
            project_id: 1,
        };
        
        let result = repo.insert_new_category(&new_category);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // DieselRepoMock returns 0
    }

    #[test]
    fn test_lookup_repository_edit_category() {
        let mut repo = DieselRepoMock::default();
        let new_category = NewCategory {
            id: Some(1),
            title: "Updated Category".to_string(),
            description: "Updated Category Description".to_string(),
            tag: "UPDATED".to_string(),
            project_id: 1,
        };
        
        let result = repo.edit_category(&new_category);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // DieselRepoMock returns false
    }

    #[test]
    fn test_lookup_repository_delete_category() {
        let mut repo = DieselRepoMock::default();
        
        let result = repo.delete_category(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // DieselRepoMock returns false
    }

    #[test]
    fn test_lookup_repository_insert_new_applicability() {
        let mut repo = DieselRepoMock::default();
        let new_applicability = NewApplicability {
            id: None,
            title: "New Applicability".to_string(),
            description: "New Applicability Description".to_string(),
            tag: "NEW".to_string(),
            project_id: 1,
        };
        
        let result = repo.insert_new_applicability(&new_applicability);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // DieselRepoMock returns 0
    }

    #[test]
    fn test_lookup_repository_edit_applicability() {
        let mut repo = DieselRepoMock::default();
        let new_applicability = NewApplicability {
            id: Some(1),
            title: "Updated Applicability".to_string(),
            description: "Updated Applicability Description".to_string(),
            tag: "UPDATED".to_string(),
            project_id: 1,
        };
        
        let result = repo.edit_applicability(&new_applicability);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // DieselRepoMock returns false
    }

    #[test]
    fn test_lookup_repository_delete_applicability() {
        let mut repo = DieselRepoMock::default();
        
        let result = repo.delete_applicability(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // DieselRepoMock returns false
    }

    // ProjectsRepository Tests
    #[test]
    fn test_projects_repository_get_projects_all() {
        let mut repo = DieselRepoMock::default();
        let project = create_test_project();
        repo.projects.insert(1, project.clone());
        
        let result = repo.get_projects_all();
        assert!(result.is_ok());
        let projects = result.unwrap();
        assert_eq!(projects.len(), 0); // DieselRepoMock returns empty vector
    }

    #[test]
    fn test_projects_repository_get_project_by_id() {
        let mut repo = DieselRepoMock::default();
        let project = create_test_project();
        repo.projects.insert(1, project.clone());
        
        let result = repo.get_project_by_id(1);
        assert!(result.is_err()); // DieselRepoMock returns NotFound
    }

    #[test]
    fn test_projects_repository_get_project_by_id_not_found() {
        let repo = DieselRepoMock::default();
        
        let result = repo.get_project_by_id(999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::NotFound));
    }

    #[test]
    fn test_projects_repository_insert_new_project() {
        let mut repo = DieselRepoMock::default();
        let new_project = NewProject {
            name: "New Project".to_string(),
            description: Some("New Project Description".to_string()),
            status_id: "Active".to_string(),
            owner_id: Some(1),
        };
        
        let result = repo.insert_new_project(&new_project);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // DieselRepoMock returns 0
    }

    #[test]
    fn test_projects_repository_edit_project() {
        let mut repo = DieselRepoMock::default();
        let update_project = UpdateProject {
            name: "Updated Project".to_string(),
            description: Some("Updated Project Description".to_string()),
            status_id: "Active".to_string(),
            owner_id: Some(1),
        };
        
        let result = repo.edit_project(1, &update_project);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // DieselRepoMock returns false
    }

    #[test]
    fn test_projects_repository_delete_project() {
        let mut repo = DieselRepoMock::default();
        
        let result = repo.delete_project(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // DieselRepoMock returns false
    }

    // MatrixRepository Tests
    #[test]
    fn test_matrix_repository_get_matrix_all() {
        let mut repo = DieselRepoMock::default();
        let matrix = create_test_matrix();
        repo.matrices.push(matrix.clone());
        
        let result = repo.get_matrix_all();
        assert!(result.is_ok());
        let matrices = result.unwrap();
        assert_eq!(matrices.len(), 1);
        assert_eq!(matrices[0].req_id, 1);
        assert_eq!(matrices[0].id, 1);
    }

    #[test]
    fn test_matrix_repository_get_matrix_by_project() {
        let mut repo = DieselRepoMock::default();
        let matrix = create_test_matrix();
        repo.matrices.push(matrix.clone());
        
        let result = repo.get_matrix_by_project(1);
        assert!(result.is_ok());
        let matrices = result.unwrap();
        assert_eq!(matrices.len(), 1);
        assert_eq!(matrices[0].project_id, 1);
    }

    #[test]
    fn test_matrix_repository_insert_new_matrix_item() {
        let mut repo = DieselRepoMock::default();
        let new_matrix = NewMatrixLink {
            req_id: 1,
            test_id: 1,
            project_id: 1,
        };
        
        let result = repo.insert_new_matrix_item(&new_matrix);
        assert!(result.is_ok());
    }

    #[test]
    fn test_matrix_repository_insert_matrix_link() {
        let mut repo = DieselRepoMock::default();
        
        let result = repo.insert_matrix_link(1, 1, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true); // DieselRepoMock returns true
    }

    #[test]
    fn test_matrix_repository_delete_matrix_link() {
        let mut repo = DieselRepoMock::default();
        
        let result = repo.delete_matrix_link(1, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // DieselRepoMock returns false
    }

    // Error Handling Tests
    #[test]
    fn test_repo_error_handling() {
        let repo = DieselRepoMock::with_error();
        
        // Test that error is propagated correctly
        let result = repo.get_user_by_id(1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::NotFound));
    }

    // Edge Cases Tests
    #[test]
    fn test_empty_repository_queries() {
        let repo = DieselRepoMock::default();
        
        // Test queries on empty repository
        assert_eq!(repo.get_users_all().unwrap().len(), 0);
        assert_eq!(repo.get_requirements_all().unwrap().len(), 0);
        assert_eq!(repo.get_tests_all().unwrap().len(), 0);
        assert_eq!(repo.get_projects_all().unwrap().len(), 0);
        assert_eq!(repo.get_matrix_all().unwrap().len(), 0);
        assert_eq!(repo.get_requirement_status_all().unwrap().len(), 0);
        assert_eq!(repo.get_test_status_all().unwrap().len(), 0);
        assert_eq!(repo.get_categories_all().unwrap().len(), 0);
        assert_eq!(repo.get_applicability_all().unwrap().len(), 0);
        assert_eq!(repo.get_verification_all().unwrap().len(), 0);
    }

    #[test]
    fn test_filtering_by_nonexistent_values() {
        let repo = DieselRepoMock::default();
        
        // Test filtering by nonexistent values
        assert_eq!(repo.get_requirements_by_project(999).unwrap().len(), 0);
        assert_eq!(repo.get_tests_by_project(999).unwrap().len(), 0);
        assert_eq!(repo.get_categories_by_project(999).unwrap().len(), 0);
        assert_eq!(repo.get_applicability_by_project(999).unwrap().len(), 0);
        assert_eq!(repo.get_verification_by_project(999).unwrap().len(), 0);
        assert_eq!(repo.get_matrix_by_project(999).unwrap().len(), 0);
    }

    #[test]
    fn test_matrix_relationships() {
        let mut repo = DieselRepoMock::default();
        
        // Add test data
        let requirement = create_test_requirement();
        let test = create_test_test();
        let matrix = create_test_matrix();
        
        repo.requirements.insert(1, requirement);
        repo.tests.insert(1, test);
        repo.matrices.push(matrix);
        
        // Test bidirectional relationships
        let requirements_for_test = repo.get_requirements_for_test(1).unwrap();
        assert_eq!(requirements_for_test.len(), 1);
        
        let tests_for_requirement = repo.get_tests_for_requirement(1).unwrap();
        assert_eq!(tests_for_requirement.len(), 1);
    }

    // ============================================================================
    // Additional Tests for Better Coverage
    // ============================================================================

    #[test]
    fn test_user_repository_update_user_with_existing_user() {
        let user = create_test_user();
        let mut repo = DieselRepoMock::with_users(vec![user]);
        
        let user_data = NewUser {
            id: Some(1),
            username: "updated".to_string(),
            name: "Updated User".to_string(),
            email: "updated@example.com".to_string(),
            user_level: 1,
            password_hash: "hash".to_string(),
            project_id: Some(1),
            is_admin: false,
        };
        
        let result = repo.update_user(&user_data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_user_repository_delete_user_with_existing_user() {
        let user = create_test_user();
        let mut repo = DieselRepoMock::with_users(vec![user]);
        
        let result = repo.delete_user(1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_requirements_repository_get_requirements_by_project_different_id() {
        let mut repo = DieselRepoMock::default();
        let mut requirement = create_test_requirement();
        requirement.project_id = 2;
        repo.requirements.insert(1, requirement);
        
        let result = repo.get_requirements_by_project(2);
        assert!(result.is_ok());
        let requirements = result.unwrap();
        assert_eq!(requirements.len(), 1);
    }

    #[test]
    fn test_requirements_repository_edit_requirement_with_existing() {
        let mut repo = DieselRepoMock::default();
        let requirement = create_test_requirement();
        repo.requirements.insert(1, requirement);
        
        let new_req = NewRequirement {
            id: Some(1),
            title: "Updated Requirement".to_string(),
            description: "Updated Description".to_string(),
            verification_method_id: 1,
            author_id: 1,
            req_link: "http://example.com".to_string(),
            category_id: 1,
            status_id: 1,
            parent_id: None,
            reference_code: "REQ-UPDATED".to_string(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: None,
            project_id: 1,
        };
        
        let result = repo.edit_requirement(&new_req);
        assert!(result.is_ok());
    }

    #[test]
    fn test_requirements_repository_delete_requirement_with_existing() {
        let mut repo = DieselRepoMock::default();
        let requirement = create_test_requirement();
        repo.requirements.insert(1, requirement);
        
        let result = repo.delete_requirement(1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tests_repository_get_tests_by_project_different_id() {
        let mut repo = DieselRepoMock::default();
        let mut test = create_test_test();
        test.project_id = 2;
        repo.tests.insert(1, test);
        
        let result = repo.get_tests_by_project(2);
        assert!(result.is_ok());
        let tests = result.unwrap();
        assert_eq!(tests.len(), 1);
    }

    #[test]
    fn test_tests_repository_edit_test_with_existing() {
        let mut repo = DieselRepoMock::default();
        let test = create_test_test();
        repo.tests.insert(1, test);
        
        let new_test = NewTestCase {
            id: Some(1),
            name: "Updated Test".to_string(),
            description: "Updated Description".to_string(),
            source: "Manual".to_string(),
            reference_code: "TEST-UPDATED".to_string(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
        };
        
        let result = repo.edit_test(&new_test);
        assert!(result.is_ok());
    }

    #[test]
    fn test_tests_repository_delete_test_with_existing() {
        let mut repo = DieselRepoMock::default();
        let test = create_test_test();
        repo.tests.insert(1, test);
        
        let result = repo.delete_test(1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lookup_repository_get_category_by_id_not_found() {
        let repo = DieselRepoMock::default();
        
        let result = repo.get_category_by_id(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_lookup_repository_get_applicability_by_id_not_found() {
        let repo = DieselRepoMock::default();
        
        let result = repo.get_applicability_by_id(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_lookup_repository_get_verification_by_id_not_found() {
        let repo = DieselRepoMock::default();
        
        let result = repo.get_verification_by_id(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_lookup_repository_edit_category_with_existing() {
        let mut repo = DieselRepoMock::default();
        let category = create_test_category();
        repo.categories.insert(1, category);
        
        let new_category = NewCategory {
            id: Some(1),
            title: "Updated Category".to_string(),
            description: "Updated Category Description".to_string(),
            tag: "UPDATED".to_string(),
            project_id: 1,
        };
        
        let result = repo.edit_category(&new_category);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lookup_repository_delete_category_with_existing() {
        let mut repo = DieselRepoMock::default();
        let category = create_test_category();
        repo.categories.insert(1, category);
        
        let result = repo.delete_category(1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lookup_repository_edit_applicability_with_existing() {
        let mut repo = DieselRepoMock::default();
        let applicability = create_test_applicability();
        repo.applicability.insert(1, applicability);
        
        let new_applicability = NewApplicability {
            id: Some(1),
            title: "Updated Applicability".to_string(),
            description: "Updated Applicability Description".to_string(),
            tag: "UPDATED".to_string(),
            project_id: 1,
        };
        
        let result = repo.edit_applicability(&new_applicability);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lookup_repository_delete_applicability_with_existing() {
        let mut repo = DieselRepoMock::default();
        let applicability = create_test_applicability();
        repo.applicability.insert(1, applicability);
        
        let result = repo.delete_applicability(1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_projects_repository_get_project_by_id_with_existing() {
        let mut repo = DieselRepoMock::default();
        let project = create_test_project();
        repo.projects.insert(1, project);
        
        let result = repo.get_project_by_id(1);
        assert!(result.is_err()); // DieselRepoMock returns NotFound even for existing
    }

    #[test]
    fn test_projects_repository_get_projects_all_with_data() {
        let mut repo = DieselRepoMock::default();
        let project = create_test_project();
        repo.projects.insert(1, project);
        
        let result = repo.get_projects_all();
        assert!(result.is_ok());
        // DieselRepoMock returns empty vector
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_matrix_repository_get_matrix_by_project_different_id() {
        let mut repo = DieselRepoMock::default();
        let mut matrix = create_test_matrix();
        matrix.project_id = 2;
        repo.matrices.push(matrix);
        
        let result = repo.get_matrix_by_project(2);
        assert!(result.is_ok());
        let matrices = result.unwrap();
        assert_eq!(matrices.len(), 1);
    }

    #[test]
    fn test_multiple_users() {
        let user1 = create_test_user();
        let mut user2 = create_test_user();
        user2.id = 2;
        user2.username = "user2".to_string();
        let repo = DieselRepoMock::with_users(vec![user1, user2]);
        
        let result = repo.get_users_all();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_multiple_requirements() {
        let mut repo = DieselRepoMock::default();
        let req1 = create_test_requirement();
        let mut req2 = create_test_requirement();
        req2.id = 2;
        req2.title = "Requirement 2".to_string();
        
        repo.requirements.insert(1, req1);
        repo.requirements.insert(2, req2);
        
        let result = repo.get_requirements_all();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_multiple_tests() {
        let mut repo = DieselRepoMock::default();
        let test1 = create_test_test();
        let mut test2 = create_test_test();
        test2.id = 2;
        test2.name = "Test 2".to_string();
        
        repo.tests.insert(1, test1);
        repo.tests.insert(2, test2);
        
        let result = repo.get_tests_all();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_requirement_with_parent() {
        let mut repo = DieselRepoMock::default();
        let parent = create_test_requirement();
        let mut child = create_test_requirement();
        child.id = 2;
        child.parent_id = Some(1);
        
        repo.requirements.insert(1, parent);
        repo.requirements.insert(2, child);
        
        let result = repo.get_requirement_by_id(2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().parent_id, Some(1));
    }

    #[test]
    fn test_test_with_parent() {
        let mut repo = DieselRepoMock::default();
        let parent = create_test_test();
        let mut child = create_test_test();
        child.id = 2;
        child.parent_id = Some(1);
        
        repo.tests.insert(1, parent);
        repo.tests.insert(2, child);
        
        let result = repo.get_test_by_id(2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().parent_id, Some(1));
    }

    #[test]
    fn test_matrix_multiple_links() {
        let mut repo = DieselRepoMock::default();
        let matrix1 = create_test_matrix();
        let mut matrix2 = create_test_matrix();
        matrix2.req_id = 2;
        matrix2.id = 2;
        
        repo.matrices.push(matrix1);
        repo.matrices.push(matrix2);
        
        let result = repo.get_matrix_all();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_get_requirements_for_test_multiple() {
        let mut repo = DieselRepoMock::default();
        let req1 = create_test_requirement();
        let mut req2 = create_test_requirement();
        req2.id = 2;
        let test = create_test_test();
        let mut matrix1 = create_test_matrix();
        let mut matrix2 = create_test_matrix();
        matrix2.req_id = 2;
        matrix2.id = 2;
        
        repo.requirements.insert(1, req1);
        repo.requirements.insert(2, req2);
        repo.tests.insert(1, test);
        repo.matrices.push(matrix1);
        repo.matrices.push(matrix2);
        
        let result = repo.get_requirements_for_test(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_get_tests_for_requirement_multiple() {
        let mut repo = DieselRepoMock::default();
        let requirement = create_test_requirement();
        let test1 = create_test_test();
        let mut test2 = create_test_test();
        test2.id = 2;
        let mut matrix1 = create_test_matrix();
        let mut matrix2 = create_test_matrix();
        matrix2.test_id = 2;
        matrix2.id = 2;
        
        repo.requirements.insert(1, requirement);
        repo.tests.insert(1, test1);
        repo.tests.insert(2, test2);
        repo.matrices.push(matrix1);
        repo.matrices.push(matrix2);
        
        let result = repo.get_tests_for_requirement(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_update_user_password_with_existing_user() {
        let user = create_test_user();
        let mut repo = DieselRepoMock::with_users(vec![user]);
        
        let result = repo.update_user_password(1, "new_hash");
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_user_without_password_with_existing_user() {
        let user = create_test_user();
        let mut repo = DieselRepoMock::with_users(vec![user]);
        
        let user_data = UpdateUser {
            id: Some(1),
            username: "updated".to_string(),
            name: "Updated User".to_string(),
            email: "updated@example.com".to_string(),
            user_level: 1,
            is_admin: false,
        };
        
        let result = repo.update_user_without_password(&user_data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_categories_by_project_multiple() {
        let mut repo = DieselRepoMock::default();
        let cat1 = create_test_category();
        let mut cat2 = create_test_category();
        cat2.id = 2;
        cat2.title = "Category 2".to_string();
        
        repo.categories.insert(1, cat1);
        repo.categories.insert(2, cat2);
        
        let result = repo.get_categories_by_project(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_get_applicability_by_project_multiple() {
        let mut repo = DieselRepoMock::default();
        let app1 = create_test_applicability();
        let mut app2 = create_test_applicability();
        app2.id = 2;
        app2.title = "Applicability 2".to_string();
        
        repo.applicability.insert(1, app1);
        repo.applicability.insert(2, app2);
        
        let result = repo.get_applicability_by_project(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_get_verification_by_project_multiple() {
        let mut repo = DieselRepoMock::default();
        let ver1 = create_test_verification();
        let mut ver2 = create_test_verification();
        ver2.id = 2;
        ver2.title = "Verification 2".to_string();
        
        repo.verifications.insert(1, ver1);
        repo.verifications.insert(2, ver2);
        
        let result = repo.get_verification_by_project(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_get_requirements_by_project_multiple() {
        let mut repo = DieselRepoMock::default();
        let req1 = create_test_requirement();
        let mut req2 = create_test_requirement();
        req2.id = 2;
        req2.title = "Requirement 2".to_string();
        
        repo.requirements.insert(1, req1);
        repo.requirements.insert(2, req2);
        
        let result = repo.get_requirements_by_project(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_get_tests_by_project_multiple() {
        let mut repo = DieselRepoMock::default();
        let test1 = create_test_test();
        let mut test2 = create_test_test();
        test2.id = 2;
        test2.name = "Test 2".to_string();
        
        repo.tests.insert(1, test1);
        repo.tests.insert(2, test2);
        
        let result = repo.get_tests_by_project(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_get_matrix_by_project_multiple() {
        let mut repo = DieselRepoMock::default();
        let matrix1 = create_test_matrix();
        let mut matrix2 = create_test_matrix();
        matrix2.id = 2;
        
        repo.matrices.push(matrix1);
        repo.matrices.push(matrix2);
        
        let result = repo.get_matrix_by_project(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_update_test_requirement_links_with_multiple_requirements() {
        let mut repo = DieselRepoMock::default();
        
        let result = repo.update_test_requirement_links(1, &[1, 2, 3, 4, 5]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_test_requirement_links_empty() {
        let mut repo = DieselRepoMock::default();
        
        let result = repo.update_test_requirement_links(1, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_insert_new_verification() {
        let mut repo = DieselRepoMock::default();
        let new_verification = NewVerificationMethod {
            id: None,
            title: "New Verification".to_string(),
            description: "New Verification Description".to_string(),
            tag: "NEW".to_string(),
            project_id: 1,
        };
        
        let result = repo.insert_new_verification(&new_verification);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_verification_by_id_with_existing() {
        let mut repo = DieselRepoMock::default();
        let verification = create_test_verification();
        repo.verifications.insert(1, verification);
        
        let result = repo.get_verification_by_id(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().title, "Test Verification");
    }

    #[test]
    fn test_get_test_status_by_id_with_existing() {
        let mut repo = DieselRepoMock::default();
        let status = create_test_test_status();
        repo.test_statuses.insert(1, status);
        
        let result = repo.get_test_status_by_id(1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().title, "Draft");
    }

    #[test]
    fn test_get_test_status_all_with_data() {
        let mut repo = DieselRepoMock::default();
        let status = create_test_test_status();
        repo.test_statuses.insert(1, status);
        
        let result = repo.get_test_status_all();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_get_requirement_status_all_with_multiple() {
        let mut repo = DieselRepoMock::default();
        let status1 = create_test_requirement_status();
        let mut status2 = create_test_requirement_status();
        status2.id = 2;
        status2.title = "Status 2".to_string();
        
        repo.requirement_statuses.insert(1, status1);
        repo.requirement_statuses.insert(2, status2);
        
        let result = repo.get_requirement_status_all();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_get_test_status_all_with_multiple() {
        let mut repo = DieselRepoMock::default();
        let status1 = create_test_test_status();
        let mut status2 = create_test_test_status();
        status2.id = 2;
        status2.title = "Status 2".to_string();
        
        repo.test_statuses.insert(1, status1);
        repo.test_statuses.insert(2, status2);
        
        let result = repo.get_test_status_all();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_get_categories_all_with_multiple() {
        let mut repo = DieselRepoMock::default();
        let cat1 = create_test_category();
        let mut cat2 = create_test_category();
        cat2.id = 2;
        cat2.title = "Category 2".to_string();
        
        repo.categories.insert(1, cat1);
        repo.categories.insert(2, cat2);
        
        let result = repo.get_categories_all();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_get_applicability_all_with_multiple() {
        let mut repo = DieselRepoMock::default();
        let app1 = create_test_applicability();
        let mut app2 = create_test_applicability();
        app2.id = 2;
        app2.title = "Applicability 2".to_string();
        
        repo.applicability.insert(1, app1);
        repo.applicability.insert(2, app2);
        
        let result = repo.get_applicability_all();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[test]
    fn test_get_verification_all_with_multiple() {
        let mut repo = DieselRepoMock::default();
        let ver1 = create_test_verification();
        let mut ver2 = create_test_verification();
        ver2.id = 2;
        ver2.title = "Verification 2".to_string();
        
        repo.verifications.insert(1, ver1);
        repo.verifications.insert(2, ver2);
        
        let result = repo.get_verification_all();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }
}

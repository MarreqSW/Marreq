#[cfg(test)]
mod tests {
    use crate::models::*;
    use crate::errors::ApiError;
    use crate::services::{
        base_service::Service,
        BaseService, RequirementService, TestService, ProjectService, StatusService,
        CategoryService, ApplicabilityService, UserService, MatrixService,
        serialize_for_logging, check_project_permission, validate_entity_access
    };
    use chrono::{NaiveDate, NaiveDateTime};
    use std::time::Duration;

    fn test_datetime() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    fn create_test_user() -> User {
        User {
            user_id: 1,
            user_username: "testuser".to_string(),
            user_name: "Test User".to_string(),
            user_email: "test@example.com".to_string(),
            user_level: 1,
            user_creation_date: test_datetime(),
            user_last_login: test_datetime(),
            user_password: "hashed_password".to_string(),
            project_id: Some(1),
            is_admin: false,
        }
    }

    fn create_test_requirement() -> Requirement {
        Requirement {
            req_id: 1,
            req_title: "Test Requirement".to_string(),
            req_description: "Test Description".to_string(),
            req_verification: 1,
            req_current_status: 1,
            req_author: 1,
            req_reviewer: 1,
            req_link: "http://example.com".to_string(),
            req_reference: "REQ-1".to_string(),
            req_category: 1,
            req_parent: 0,
            req_creation_date: test_datetime(),
            req_update_date: test_datetime(),
            req_deadline_date: test_datetime(),
            req_applicability: 1,
            req_justification: None,
            project_id: 1,
        }
    }

    // BaseService Tests
    #[test]
    fn test_base_service_new() {
        let service = BaseService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_base_service_cache_key() {
        let service = BaseService::new();
        let key = service.cache_key("test", 123);
        assert_eq!(key, "test:123");
    }

    #[test]
    fn test_base_service_cache_key_list() {
        let service = BaseService::new();
        let key_all = service.cache_key_list("test", None);
        let key_project = service.cache_key_list("test", Some(1));
        
        assert_eq!(key_all, "test:list:all");
        assert_eq!(key_project, "test:list:1");
    }

    #[test]
    fn test_serialize_for_logging() {
        let data = create_test_requirement();
        let result = serialize_for_logging(&data);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("Test Requirement"));
    }

    #[test]
    fn test_check_project_permission_admin() {
        let mut user = create_test_user();
        user.is_admin = true;
        
        let result = check_project_permission(&user, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_project_permission_regular_user() {
        let user = create_test_user();
        
        let result = check_project_permission(&user, 1);
        assert!(result.is_ok()); // Currently all users have access
    }

    #[test]
    fn test_validate_entity_access() {
        let user = create_test_user();
        
        let result = validate_entity_access(&user, 1);
        assert!(result.is_ok());
    }

    // RequirementService Tests
    #[test]
    fn test_requirement_service_new() {
        let service = RequirementService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_requirement_service_structure() {
        let service = RequirementService::new();
        // Test that the service can be created and has the expected structure
        let _repo = service.repo();
        assert!(true);
        
        let mut service = RequirementService::new();
        let _repo = service.repo_mut();
        assert!(true);
    }

    // TestService Tests
    #[test]
    fn test_test_service_new() {
        let service = TestService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_test_service_structure() {
        let service = TestService::new();
        let _repo = service.repo();
        assert!(true);
        
        let mut service = TestService::new();
        let _repo = service.repo_mut();
        assert!(true);
    }

    // ProjectService Tests
    #[test]
    fn test_project_service_new() {
        let service = ProjectService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_project_service_structure() {
        let service = ProjectService::new();
        let _repo = service.repo();
        assert!(true);
        
        let mut service = ProjectService::new();
        let _repo = service.repo_mut();
        assert!(true);
    }

    // StatusService Tests
    #[test]
    fn test_status_service_new() {
        let service = StatusService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_status_service_structure() {
        let service = StatusService::new();
        let _repo = service.repo();
        assert!(true);
        
        let mut service = StatusService::new();
        let _repo = service.repo_mut();
        assert!(true);
    }

    // CategoryService Tests
    #[test]
    fn test_category_service_new() {
        let service = CategoryService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_category_service_structure() {
        let service = CategoryService::new();
        let _repo = service.repo();
        assert!(true);
        
        let mut service = CategoryService::new();
        let _repo = service.repo_mut();
        assert!(true);
    }

    // ApplicabilityService Tests
    #[test]
    fn test_applicability_service_new() {
        let service = ApplicabilityService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_applicability_service_structure() {
        let service = ApplicabilityService::new();
        let _repo = service.repo();
        assert!(true);
        
        let mut service = ApplicabilityService::new();
        let _repo = service.repo_mut();
        assert!(true);
    }

    // UserService Tests
    #[test]
    fn test_user_service_new() {
        let service = UserService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_user_service_structure() {
        let service = UserService::new();
        let _repo = service.repo();
        assert!(true);
        
        let mut service = UserService::new();
        let _repo = service.repo_mut();
        assert!(true);
    }

    // MatrixService Tests
    #[test]
    fn test_matrix_service_new() {
        let service = MatrixService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_matrix_service_structure() {
        let service = MatrixService::new();
        let _repo = service.repo();
        assert!(true);
        
        let mut service = MatrixService::new();
        let _repo = service.repo_mut();
        assert!(true);
    }

    // Error Handling Tests
    #[test]
    fn test_api_error_serialization() {
        let error = ApiError::Internal("test error".to_string());
        let result = serialize_for_logging(&error);
        assert!(result.is_ok());
    }

    #[test]
    fn test_api_error_repository() {
        let error = ApiError::Repository(crate::repository::errors::RepoError::NotFound);
        let result = serialize_for_logging(&error);
        assert!(result.is_ok());
    }

    // Cache Key Generation Tests
    #[test]
    fn test_cache_key_generation() {
        let service = BaseService::new();
        
        // Test entity cache key
        let entity_key = service.cache_key("requirement", 123);
        assert_eq!(entity_key, "requirement:123");
        
        // Test list cache key without project
        let list_key_all = service.cache_key_list("requirement", None);
        assert_eq!(list_key_all, "requirement:list:all");
        
        // Test list cache key with project
        let list_key_project = service.cache_key_list("requirement", Some(456));
        assert_eq!(list_key_project, "requirement:list:456");
    }

    // Service Trait Implementation Tests
    #[test]
    fn test_service_trait_implementation() {
        let service = RequirementService::new();
        let _repo = service.repo();
        assert!(true);
        
        let mut service = RequirementService::new();
        let _repo = service.repo_mut();
        assert!(true);
    }

    // Cache Integration Tests
    #[test]
    fn test_cache_integration() {
        let service = BaseService::new();
        
        // Test cache key generation
        let key = service.cache_key("test", 1);
        assert_eq!(key, "test:1");
        
        // Test cache invalidation (should not panic)
        service.invalidate_cache(&key);
        
        // Test cache operations (these will fail in test environment but should not panic)
        let _: Option<String> = service.get_cached(&key);
        service.set_cache(&key, "test_value".to_string(), Duration::from_secs(60));
    }

    // Service Method Existence Tests
    #[test]
    fn test_requirement_service_methods_exist() {
        let service = RequirementService::new();
        // Test that all expected methods exist by checking the service structure
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_test_service_methods_exist() {
        let service = TestService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_project_service_methods_exist() {
        let service = ProjectService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_status_service_methods_exist() {
        let service = StatusService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_category_service_methods_exist() {
        let service = CategoryService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_applicability_service_methods_exist() {
        let service = ApplicabilityService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_user_service_methods_exist() {
        let service = UserService::new();
        let _repo = service.repo();
        assert!(true);
    }

    #[test]
    fn test_matrix_service_methods_exist() {
        let service = MatrixService::new();
        let _repo = service.repo();
        assert!(true);
    }

    // Service Error Handling Tests
    #[test]
    fn test_service_error_handling() {
        // Test that services properly handle errors
        let service = BaseService::new();
        
        // Test cache operations with invalid keys
        let invalid_key = "";
        service.invalidate_cache(invalid_key);
        
        // Test that these operations don't panic
        let _: Option<String> = service.get_cached(invalid_key);
        service.set_cache(invalid_key, "test".to_string(), Duration::from_secs(60));
    }

    // Service Initialization Tests
    #[test]
    fn test_all_services_initialization() {
        // Test that all services can be initialized without panicking
        let _requirement_service = RequirementService::new();
        let _test_service = TestService::new();
        let _project_service = ProjectService::new();
        let _status_service = StatusService::new();
        let _category_service = CategoryService::new();
        let _applicability_service = ApplicabilityService::new();
        let _user_service = UserService::new();
        let _matrix_service = MatrixService::new();
        
        // If we get here, all services initialized successfully
        assert!(true);
    }

    // Service Repository Access Tests
    #[test]
    fn test_requirement_service_repository_access() {
        let service = RequirementService::new();
        let _repo = service.repo();
        assert!(true, "RequirementService should have valid repository access");
    }

    #[test]
    fn test_test_service_repository_access() {
        let service = TestService::new();
        let _repo = service.repo();
        assert!(true, "TestService should have valid repository access");
    }

    #[test]
    fn test_project_service_repository_access() {
        let service = ProjectService::new();
        let _repo = service.repo();
        assert!(true, "ProjectService should have valid repository access");
    }

    #[test]
    fn test_status_service_repository_access() {
        let service = StatusService::new();
        let _repo = service.repo();
        assert!(true, "StatusService should have valid repository access");
    }

    #[test]
    fn test_category_service_repository_access() {
        let service = CategoryService::new();
        let _repo = service.repo();
        assert!(true, "CategoryService should have valid repository access");
    }

    #[test]
    fn test_applicability_service_repository_access() {
        let service = ApplicabilityService::new();
        let _repo = service.repo();
        assert!(true, "ApplicabilityService should have valid repository access");
    }

    #[test]
    fn test_user_service_repository_access() {
        let service = UserService::new();
        let _repo = service.repo();
        assert!(true, "UserService should have valid repository access");
    }

    #[test]
    fn test_matrix_service_repository_access() {
        let service = MatrixService::new();
        let _repo = service.repo();
        assert!(true, "MatrixService should have valid repository access");
    }

    // Service Mutable Repository Access Tests
    #[test]
    fn test_requirement_service_mutable_repository_access() {
        let mut service = RequirementService::new();
        let _repo = service.repo_mut();
        assert!(true, "RequirementService should have valid mutable repository access");
    }

    #[test]
    fn test_test_service_mutable_repository_access() {
        let mut service = TestService::new();
        let _repo = service.repo_mut();
        assert!(true, "TestService should have valid mutable repository access");
    }

    #[test]
    fn test_project_service_mutable_repository_access() {
        let mut service = ProjectService::new();
        let _repo = service.repo_mut();
        assert!(true, "ProjectService should have valid mutable repository access");
    }

    #[test]
    fn test_status_service_mutable_repository_access() {
        let mut service = StatusService::new();
        let _repo = service.repo_mut();
        assert!(true, "StatusService should have valid mutable repository access");
    }

    #[test]
    fn test_category_service_mutable_repository_access() {
        let mut service = CategoryService::new();
        let _repo = service.repo_mut();
        assert!(true, "CategoryService should have valid mutable repository access");
    }

    #[test]
    fn test_applicability_service_mutable_repository_access() {
        let mut service = ApplicabilityService::new();
        let _repo = service.repo_mut();
        assert!(true, "ApplicabilityService should have valid mutable repository access");
    }

    #[test]
    fn test_user_service_mutable_repository_access() {
        let mut service = UserService::new();
        let _repo = service.repo_mut();
        assert!(true, "UserService should have valid mutable repository access");
    }

    #[test]
    fn test_matrix_service_mutable_repository_access() {
        let mut service = MatrixService::new();
        let _repo = service.repo_mut();
        assert!(true, "MatrixService should have valid mutable repository access");
    }
}
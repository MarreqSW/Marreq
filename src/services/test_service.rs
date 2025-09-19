//! Test service for managing test cases business logic.
//!
//! This service handles all test-related operations including CRUD operations,
//! validation, caching, and audit logging.

use crate::errors::{ApiError, ApiResult};
use crate::models::*;
use crate::validation::validate_test;
use crate::services::{BaseService, Service};
use crate::repository::TestsRepository;
use std::time::Duration;

/// Service for managing test cases
pub struct TestService {
    base: BaseService,
}

impl TestService {
    /// Create a new test service
    pub fn new() -> Self {
        Self {
            base: BaseService::new(),
        }
    }
    
    /// Get all tests
    pub async fn get_all_tests(&self) -> ApiResult<Vec<Test>> {
        let cache_key = self.base.cache_key_list("test", None);
        
        // Try to get from cache first
        if let Some(cached) = self.base.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        // Get from database
        let tests = self.base.repo()
            .get_tests_all()
            .map_err(|e| ApiError::Repository(e))?;
        
        // Cache the result
        self.base.set_cache(&cache_key, tests.clone(), Duration::from_secs(300));
        
        Ok(tests)
    }
    
    /// Get tests by project
    pub async fn get_tests_by_project(&self, project_id: i32) -> ApiResult<Vec<Test>> {
        let cache_key = self.base.cache_key_list("test", Some(project_id));
        
        // Try to get from cache first
        if let Some(cached) = self.base.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        // Get from database
        let tests = self.base.repo()
            .get_tests_by_project(project_id)
            .map_err(|e| ApiError::Repository(e))?;
        
        // Cache the result
        self.base.set_cache(&cache_key, tests.clone(), Duration::from_secs(300));
        
        Ok(tests)
    }
    
    /// Get a test by ID
    pub async fn get_test_by_id(&self, id: i32) -> ApiResult<Test> {
        let cache_key = self.base.cache_key("test", id);
        
        // Try to get from cache first
        if let Some(cached) = self.base.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        // Get from database
        let test = self.base.repo()
            .get_test_by_id(id)
            .map_err(|e| ApiError::Repository(e))?;
        
        // Cache the result
        self.base.set_cache(&cache_key, test.clone(), Duration::from_secs(600));
        
        Ok(test)
    }
    
    /// Create a new test
    pub async fn create_test(
        &self,
        mut new_test: NewTest,
        user_id: i32,
    ) -> ApiResult<i32> {
        // Validate input
        validate_test(&new_test)?;
        
        // Sanitize input
        crate::validation::sanitize_string(&mut new_test.test_name);
        crate::validation::sanitize_string(&mut new_test.test_description);
        crate::validation::sanitize_string(&mut new_test.test_source);
        
        // Insert into database
        let mut repo = crate::repository::DieselRepo::new();
        let id = repo.insert_test(&new_test)
            .map_err(|e| ApiError::Repository(e))?;
        
        // Log the creation
        if let Ok(new_values) = crate::services::serialize_for_logging(&new_test) {
            let _ = self.base.log_create(
                user_id,
                EntityType::Test,
                id,
                Some(new_test.project_id),
                Some(new_values),
                Some(format!("Created test: {}", new_test.test_name)),
            );
        }
        
        // Invalidate relevant caches
        self.base.invalidate_cache(&self.base.cache_key_list("test", None));
        self.base.invalidate_cache(&self.base.cache_key_list("test", Some(new_test.project_id)));
        crate::cache::invalidate_test_cache(id);
        crate::cache::invalidate_project_cache(new_test.project_id);
        
        Ok(id)
    }
    
    /// Update an existing test
    pub async fn update_test(
        &self,
        id: i32,
        mut updated_test: NewTest,
        user_id: i32,
    ) -> ApiResult<bool> {
        // Get existing test for logging
        let old_test = self.get_test_by_id(id).await?;
        
        // Validate input
        validate_test(&updated_test)?;
        
        // Sanitize input
        crate::validation::sanitize_string(&mut updated_test.test_name);
        crate::validation::sanitize_string(&mut updated_test.test_description);
        crate::validation::sanitize_string(&mut updated_test.test_source);
        
        // Set the ID for update
        updated_test.test_id = Some(id);
        
        // Update in database
        let mut repo = crate::repository::DieselRepo::new();
        let success = repo.edit_test(&updated_test)
            .map_err(|e| ApiError::Repository(e))?;
        
        if success {
            // Log the update
            if let (Ok(old_values), Ok(new_values)) = (
                crate::services::serialize_for_logging(&old_test),
                crate::services::serialize_for_logging(&updated_test),
            ) {
                let _ = self.base.log_update(
                    user_id,
                    EntityType::Test,
                    id,
                    Some(updated_test.project_id),
                    Some(old_values),
                    Some(new_values),
                    Some(format!("Updated test: {}", updated_test.test_name)),
                );
            }
            
            // Invalidate relevant caches
            self.base.invalidate_cache(&self.base.cache_key("test", id));
            self.base.invalidate_cache(&self.base.cache_key_list("test", None));
            self.base.invalidate_cache(&self.base.cache_key_list("test", Some(updated_test.project_id)));
            crate::cache::invalidate_test_cache(id);
            crate::cache::invalidate_project_cache(updated_test.project_id);
        }
        
        Ok(success)
    }
    
    /// Delete a test
    pub async fn delete_test(
        &self,
        id: i32,
        user_id: i32,
    ) -> ApiResult<bool> {
        // Get existing test for logging
        let old_test = self.get_test_by_id(id).await?;
        
        // Delete from database
        let mut repo = crate::repository::DieselRepo::new();
        let success = repo.delete_test(id)
            .map_err(|e| ApiError::Repository(e))?;
        
        if success {
            // Log the deletion
            if let Ok(old_values) = crate::services::serialize_for_logging(&old_test) {
                let _ = self.base.log_delete(
                    user_id,
                    EntityType::Test,
                    id,
                    Some(old_test.project_id),
                    Some(old_values),
                    Some(format!("Deleted test: {}", old_test.test_name)),
                );
            }
            
            // Invalidate relevant caches
            self.base.invalidate_cache(&self.base.cache_key("test", id));
            self.base.invalidate_cache(&self.base.cache_key_list("test", None));
            self.base.invalidate_cache(&self.base.cache_key_list("test", Some(old_test.project_id)));
            crate::cache::invalidate_test_cache(id);
            crate::cache::invalidate_project_cache(old_test.project_id);
        }
        
        Ok(success)
    }
    
    /// Get tests by status
    pub async fn get_tests_by_status(&self, status_id: i32) -> ApiResult<Vec<Test>> {
        let cache_key = format!("test:status:{}", status_id);
        
        // Try to get from cache first
        if let Some(cached) = self.base.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        // Get from database
        let tests = self.base.repo()
            .get_tests_by_status(status_id)
            .map_err(|e| ApiError::Repository(e))?;
        
        // Cache the result
        self.base.set_cache(&cache_key, tests.clone(), Duration::from_secs(300));
        
        Ok(tests)
    }
    
    /// Get tests by parent (hierarchical structure)
    pub async fn get_tests_by_parent(&self, parent_id: i32) -> ApiResult<Vec<Test>> {
        let cache_key = format!("test:parent:{}", parent_id);
        
        // Try to get from cache first
        if let Some(cached) = self.base.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        // Get from database
        let tests = self.base.repo()
            .get_tests_by_parent(parent_id)
            .map_err(|e| ApiError::Repository(e))?;
        
        // Cache the result
        self.base.set_cache(&cache_key, tests.clone(), Duration::from_secs(300));
        
        Ok(tests)
    }
}

impl Service for TestService {
    fn repo(&self) -> &crate::repository::DieselRepo {
        self.base.repo()
    }
    
    fn repo_mut(&mut self) -> &mut crate::repository::DieselRepo {
        self.base.repo_mut()
    }
}




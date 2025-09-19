//! Status service for managing status options business logic.

use crate::errors::{ApiError, ApiResult};
use crate::models::*;
use crate::validation::{validate_requirement_status, validate_test_status};
use crate::services::{BaseService, Service};
use crate::repository::LookupRepository;
use std::time::Duration;

pub struct StatusService {
    base: BaseService,
}

impl StatusService {
    pub fn new() -> Self {
        Self { base: BaseService::new() }
    }
    
    pub async fn get_all_requirement_status(&self) -> ApiResult<Vec<RequirementStatus>> {
        let cache_key = self.base.cache_key_list("requirement_status", None);
        if let Some(cached) = self.base.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        let status = self.base.repo()
            .get_requirement_status_all()
            .map_err(|e| ApiError::Repository(e))?;
        
        self.base.set_cache(&cache_key, status.clone(), Duration::from_secs(300));
        Ok(status)
    }

    pub async fn get_all_test_status(&self) -> ApiResult<Vec<TestStatus>> {
        let cache_key = self.base.cache_key_list("test_status", None);
        if let Some(cached) = self.base.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        let status = self.base.repo()
            .get_test_status_all()
            .map_err(|e| ApiError::Repository(e))?;
        
        self.base.set_cache(&cache_key, status.clone(), Duration::from_secs(300));
        Ok(status)
    }
    
    pub async fn create_requirement_status(
        &self,
        mut new_status: NewRequirementStatus,
        user_id: i32,
    ) -> ApiResult<i32> {
        validate_requirement_status(&new_status)?;
        
        crate::validation::sanitize_string(&mut new_status.req_st_title);
        crate::validation::sanitize_string(&mut new_status.req_st_description);
        
        let mut repo = crate::repository::DieselRepo::new();
        let id = repo.create_requirement_status(&new_status)
            .map_err(|e| ApiError::Repository(e))?;
        
        if let Ok(new_values) = crate::services::serialize_for_logging(&new_status) {
            let _ = self.base.log_create(
                user_id,
                EntityType::Status,
                id,
                None, // Status doesn't have project_id
                Some(new_values),
                Some(format!("Created requirement status: {}", new_status.req_st_title)),
            );
        }
        
        self.base.invalidate_cache(&self.base.cache_key_list("requirement_status", None));
        crate::cache::invalidate_status_cache(id);
        
        Ok(id)
    }

    pub async fn create_test_status(
        &self,
        mut new_status: NewTestStatus,
        user_id: i32,
    ) -> ApiResult<i32> {
        validate_test_status(&new_status)?;
        
        crate::validation::sanitize_string(&mut new_status.test_st_title);
        crate::validation::sanitize_string(&mut new_status.test_st_description);
        
        let mut repo = crate::repository::DieselRepo::new();
        let id = repo.create_test_status(&new_status)
            .map_err(|e| ApiError::Repository(e))?;
        
        if let Ok(new_values) = crate::services::serialize_for_logging(&new_status) {
            let _ = self.base.log_create(
                user_id,
                EntityType::Status,
                id,
                None, // Status doesn't have project_id
                Some(new_values),
                Some(format!("Created test status: {}", new_status.test_st_title)),
            );
        }
        
        self.base.invalidate_cache(&self.base.cache_key_list("test_status", None));
        crate::cache::invalidate_status_cache(id);
        
        Ok(id)
    }
}

impl Service for StatusService {
    fn repo(&self) -> &crate::repository::DieselRepo { self.base.repo() }
    fn repo_mut(&mut self) -> &mut crate::repository::DieselRepo { self.base.repo_mut() }
}



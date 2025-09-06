//! Status service for managing status options business logic.

use crate::errors::{ApiError, ApiResult};
use crate::models::*;
use crate::validation::validate_status;
use crate::services::{BaseService, Service, CacheableService};
use std::time::Duration;

pub struct StatusService {
    base: BaseService,
}

impl StatusService {
    pub fn new() -> Self {
        Self { base: BaseService::new() }
    }
    
    pub async fn get_all_status(&self) -> ApiResult<Vec<Status>> {
        let cache_key = self.cache_key_list("status", None);
        if let Some(cached) = self.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        let status = self.base.repo()
            .get_status_all()
            .map_err(|e| ApiError::Database(e))?;
        
        self.set_cache(&cache_key, status.clone(), Duration::from_secs(300));
        Ok(status)
    }
    
    pub async fn create_status(
        &self,
        mut new_status: NewStatus,
        user_id: i32,
    ) -> ApiResult<i32> {
        validate_status(&new_status)?;
        
        crate::validation::sanitize_string(&mut new_status.st_title);
        crate::validation::sanitize_optional_string(&mut new_status.st_description);
        
        let id = self.base.repo()
            .create_status(&new_status)
            .map_err(|e| ApiError::Database(e))?;
        
        if let Ok(new_values) = crate::services::serialize_for_logging(&new_status) {
            let _ = self.base.log_create(
                user_id,
                EntityType::Status,
                id,
                None, // Status doesn't have project_id
                Some(new_values),
                Some(format!("Created status: {}", new_status.st_title)),
            );
        }
        
        self.invalidate_cache(&self.cache_key_list("status", None));
        crate::cache::invalidate_status_cache(id);
        
        Ok(id)
    }
}

impl Service for StatusService {
    fn repo(&self) -> &crate::repository::DieselRepo { self.base.repo() }
    fn repo_mut(&mut self) -> &mut crate::repository::DieselRepo { self.base.repo_mut() }
}

impl CacheableService<Vec<Status>> for StatusService {}

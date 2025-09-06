//! Applicability service for managing applicability options business logic.

use crate::errors::{ApiError, ApiResult};
use crate::models::*;
use crate::validation::validate_applicability;
use crate::services::{BaseService, Service, CacheableService};
use std::time::Duration;

pub struct ApplicabilityService {
    base: BaseService,
}

impl ApplicabilityService {
    pub fn new() -> Self {
        Self { base: BaseService::new() }
    }
    
    pub async fn get_all_applicability(&self) -> ApiResult<Vec<Applicability>> {
        let cache_key = self.cache_key_list("applicability", None);
        if let Some(cached) = self.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        let applicability = self.base.repo()
            .get_applicability_all()
            .map_err(|e| ApiError::Database(e))?;
        
        self.set_cache(&cache_key, applicability.clone(), Duration::from_secs(300));
        Ok(applicability)
    }
    
    pub async fn get_applicability_by_id(&self, id: i32) -> ApiResult<Applicability> {
        let cache_key = self.cache_key("applicability", id);
        if let Some(cached) = self.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        let applicability = self.base.repo()
            .get_applicability_by_id(id)
            .map_err(|e| ApiError::Database(e))?;
        
        self.set_cache(&cache_key, applicability.clone(), Duration::from_secs(600));
        Ok(applicability)
    }
    
    pub async fn create_applicability(
        &self,
        mut new_applicability: NewApplicability,
        user_id: i32,
    ) -> ApiResult<i32> {
        validate_applicability(&new_applicability)?;
        
        crate::validation::sanitize_string(&mut new_applicability.app_title);
        crate::validation::sanitize_string(&mut new_applicability.app_description);
        crate::validation::sanitize_string(&mut new_applicability.app_tag);
        
        let id = self.base.repo()
            .insert_new_applicability(&new_applicability)
            .map_err(|e| ApiError::Database(e))?;
        
        if let Ok(new_values) = crate::services::serialize_for_logging(&new_applicability) {
            let _ = self.base.log_create(
                user_id,
                EntityType::Applicability,
                id,
                Some(new_applicability.project_id),
                Some(new_values),
                Some(format!("Created applicability: {}", new_applicability.app_title)),
            );
        }
        
        self.invalidate_cache(&self.cache_key_list("applicability", None));
        self.invalidate_cache(&self.cache_key_list("applicability", Some(new_applicability.project_id)));
        crate::cache::invalidate_applicability_cache(id);
        crate::cache::invalidate_project_cache(new_applicability.project_id);
        
        Ok(id)
    }
    
    pub async fn update_applicability(
        &self,
        id: i32,
        mut updated_applicability: NewApplicability,
        user_id: i32,
    ) -> ApiResult<bool> {
        let old_applicability = self.get_applicability_by_id(id).await?;
        validate_applicability(&updated_applicability)?;
        
        crate::validation::sanitize_string(&mut updated_applicability.app_title);
        crate::validation::sanitize_string(&mut updated_applicability.app_description);
        crate::validation::sanitize_string(&mut updated_applicability.app_tag);
        
        updated_applicability.app_id = Some(id);
        
        let success = self.base.repo()
            .edit_applicability(&updated_applicability)
            .map_err(|e| ApiError::Database(e))?;
        
        if success {
            if let (Ok(old_values), Ok(new_values)) = (
                crate::services::serialize_for_logging(&old_applicability),
                crate::services::serialize_for_logging(&updated_applicability),
            ) {
                let _ = self.base.log_update(
                    user_id,
                    EntityType::Applicability,
                    id,
                    Some(updated_applicability.project_id),
                    Some(old_values),
                    Some(new_values),
                    Some(format!("Updated applicability: {}", updated_applicability.app_title)),
                );
            }
            
            self.invalidate_cache(&self.cache_key("applicability", id));
            self.invalidate_cache(&self.cache_key_list("applicability", None));
            self.invalidate_cache(&self.cache_key_list("applicability", Some(updated_applicability.project_id)));
            crate::cache::invalidate_applicability_cache(id);
            crate::cache::invalidate_project_cache(updated_applicability.project_id);
        }
        
        Ok(success)
    }
    
    pub async fn delete_applicability(&self, id: i32, user_id: i32) -> ApiResult<bool> {
        let old_applicability = self.get_applicability_by_id(id).await?;
        
        let success = self.base.repo()
            .delete_applicability(id)
            .map_err(|e| ApiError::Database(e))?;
        
        if success {
            if let Ok(old_values) = crate::services::serialize_for_logging(&old_applicability) {
                let _ = self.base.log_delete(
                    user_id,
                    EntityType::Applicability,
                    id,
                    Some(old_applicability.project_id),
                    Some(old_values),
                    Some(format!("Deleted applicability: {}", old_applicability.app_title)),
                );
            }
            
            self.invalidate_cache(&self.cache_key("applicability", id));
            self.invalidate_cache(&self.cache_key_list("applicability", None));
            self.invalidate_cache(&self.cache_key_list("applicability", Some(old_applicability.project_id)));
            crate::cache::invalidate_applicability_cache(id);
            crate::cache::invalidate_project_cache(old_applicability.project_id);
        }
        
        Ok(success)
    }
}

impl Service for ApplicabilityService {
    fn repo(&self) -> &crate::repository::DieselRepo { self.base.repo() }
    fn repo_mut(&mut self) -> &mut crate::repository::DieselRepo { self.base.repo_mut() }
}

impl CacheableService<Vec<Applicability>> for ApplicabilityService {}
impl CacheableService<Applicability> for ApplicabilityService {}

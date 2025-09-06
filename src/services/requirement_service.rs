//! Requirement service for managing requirements business logic.
//!
//! This service handles all requirement-related operations including CRUD operations,
//! validation, caching, and audit logging.

use crate::errors::{ApiError, ApiResult};
use crate::models::*;
use crate::validation::validate_requirement;
use crate::services::{BaseService, Service, CacheableService};
use std::time::Duration;

/// Service for managing requirements
pub struct RequirementService {
    base: BaseService,
}

impl RequirementService {
    /// Create a new requirement service
    pub fn new() -> Self {
        Self {
            base: BaseService::new(),
        }
    }
    
    /// Get all requirements
    pub async fn get_all_requirements(&self) -> ApiResult<Vec<Requirement>> {
        let cache_key = self.cache_key_list("requirement", None);
        
        // Try to get from cache first
        if let Some(cached) = self.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        // Get from database
        let requirements = self.base.repo()
            .get_requirements_all()
            .map_err(|e| ApiError::Database(e))?;
        
        // Cache the result
        self.set_cache(&cache_key, requirements.clone(), Duration::from_secs(300));
        
        Ok(requirements)
    }
    
    /// Get requirements by project
    pub async fn get_requirements_by_project(&self, project_id: i32) -> ApiResult<Vec<Requirement>> {
        let cache_key = self.cache_key_list("requirement", Some(project_id));
        
        // Try to get from cache first
        if let Some(cached) = self.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        // Get from database
        let requirements = self.base.repo()
            .get_requirements_by_project(project_id)
            .map_err(|e| ApiError::Database(e))?;
        
        // Cache the result
        self.set_cache(&cache_key, requirements.clone(), Duration::from_secs(300));
        
        Ok(requirements)
    }
    
    /// Get a requirement by ID
    pub async fn get_requirement_by_id(&self, id: i32) -> ApiResult<Requirement> {
        let cache_key = self.cache_key("requirement", id);
        
        // Try to get from cache first
        if let Some(cached) = self.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        // Get from database
        let requirement = self.base.repo()
            .get_requirement_by_id(id)
            .map_err(|e| ApiError::Database(e))?;
        
        // Cache the result
        self.set_cache(&cache_key, requirement.clone(), Duration::from_secs(600));
        
        Ok(requirement)
    }
    
    /// Create a new requirement
    pub async fn create_requirement(
        &self,
        mut new_req: NewRequirement,
        user_id: i32,
    ) -> ApiResult<i32> {
        // Validate input
        validate_requirement(&new_req)?;
        
        // Sanitize input
        crate::validation::sanitize_string(&mut new_req.req_title);
        crate::validation::sanitize_string(&mut new_req.req_description);
        crate::validation::sanitize_string(&mut new_req.req_reference);
        crate::validation::sanitize_string(&mut new_req.req_link);
        crate::validation::sanitize_optional_string(&mut new_req.req_justification);
        
        // Set timestamps
        let now = chrono::Utc::now().naive_utc();
        new_req.req_creation_date = Some(now);
        new_req.req_update_date = Some(now);
        
        // Insert into database
        let id = self.base.repo()
            .insert_new_requirement(&new_req)
            .map_err(|e| ApiError::Database(e))?;
        
        // Log the creation
        if let Ok(new_values) = crate::services::serialize_for_logging(&new_req) {
            let _ = self.base.log_create(
                user_id,
                EntityType::Requirement,
                id,
                Some(new_req.project_id),
                Some(new_values),
                Some(format!("Created requirement: {}", new_req.req_title)),
            );
        }
        
        // Invalidate relevant caches
        self.invalidate_cache(&self.cache_key_list("requirement", None));
        self.invalidate_cache(&self.cache_key_list("requirement", Some(new_req.project_id)));
        crate::cache::invalidate_requirement_cache(id);
        crate::cache::invalidate_project_cache(new_req.project_id);
        
        Ok(id)
    }
    
    /// Update an existing requirement
    pub async fn update_requirement(
        &self,
        id: i32,
        mut updated_req: NewRequirement,
        user_id: i32,
    ) -> ApiResult<bool> {
        // Get existing requirement for logging
        let old_req = self.get_requirement_by_id(id).await?;
        
        // Validate input
        validate_requirement(&updated_req)?;
        
        // Sanitize input
        crate::validation::sanitize_string(&mut updated_req.req_title);
        crate::validation::sanitize_string(&mut updated_req.req_description);
        crate::validation::sanitize_string(&mut updated_req.req_reference);
        crate::validation::sanitize_string(&mut updated_req.req_link);
        crate::validation::sanitize_optional_string(&mut updated_req.req_justification);
        
        // Set update timestamp
        updated_req.req_update_date = Some(chrono::Utc::now().naive_utc());
        updated_req.req_id = Some(id);
        
        // Update in database
        let success = self.base.repo()
            .edit_requirement(&updated_req)
            .map_err(|e| ApiError::Database(e))?;
        
        if success {
            // Log the update
            if let (Ok(old_values), Ok(new_values)) = (
                crate::services::serialize_for_logging(&old_req),
                crate::services::serialize_for_logging(&updated_req),
            ) {
                let _ = self.base.log_update(
                    user_id,
                    EntityType::Requirement,
                    id,
                    Some(updated_req.project_id),
                    Some(old_values),
                    Some(new_values),
                    Some(format!("Updated requirement: {}", updated_req.req_title)),
                );
            }
            
            // Invalidate relevant caches
            self.invalidate_cache(&self.cache_key("requirement", id));
            self.invalidate_cache(&self.cache_key_list("requirement", None));
            self.invalidate_cache(&self.cache_key_list("requirement", Some(updated_req.project_id)));
            crate::cache::invalidate_requirement_cache(id);
            crate::cache::invalidate_project_cache(updated_req.project_id);
        }
        
        Ok(success)
    }
    
    /// Delete a requirement
    pub async fn delete_requirement(
        &self,
        id: i32,
        user_id: i32,
    ) -> ApiResult<bool> {
        // Get existing requirement for logging
        let old_req = self.get_requirement_by_id(id).await?;
        
        // Delete from database
        let success = self.base.repo()
            .delete_requirement(id)
            .map_err(|e| ApiError::Database(e))?;
        
        if success {
            // Log the deletion
            if let Ok(old_values) = crate::services::serialize_for_logging(&old_req) {
                let _ = self.base.log_delete(
                    user_id,
                    EntityType::Requirement,
                    id,
                    Some(old_req.project_id),
                    Some(old_values),
                    Some(format!("Deleted requirement: {}", old_req.req_title)),
                );
            }
            
            // Invalidate relevant caches
            self.invalidate_cache(&self.cache_key("requirement", id));
            self.invalidate_cache(&self.cache_key_list("requirement", None));
            self.invalidate_cache(&self.cache_key_list("requirement", Some(old_req.project_id)));
            crate::cache::invalidate_requirement_cache(id);
            crate::cache::invalidate_project_cache(old_req.project_id);
        }
        
        Ok(success)
    }
    
    /// Get requirements by category
    pub async fn get_requirements_by_category(&self, category_id: i32) -> ApiResult<Vec<Requirement>> {
        let cache_key = format!("requirement:category:{}", category_id);
        
        // Try to get from cache first
        if let Some(cached) = self.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        // Get from database
        let requirements = self.base.repo()
            .get_requirements_by_category(category_id)
            .map_err(|e| ApiError::Database(e))?;
        
        // Cache the result
        self.set_cache(&cache_key, requirements.clone(), Duration::from_secs(300));
        
        Ok(requirements)
    }
    
    /// Get requirements by status
    pub async fn get_requirements_by_status(&self, status_id: i32) -> ApiResult<Vec<Requirement>> {
        let cache_key = format!("requirement:status:{}", status_id);
        
        // Try to get from cache first
        if let Some(cached) = self.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        // Get from database
        let requirements = self.base.repo()
            .get_requirements_by_status(status_id)
            .map_err(|e| ApiError::Database(e))?;
        
        // Cache the result
        self.set_cache(&cache_key, requirements.clone(), Duration::from_secs(300));
        
        Ok(requirements)
    }
}

impl Service for RequirementService {
    fn repo(&self) -> &crate::repository::DieselRepo {
        self.base.repo()
    }
    
    fn repo_mut(&mut self) -> &mut crate::repository::DieselRepo {
        self.base.repo_mut()
    }
}

impl CacheableService<Vec<Requirement>> for RequirementService {}
impl CacheableService<Requirement> for RequirementService {}

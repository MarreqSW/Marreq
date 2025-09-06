//! Base service trait and common functionality for all services.
//!
//! This module provides the foundation for all service implementations,
//! including common patterns for database operations, caching, and error handling.

use crate::errors::{ApiError, ApiResult};
use crate::repository::DieselRepo;
use crate::cache;
use std::time::Duration;

/// Base trait for all services
pub trait Service {
    /// Get the repository instance
    fn repo(&self) -> &DieselRepo;
    
    /// Get a mutable repository instance
    fn repo_mut(&mut self) -> &mut DieselRepo;
}

/// Cacheable service trait for services that support caching
pub trait CacheableService<T> {
    /// Get cached value by key
    fn get_cached(&self, key: &str) -> Option<T>;
    
    /// Set cache value with TTL
    fn set_cache(&self, key: &str, value: T, ttl: Duration);
    
    /// Invalidate cache entry
    fn invalidate_cache(&self, key: &str);
    
    /// Generate cache key for entity
    fn cache_key(&self, entity_type: &str, id: i32) -> String {
        format!("{}:{}", entity_type, id)
    }
    
    /// Generate cache key for list
    fn cache_key_list(&self, entity_type: &str, project_id: Option<i32>) -> String {
        match project_id {
            Some(pid) => format!("{}:list:{}", entity_type, pid),
            None => format!("{}:list:all", entity_type),
        }
    }
}

/// Base service implementation
pub struct BaseService {
    repo: DieselRepo,
}

impl BaseService {
    /// Create a new base service
    pub fn new() -> Self {
        Self {
            repo: DieselRepo::new(),
        }
    }
    
    /// Get database connection with proper error handling
    pub fn get_connection(&self) -> ApiResult<crate::repository::PooledConnectionWrapper> {
        self.repo.get_conn()
            .map_err(|e| ApiError::Internal(format!("Database connection error: {}", e)))
    }
    
    /// Log operation for audit trail
    pub fn log_operation(
        &self,
        user_id: i32,
        action_type: crate::models::ActionType,
        entity_type: crate::models::EntityType,
        entity_id: Option<i32>,
        project_id: Option<i32>,
        old_values: Option<String>,
        new_values: Option<String>,
        description: Option<String>,
    ) -> ApiResult<()> {
        let mut conn = self.get_connection()?;
        
        crate::logger::Logger::log_action(
            conn.as_mut(),
            user_id,
            action_type,
            entity_type,
            entity_id,
            project_id,
            old_values,
            new_values,
            description,
            None, // No request context in service layer
        ).map_err(|e| ApiError::Internal(format!("Failed to log operation: {}", e)))?;
        
        Ok(())
    }
    
    /// Log creation operation
    pub fn log_create(
        &self,
        user_id: i32,
        entity_type: crate::models::EntityType,
        entity_id: i32,
        project_id: Option<i32>,
        new_values: Option<String>,
        description: Option<String>,
    ) -> ApiResult<()> {
        self.log_operation(
            user_id,
            crate::models::ActionType::Create,
            entity_type,
            Some(entity_id),
            project_id,
            None,
            new_values,
            description,
        )
    }
    
    /// Log update operation
    pub fn log_update(
        &self,
        user_id: i32,
        entity_type: crate::models::EntityType,
        entity_id: i32,
        project_id: Option<i32>,
        old_values: Option<String>,
        new_values: Option<String>,
        description: Option<String>,
    ) -> ApiResult<()> {
        self.log_operation(
            user_id,
            crate::models::ActionType::Update,
            entity_type,
            Some(entity_id),
            project_id,
            old_values,
            new_values,
            description,
        )
    }
    
    /// Log deletion operation
    pub fn log_delete(
        &self,
        user_id: i32,
        entity_type: crate::models::EntityType,
        entity_id: i32,
        project_id: Option<i32>,
        old_values: Option<String>,
        description: Option<String>,
    ) -> ApiResult<()> {
        self.log_operation(
            user_id,
            crate::models::ActionType::Delete,
            entity_type,
            Some(entity_id),
            project_id,
            old_values,
            None,
            description,
        )
    }
}

impl Service for BaseService {
    fn repo(&self) -> &DieselRepo {
        &self.repo
    }
    
    fn repo_mut(&mut self) -> &mut DieselRepo {
        &mut self.repo
    }
}

/// Default implementation for cache operations
impl<T> CacheableService<T> for BaseService 
where 
    T: serde::Serialize + serde::de::DeserializeOwned + Clone,
{
    fn get_cached(&self, key: &str) -> Option<T> {
        cache::get_cached_value(key)
    }
    
    fn set_cache(&self, key: &str, value: T, ttl: Duration) {
        cache::set_cached_value(key, value, ttl);
    }
    
    fn invalidate_cache(&self, key: &str) {
        cache::invalidate_cache_key(key);
    }
}

/// Helper function to serialize data for logging
pub fn serialize_for_logging<T>(data: &T) -> ApiResult<String>
where
    T: serde::Serialize,
{
    serde_json::to_string(data)
        .map_err(|e| ApiError::Serialization(e))
}

/// Helper function to check if user has permission for project
pub fn check_project_permission(
    user: &crate::models::User,
    _project_id: i32,
) -> ApiResult<()> {
    // Admin users have access to all projects
    if user.is_admin {
        return Ok(());
    }
    
    // For now, all users have access to all projects
    // This can be enhanced later with project-specific permissions
    Ok(())
}

/// Helper function to validate entity ownership
pub fn validate_entity_access(
    user: &crate::models::User,
    entity_project_id: i32,
) -> ApiResult<()> {
    check_project_permission(user, entity_project_id)
}

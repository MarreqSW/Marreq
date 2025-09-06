//! User service for managing users business logic.

use crate::errors::{ApiError, ApiResult};
use crate::models::*;
use crate::validation::validate_user;
use crate::services::{BaseService, Service};
use crate::repository::UserRepository;
use std::time::Duration;

pub struct UserService {
    base: BaseService,
}

impl UserService {
    pub fn new() -> Self {
        Self { base: BaseService::new() }
    }
    
    pub async fn get_all_users(&self) -> ApiResult<Vec<User>> {
        let cache_key = self.base.cache_key_list("user", None);
        if let Some(cached) = self.base.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        let users = self.base.repo()
            .get_users_all()
            .map_err(|e| ApiError::Repository(e))?;
        
        self.base.set_cache(&cache_key, users.clone(), Duration::from_secs(300));
        Ok(users)
    }
    
    pub async fn get_user_by_id(&self, id: i32) -> ApiResult<User> {
        let cache_key = self.base.cache_key("user", id);
        if let Some(cached) = self.base.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        let user = self.base.repo()
            .get_user_by_id(id)
            .map_err(|e| ApiError::Repository(e))?;
        
        self.base.set_cache(&cache_key, user.clone(), Duration::from_secs(600));
        Ok(user)
    }
    
    pub async fn create_user(
        &self,
        mut new_user: NewUser,
        user_id: i32,
    ) -> ApiResult<i32> {
        validate_user(&new_user)?;
        
        crate::validation::sanitize_string(&mut new_user.user_username);
        crate::validation::sanitize_string(&mut new_user.user_name);
        crate::validation::sanitize_string(&mut new_user.user_email);
        
        let mut repo = crate::repository::DieselRepo::new();
        let id = repo.insert_user(&new_user)
            .map_err(|e| ApiError::Repository(e))?;
        
        if let Ok(new_values) = crate::services::serialize_for_logging(&new_user) {
            let _ = self.base.log_create(
                user_id,
                EntityType::User,
                id,
                new_user.project_id,
                Some(new_values),
                Some(format!("Created user: {}", new_user.user_username)),
            );
        }
        
        self.base.invalidate_cache(&self.base.cache_key_list("user", None));
        if let Some(project_id) = new_user.project_id {
            crate::cache::invalidate_project_cache(project_id);
        }
        crate::cache::invalidate_user_cache(id);
        
        Ok(id)
    }
    
    pub async fn delete_user(&self, id: i32, user_id: i32) -> ApiResult<bool> {
        let old_user = self.get_user_by_id(id).await?;
        
        let mut repo = crate::repository::DieselRepo::new();
        let success = repo.delete_user(id)
            .map_err(|e| ApiError::Repository(e))?;
        
        if success {
            if let Ok(old_values) = crate::services::serialize_for_logging(&old_user) {
                let _ = self.base.log_delete(
                    user_id,
                    EntityType::User,
                    id,
                    None, // Users don't have project_id
                    Some(old_values),
                    Some(format!("Deleted user: {}", old_user.user_username)),
                );
            }
            
            self.base.invalidate_cache(&self.base.cache_key("user", id));
            self.base.invalidate_cache(&self.base.cache_key_list("user", None));
            crate::cache::invalidate_user_cache(id);
        }
        
        Ok(success)
    }
}

impl Service for UserService {
    fn repo(&self) -> &crate::repository::DieselRepo { self.base.repo() }
    fn repo_mut(&mut self) -> &mut crate::repository::DieselRepo { self.base.repo_mut() }
}




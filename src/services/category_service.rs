//! Category service for managing requirement categories business logic.

use crate::errors::{ApiError, ApiResult};
use crate::models::*;
use crate::validation::validate_category;
use crate::services::{BaseService, Service, CacheableService};
use std::time::Duration;

pub struct CategoryService {
    base: BaseService,
}

impl CategoryService {
    pub fn new() -> Self {
        Self { base: BaseService::new() }
    }
    
    pub async fn get_all_categories(&self) -> ApiResult<Vec<Category>> {
        let cache_key = self.cache_key_list("category", None);
        if let Some(cached) = self.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        let categories = self.base.repo()
            .get_categories_all()
            .map_err(|e| ApiError::Database(e))?;
        
        self.set_cache(&cache_key, categories.clone(), Duration::from_secs(300));
        Ok(categories)
    }
    
    pub async fn get_category_by_id(&self, id: i32) -> ApiResult<Category> {
        let cache_key = self.cache_key("category", id);
        if let Some(cached) = self.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        let category = self.base.repo()
            .get_category_by_id(id)
            .map_err(|e| ApiError::Database(e))?;
        
        self.set_cache(&cache_key, category.clone(), Duration::from_secs(600));
        Ok(category)
    }
    
    pub async fn create_category(
        &self,
        mut new_category: NewCategory,
        user_id: i32,
    ) -> ApiResult<i32> {
        validate_category(&new_category)?;
        
        crate::validation::sanitize_string(&mut new_category.cat_title);
        crate::validation::sanitize_string(&mut new_category.cat_description);
        crate::validation::sanitize_string(&mut new_category.cat_tag);
        
        let id = self.base.repo()
            .insert_new_category(&new_category)
            .map_err(|e| ApiError::Database(e))?;
        
        if let Ok(new_values) = crate::services::serialize_for_logging(&new_category) {
            let _ = self.base.log_create(
                user_id,
                EntityType::Category,
                id,
                Some(new_category.project_id),
                Some(new_values),
                Some(format!("Created category: {}", new_category.cat_title)),
            );
        }
        
        self.invalidate_cache(&self.cache_key_list("category", None));
        self.invalidate_cache(&self.cache_key_list("category", Some(new_category.project_id)));
        crate::cache::invalidate_category_cache(id);
        crate::cache::invalidate_project_cache(new_category.project_id);
        
        Ok(id)
    }
    
    pub async fn update_category(
        &self,
        id: i32,
        mut updated_category: NewCategory,
        user_id: i32,
    ) -> ApiResult<bool> {
        let old_category = self.get_category_by_id(id).await?;
        validate_category(&updated_category)?;
        
        crate::validation::sanitize_string(&mut updated_category.cat_title);
        crate::validation::sanitize_string(&mut updated_category.cat_description);
        crate::validation::sanitize_string(&mut updated_category.cat_tag);
        
        updated_category.cat_id = Some(id);
        
        let success = self.base.repo()
            .edit_category(&updated_category)
            .map_err(|e| ApiError::Database(e))?;
        
        if success {
            if let (Ok(old_values), Ok(new_values)) = (
                crate::services::serialize_for_logging(&old_category),
                crate::services::serialize_for_logging(&updated_category),
            ) {
                let _ = self.base.log_update(
                    user_id,
                    EntityType::Category,
                    id,
                    Some(updated_category.project_id),
                    Some(old_values),
                    Some(new_values),
                    Some(format!("Updated category: {}", updated_category.cat_title)),
                );
            }
            
            self.invalidate_cache(&self.cache_key("category", id));
            self.invalidate_cache(&self.cache_key_list("category", None));
            self.invalidate_cache(&self.cache_key_list("category", Some(updated_category.project_id)));
            crate::cache::invalidate_category_cache(id);
            crate::cache::invalidate_project_cache(updated_category.project_id);
        }
        
        Ok(success)
    }
    
    pub async fn delete_category(&self, id: i32, user_id: i32) -> ApiResult<bool> {
        let old_category = self.get_category_by_id(id).await?;
        
        let success = self.base.repo()
            .delete_category(id)
            .map_err(|e| ApiError::Database(e))?;
        
        if success {
            if let Ok(old_values) = crate::services::serialize_for_logging(&old_category) {
                let _ = self.base.log_delete(
                    user_id,
                    EntityType::Category,
                    id,
                    Some(old_category.project_id),
                    Some(old_values),
                    Some(format!("Deleted category: {}", old_category.cat_title)),
                );
            }
            
            self.invalidate_cache(&self.cache_key("category", id));
            self.invalidate_cache(&self.cache_key_list("category", None));
            self.invalidate_cache(&self.cache_key_list("category", Some(old_category.project_id)));
            crate::cache::invalidate_category_cache(id);
            crate::cache::invalidate_project_cache(old_category.project_id);
        }
        
        Ok(success)
    }
}

impl Service for CategoryService {
    fn repo(&self) -> &crate::repository::DieselRepo { self.base.repo() }
    fn repo_mut(&mut self) -> &mut crate::repository::DieselRepo { self.base.repo_mut() }
}

impl CacheableService<Vec<Category>> for CategoryService {}
impl CacheableService<Category> for CategoryService {}

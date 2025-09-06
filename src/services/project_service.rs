//! Project service for managing projects business logic.

use crate::errors::{ApiError, ApiResult};
use crate::models::*;
use crate::validation::validate_project;
use crate::services::{BaseService, Service};
use crate::repository::ProjectsRepository;
use std::time::Duration;

pub struct ProjectService {
    base: BaseService,
}

impl ProjectService {
    pub fn new() -> Self {
        Self { base: BaseService::new() }
    }
    
    pub async fn get_all_projects(&self) -> ApiResult<Vec<Project>> {
        let cache_key = self.base.cache_key_list("project", None);
        if let Some(cached) = self.base.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        let projects = self.base.repo()
            .get_projects_all()
            .map_err(|e| ApiError::Repository(e))?;
        
        self.base.set_cache(&cache_key, projects.clone(), Duration::from_secs(300));
        Ok(projects)
    }
    
    pub async fn get_project_by_id(&self, id: i32) -> ApiResult<Project> {
        let cache_key = self.base.cache_key("project", id);
        if let Some(cached) = self.base.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        let project = self.base.repo()
            .get_project_by_id(id)
            .map_err(|e| ApiError::Repository(e))?;
        
        self.base.set_cache(&cache_key, project.clone(), Duration::from_secs(600));
        Ok(project)
    }
    
    pub async fn create_project(
        &self,
        mut new_project: NewProject,
        user_id: i32,
    ) -> ApiResult<i32> {
        validate_project(&new_project)?;
        
        crate::validation::sanitize_string(&mut new_project.project_name);
        crate::validation::sanitize_optional_string(&mut new_project.project_description);
        
        let mut repo = crate::repository::DieselRepo::new();
        let id = repo.insert_new_project(&new_project)
            .map_err(|e| ApiError::Repository(e))?;
        
        if let Ok(new_values) = crate::services::serialize_for_logging(&new_project) {
            let _ = self.base.log_create(
                user_id,
                EntityType::Project,
                id,
                None, // Projects don't have a parent project
                Some(new_values),
                Some(format!("Created project: {}", new_project.project_name)),
            );
        }
        
        self.base.invalidate_cache(&self.base.cache_key_list("project", None));
        crate::cache::invalidate_project_cache(id);
        
        Ok(id)
    }
    
    pub async fn update_project(
        &self,
        id: i32,
        mut updated_project: NewProject,
        user_id: i32,
    ) -> ApiResult<bool> {
        let old_project = self.get_project_by_id(id).await?;
        validate_project(&updated_project)?;
        
        crate::validation::sanitize_string(&mut updated_project.project_name);
        crate::validation::sanitize_optional_string(&mut updated_project.project_description);
        
        let update_data = crate::models::UpdateProject {
            project_name: updated_project.project_name.clone(),
            project_description: updated_project.project_description.clone(),
            project_status: "active".to_string(), // Default status
            project_owner_id: None, // Default owner
        };
        
        let mut repo = crate::repository::DieselRepo::new();
        let success = repo.edit_project(id, &update_data)
            .map_err(|e| ApiError::Repository(e))?;
        
        if success {
            if let (Ok(old_values), Ok(new_values)) = (
                crate::services::serialize_for_logging(&old_project),
                crate::services::serialize_for_logging(&updated_project),
            ) {
                let _ = self.base.log_update(
                    user_id,
                    EntityType::Project,
                    id,
                    None,
                    Some(old_values),
                    Some(new_values),
                    Some(format!("Updated project: {}", updated_project.project_name)),
                );
            }
            
            self.base.invalidate_cache(&self.base.cache_key("project", id));
            self.base.invalidate_cache(&self.base.cache_key_list("project", None));
            crate::cache::invalidate_project_cache(id);
        }
        
        Ok(success)
    }
    
    pub async fn delete_project(&self, id: i32, user_id: i32) -> ApiResult<bool> {
        let old_project = self.get_project_by_id(id).await?;
        
        let mut repo = crate::repository::DieselRepo::new();
        let success = repo.delete_project(id)
            .map_err(|e| ApiError::Repository(e))?;
        
        if success {
            if let Ok(old_values) = crate::services::serialize_for_logging(&old_project) {
                let _ = self.base.log_delete(
                    user_id,
                    EntityType::Project,
                    id,
                    None,
                    Some(old_values),
                    Some(format!("Deleted project: {}", old_project.project_name)),
                );
            }
            
            self.base.invalidate_cache(&self.base.cache_key("project", id));
            self.base.invalidate_cache(&self.base.cache_key_list("project", None));
            crate::cache::invalidate_project_cache(id);
        }
        
        Ok(success)
    }
}

impl Service for ProjectService {
    fn repo(&self) -> &crate::repository::DieselRepo { self.base.repo() }
    fn repo_mut(&mut self) -> &mut crate::repository::DieselRepo { self.base.repo_mut() }
}




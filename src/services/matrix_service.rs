//! Matrix service for managing traceability matrix business logic.

use crate::errors::{ApiError, ApiResult};
use crate::models::*;
use crate::services::{BaseService, Service};
use crate::repository::MatrixRepository;
use std::time::Duration;

pub struct MatrixService {
    base: BaseService,
}

impl MatrixService {
    pub fn new() -> Self {
        Self { base: BaseService::new() }
    }
    
    pub async fn get_all_matrix(&self) -> ApiResult<Vec<Matrix>> {
        let cache_key = self.base.cache_key_list("matrix", None);
        if let Some(cached) = self.base.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        let matrix = self.base.repo()
            .get_matrix_all()
            .map_err(|e| ApiError::Repository(e))?;
        
        self.base.set_cache(&cache_key, matrix.clone(), Duration::from_secs(300));
        Ok(matrix)
    }
    
    pub async fn get_matrix_by_project(&self, project_id: i32) -> ApiResult<Vec<Matrix>> {
        let cache_key = self.base.cache_key_list("matrix", Some(project_id));
        if let Some(cached) = self.base.get_cached(&cache_key) {
            return Ok(cached);
        }
        
        let matrix = self.base.repo()
            .get_matrix_by_project(project_id)
            .map_err(|e| ApiError::Repository(e))?;
        
        self.base.set_cache(&cache_key, matrix.clone(), Duration::from_secs(300));
        Ok(matrix)
    }
    
    pub async fn create_matrix_link(
        &self,
        req_id: i32,
        test_id: i32,
        project_id: i32,
        user_id: i32,
    ) -> ApiResult<bool> {
        let mut repo = crate::repository::DieselRepo::new();
        let success = repo.insert_matrix_link(req_id, test_id, project_id)
            .map_err(|e| ApiError::Repository(e))?;
        
        if success {
            let _ = self.base.log_create(
                user_id,
                EntityType::Matrix,
                0, // Matrix doesn't have a single ID
                Some(project_id),
                None,
                Some(format!("Created matrix link: REQ-{} -> TEST-{}", req_id, test_id)),
            );
            
            self.base.invalidate_cache(&self.base.cache_key_list("matrix", None));
            self.base.invalidate_cache(&self.base.cache_key_list("matrix", Some(project_id)));
        }
        
        Ok(success)
    }
    
    pub async fn delete_matrix_link(
        &self,
        req_id: i32,
        test_id: i32,
        project_id: i32,
        user_id: i32,
    ) -> ApiResult<bool> {
        let mut repo = crate::repository::DieselRepo::new();
        let success = repo.delete_matrix_link(req_id, test_id)
            .map_err(|e| ApiError::Repository(e))?;
        
        if success {
            let _ = self.base.log_delete(
                user_id,
                EntityType::Matrix,
                0, // Matrix doesn't have a single ID
                Some(project_id),
                None,
                Some(format!("Deleted matrix link: REQ-{} -> TEST-{}", req_id, test_id)),
            );
            
            self.base.invalidate_cache(&self.base.cache_key_list("matrix", None));
            self.base.invalidate_cache(&self.base.cache_key_list("matrix", Some(project_id)));
        }
        
        Ok(success)
    }
}

impl Service for MatrixService {
    fn repo(&self) -> &crate::repository::DieselRepo { self.base.repo() }
    fn repo_mut(&mut self) -> &mut crate::repository::DieselRepo { self.base.repo_mut() }
}



use crate::cache::keys::Keyspace;
use crate::cache::{get_cache, invalidate_project_cache, invalidate_requirement_cache, keys};
use crate::models::{NewRequirement, Requirement};
use crate::repository::errors::RepoError;
use crate::repository::{DieselRepo, RequirementsRepository};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

/// Repository wrapper that checks the cache before hitting the database
pub struct CacheRepository {
    inner: DieselRepo,
}

impl CacheRepository {
    pub fn new() -> Self {
        Self {
            inner: DieselRepo::new(),
        }
    }

    fn get_or_fetch<T, F>(&self, key: &str, ttl: Duration, fetch: F) -> Result<T, RepoError>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Result<T, RepoError>,
    {
        let cache = get_cache();
        if let Some(cached) = cache.get(key) {
            if let Ok(value) = serde_json::from_str(&cached) {
                return Ok(value);
            }
            cache.remove(key);
        }
        let value = fetch()?;
        if let Ok(json) = serde_json::to_string(&value) {
            cache.set_with_ttl(key, json, ttl);
        }
        Ok(value)
    }
}

impl RequirementsRepository for CacheRepository {
    fn get_requirement_by_id(&self, id: i32) -> Result<Requirement, RepoError> {
        let key = keys::Requirements::by_id(id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_requirement_by_id(id)
        })
    }

    fn get_requirements_all(&self) -> Result<Vec<Requirement>, RepoError> {
        self.get_or_fetch(keys::REQUIREMENTS_ALL, Duration::from_secs(300), || {
            self.inner.get_requirements_all()
        })
    }

    fn get_requirements_by_project(&self, project_id: i32) -> Result<Vec<Requirement>, RepoError> {
        let key = keys::Requirements::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_requirements_by_project(project_id)
        })
    }

    fn insert_new_requirement(&mut self, new: &NewRequirement) -> Result<i32, RepoError> {
        let id = self.inner.insert_new_requirement(new)?;
        invalidate_requirement_cache(id);
        invalidate_project_cache(new.project_id);
        Ok(id)
    }

    fn edit_requirement(&mut self, new: &NewRequirement) -> Result<bool, RepoError> {
        let res = self.inner.edit_requirement(new)?;
        if let Some(id) = new.req_id {
            invalidate_requirement_cache(id);
        }
        invalidate_project_cache(new.project_id);
        Ok(res)
    }

    fn delete_requirement(&mut self, id: i32) -> Result<bool, RepoError> {
        let res = self.inner.delete_requirement(id)?;
        invalidate_requirement_cache(id);
        Ok(res)
    }

    fn update_requirement(&mut self, req: i32) -> Result<(), RepoError> {
        self.inner.update_requirement(req)?;
        invalidate_requirement_cache(req);
        Ok(())
    }
}

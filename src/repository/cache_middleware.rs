use super::cache::keys::Keyspace;
use super::cache::{keys, Cache};
use crate::models::*;
use crate::repository::errors::RepoError;
use crate::repository::{
    LookupRepository, MatrixRepository, ProjectsRepository, Repository, RequirementsRepository, TestsRepository, UserRepository
};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Repository wrapper that checks the cache before hitting the database
pub struct CacheRepository<R> {
    inner: R,
    cache: Arc<Cache>,
}

impl<R: Repository> CacheRepository<R> {
    /// Create a new repository wrapper with the provided cache instance
    pub fn new(inner: R, ttl_seconds: u64) -> Self {
        Self {
            inner,
            cache: Cache::new(ttl_seconds).into(),
        }
    }

    pub fn inner_repo(&self) -> &R {
        &self.inner
    }

    /// Get a reference to the underlying cache
    pub fn cache(&self) -> Arc<Cache> {
        Arc::clone(&self.cache)
    }

    fn get_or_fetch<T, F>(&self, key: &str, ttl: Duration, fetch: F) -> Result<T, RepoError>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Result<T, RepoError>,
    {
        if let Some(cached) = self.cache.get(key) {
            if let Ok(value) = serde_json::from_str(&cached) {
                return Ok(value);
            }
            self.cache.remove(key);
        }
        let value = fetch()?;
        if let Ok(json) = serde_json::to_string(&value) {
            self.cache.set_with_ttl(key, json, ttl);
        }
        Ok(value)
    }

        /// Warm up the cache with frequently accessed data
    ///
    /// Populates the cache with common queries to improve initial performance.
    /// Note: This function may copy significant amounts of data; use with caution.
    pub fn warm_cache(&self) {
        // Warm up projects cache
        if let Ok(projects) = self.inner.get_projects_all() {
            if let Ok(json_data) = serde_json::to_string(&projects) {
                self.cache.set_with_ttl(keys::PROJECTS_ALL, json_data, Duration::from_secs(600));
            }
        }

        // Warm up status cache
        if let Ok(statuses) = self.inner.get_status_all() {
            if let Ok(json_data) = serde_json::to_string(&statuses) {
                self.cache.set_with_ttl(keys::STATUS_ALL, json_data, Duration::from_secs(900));
            }
        }

        // Warm up categories cache
        if let Ok(categories) = self.inner.get_categories_all() {
            if let Ok(json_data) = serde_json::to_string(&categories) {
                self.cache.set_with_ttl(keys::CATEGORIES_ALL, json_data, Duration::from_secs(900));
            }
        }

        // Warm up users cache
        if let Ok(users) = self.inner.get_users_all() {
            if let Ok(json_data) = serde_json::to_string(&users) {
                self.cache.set_with_ttl(keys::USERS_ALL, json_data, Duration::from_secs(600));
            }
        }

        // Warm up projects navigation cache
        if let Ok(projects) = self.inner.get_projects_all() {
            if let Ok(json_data) = serde_json::to_string(&projects) {
                self.cache.set_with_ttl(keys::PROJECTS_NAV, json_data, Duration::from_secs(300));
            }
        }
    }
}

impl<R: Repository> RequirementsRepository for CacheRepository<R> {
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
        self.cache.invalidate_requirement(id);
        self.cache.invalidate_project(new.project_id);
        Ok(id)
    }

    fn edit_requirement(&mut self, new: &NewRequirement) -> Result<bool, RepoError> {
        let res = self.inner.edit_requirement(new)?;
        if let Some(id) = new.req_id {
            self.cache.invalidate_requirement(id);
        }
        self.cache.invalidate_project(new.project_id);
        Ok(res)
    }

    fn delete_requirement(&mut self, id: i32) -> Result<Requirement, RepoError> {
        let req = self.inner.delete_requirement(id)?;
        self.cache.invalidate_requirement(id);
        self.cache.invalidate_project(req.project_id);
        Ok(req)
    }

    fn update_requirement(&mut self, req: i32) -> Result<(), RepoError> {
        self.inner.update_requirement(req)?;
        self.cache.invalidate_requirement(req);
        Ok(())
    }
}

impl<R: Repository> UserRepository for CacheRepository<R> {
    fn get_users_all(&self) -> Result<Vec<User>, RepoError> {
        self.get_or_fetch(keys::USERS_ALL, Duration::from_secs(300), || {
            self.inner.get_users_all()
        })
    }

    fn get_user_by_id(&self, id: i32) -> Result<User, RepoError> {
        let key = keys::Users::by_id(id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_user_by_id(id)
        })
    }

    fn get_user_by_username(&self, uname: &str) -> Result<Option<User>, RepoError> {
        let key = format!("user:username:{}", uname);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_user_by_username(uname)
        })
    }

    fn insert_user(&mut self, new: &NewUser) -> Result<i32, RepoError> {
        let id = self.inner.insert_user(new)?;
        self.cache.invalidate_user(id);
        if let Some(project_id) = new.project_id {
            self.cache.invalidate_project(project_id);
        }
        Ok(id)
    }

    fn update_user_password(&mut self, id: i32, new_hash: &str) -> Result<(), RepoError> {
        self.inner.update_user_password(id, new_hash)?;
        self.cache.invalidate_user(id);
        Ok(())
    }

    fn update_user(&mut self, user_data: &NewUser) -> Result<bool, RepoError> {
        let res = self.inner.update_user(user_data)?;
        if let Some(id) = user_data.user_id {
            self.cache.invalidate_user(id);
        }
        Ok(res)
    }

    fn update_user_without_password(&mut self, user_data: &UpdateUser) -> Result<bool, RepoError> {
        let res = self.inner.update_user_without_password(user_data)?;
        if let Some(id) = user_data.user_id {
            self.cache.invalidate_user(id);
        }
        Ok(res)
    }

    fn delete_user(&mut self, id: i32) -> Result<User, RepoError> {
        let user = self.inner.delete_user(id)?;
        self.cache.invalidate_user(id);
        if let Some(pid) = user.project_id {
            self.cache.invalidate_project(pid);
        }
        Ok(user)
    }
}

impl<R: Repository> TestsRepository for CacheRepository<R> {
    fn get_test_by_id(&self, id: i32) -> Result<Test, RepoError> {
        let key = keys::Tests::by_id(id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_test_by_id(id)
        })
    }

    fn get_tests_all(&self) -> Result<Vec<Test>, RepoError> {
        self.get_or_fetch(keys::TESTS_ALL, Duration::from_secs(300), || {
            self.inner.get_tests_all()
        })
    }

    fn get_tests_by_project(&self, project_id: i32) -> Result<Vec<Test>, RepoError> {
        let key = keys::Tests::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_tests_by_project(project_id)
        })
    }

    fn get_requirements_for_test(&self, test_id: i32) -> Result<Vec<Requirement>, RepoError> {
        let key = keys::LinkedRequirements::for_test(test_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_requirements_for_test(test_id)
        })
    }

    fn get_tests_for_requirement(&self, req_id: i32) -> Result<Vec<Test>, RepoError> {
        let key = keys::LinkedTests::for_requirement(req_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_tests_for_requirement(req_id)
        })
    }

    fn insert_test(&mut self, new: &NewTest) -> Result<i32, RepoError> {
        let id = self.inner.insert_test(new)?;
        self.cache.invalidate_test(id);
        self.cache.invalidate_project(new.project_id);
        Ok(id)
    }

    fn edit_test(&mut self, new: &NewTest) -> Result<bool, RepoError> {
        let res = self.inner.edit_test(new)?;
        if let Some(id) = new.test_id {
            self.cache.invalidate_test(id);
        }
        self.cache.invalidate_project(new.project_id);
        Ok(res)
    }

    fn delete_test(&mut self, id: i32) -> Result<Test, RepoError> {
        let test = self.inner.delete_test(id)?;
        self.cache.invalidate_test(id);
        self.cache.invalidate_project(test.project_id);
        Ok(test)
    }

    fn update_test_requirement_links(
        &mut self,
        test_id: i32,
        requirement_ids: &[i32],
    ) -> Result<(), RepoError> {
        self.inner
            .update_test_requirement_links(test_id, requirement_ids)?;
        self.cache.invalidate_test(test_id);
        for &rid in requirement_ids {
            self.cache.invalidate_requirement(rid);
        }
        Ok(())
    }
}

impl<R: Repository> LookupRepository for CacheRepository<R> {
    fn get_status_all(&self) -> Result<Vec<Status>, RepoError> {
        self.get_or_fetch(keys::STATUS_ALL, Duration::from_secs(900), || {
            self.inner.get_status_all()
        })
    }

    fn get_status_by_id(&self, id: i32) -> Result<Status, RepoError> {
        let key = keys::Status::by_id(id);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner.get_status_by_id(id)
        })
    }

    fn get_categories_all(&self) -> Result<Vec<Category>, RepoError> {
        self.get_or_fetch(keys::CATEGORIES_ALL, Duration::from_secs(600), || {
            self.inner.get_categories_all()
        })
    }

    fn get_categories_by_project(&self, project_id: i32) -> Result<Vec<Category>, RepoError> {
        let key = keys::Categories::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_categories_by_project(project_id)
        })
    }

    fn get_category_by_id(&self, id: i32) -> Result<Category, RepoError> {
        let key = keys::Categories::by_id(id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_category_by_id(id)
        })
    }

    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError> {
        self.get_or_fetch(keys::APPLICABILITY_ALL, Duration::from_secs(600), || {
            self.inner.get_applicability_all()
        })
    }

    fn get_applicability_by_id(&self, id: i32) -> Result<Applicability, RepoError> {
        let key = keys::Applicability::by_id(id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_applicability_by_id(id)
        })
    }

    fn get_applicability_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<Applicability>, RepoError> {
        let key = keys::Applicability::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_applicability_by_project(project_id)
        })
    }

    fn get_verification_all(&self) -> Result<Vec<Verification>, RepoError> {
        self.get_or_fetch(keys::VERIFICATION_ALL, Duration::from_secs(600), || {
            self.inner.get_verification_all()
        })
    }

    fn get_verification_by_id(&self, id: i32) -> Result<Verification, RepoError> {
        let key = keys::Verification::by_id(id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_verification_by_id(id)
        })
    }

    fn get_verification_by_project(&self, project_id: i32) -> Result<Vec<Verification>, RepoError> {
        let key = keys::Verification::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_verification_by_project(project_id)
        })
    }

    fn create_status(&mut self, new: &NewStatus) -> Result<i32, RepoError> {
        let id = self.inner.create_status(new)?;
        self.cache.invalidate_status(id);
        Ok(id)
    }

    fn insert_new_category(&mut self, new: &NewCategory) -> Result<i32, RepoError> {
        let id = self.inner.insert_new_category(new)?;
        self.cache.invalidate_category(id);
        self.cache.invalidate_project(new.project_id);
        Ok(id)
    }

    fn edit_category(&mut self, new: &NewCategory) -> Result<bool, RepoError> {
        let res = self.inner.edit_category(new)?;
        if let Some(id) = new.cat_id {
            self.cache.invalidate_category(id);
        }
        self.cache.invalidate_project(new.project_id);
        Ok(res)
    }

    fn delete_category(&mut self, id: i32) -> Result<Category, RepoError> {
        let cat = self.inner.delete_category(id)?;
        self.cache.invalidate_category(id);
        self.cache.invalidate_project(cat.project_id);
        Ok(cat)
    }

    fn insert_new_applicability(&mut self, new: &NewApplicability) -> Result<i32, RepoError> {
        let id = self.inner.insert_new_applicability(new)?;
        self.cache.invalidate_applicability(id);
        self.cache.invalidate_project(new.project_id);
        Ok(id)
    }

    fn edit_applicability(&mut self, new: &NewApplicability) -> Result<bool, RepoError> {
        let res = self.inner.edit_applicability(new)?;
        if let Some(id) = new.app_id {
            self.cache.invalidate_applicability(id);
        }
        self.cache.invalidate_project(new.project_id);
        Ok(res)
    }

    fn delete_applicability(&mut self, id: i32) -> Result<Applicability, RepoError> {
        let app = self.inner.delete_applicability(id)?;
        self.cache.invalidate_applicability(id);
        self.cache.invalidate_project(app.project_id);
        Ok(app)
    }
}

impl<R: Repository> ProjectsRepository for CacheRepository<R> {
    fn get_projects_all(&self) -> Result<Vec<Project>, RepoError> {
        self.get_or_fetch(keys::PROJECTS_ALL, Duration::from_secs(600), || {
            self.inner.get_projects_all()
        })
    }

    fn get_project_by_id(&self, id: i32) -> Result<Project, RepoError> {
        let key = keys::Projects::by_id(id);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner.get_project_by_id(id)
        })
    }

    fn insert_new_project(&mut self, new: &NewProject) -> Result<i32, RepoError> {
        let id = self.inner.insert_new_project(new)?;
        self.cache.invalidate_project(id);
        Ok(id)
    }

    fn edit_project(&mut self, project_id: i32, update: &UpdateProject) -> Result<bool, RepoError> {
        let res = self.inner.edit_project(project_id, update)?;
        self.cache.invalidate_project(project_id);
        Ok(res)
    }

    fn delete_project(&mut self, project_id: i32) -> Result<Project, RepoError> {
        let proj = self.inner.delete_project(project_id)?;
        self.cache.invalidate_project(project_id);
        Ok(proj)
    }
}

impl<R: Repository> MatrixRepository for CacheRepository<R> {
    fn get_matrix_by_project(&self, project_id: i32) -> Result<Vec<Matrix>, RepoError> {
        let key = keys::Matrix::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(180), || {
            self.inner.get_matrix_by_project(project_id)
        })
    }

    fn insert_new_matrix_item(&mut self, new: &NewMatrix) -> Result<(), RepoError> {
        self.inner.insert_new_matrix_item(new)?;
        let key = keys::Matrix::by_project(new.project_id);
        self.cache.remove(&key);
        Ok(())
    }
}

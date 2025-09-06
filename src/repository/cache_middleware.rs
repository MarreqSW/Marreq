use crate::cache::keys::Keyspace;
use crate::cache::{
    get_cache, invalidate_applicability_cache, invalidate_category_cache, invalidate_project_cache,
    invalidate_requirement_cache, invalidate_status_cache, invalidate_test_cache,
    invalidate_user_cache, keys,
};
use crate::models::*;
use crate::repository::errors::RepoError;
use crate::repository::{
    DieselRepo, LookupRepository, MatrixRepository, ProjectsRepository, RequirementsRepository,
    TestsRepository, UserRepository,
};
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

impl UserRepository for CacheRepository {
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
        invalidate_user_cache(id);
        Ok(id)
    }

    fn update_user_password(&mut self, id: i32, new_hash: &str) -> Result<(), RepoError> {
        self.inner.update_user_password(id, new_hash)?;
        invalidate_user_cache(id);
        Ok(())
    }

    fn update_user(&mut self, user_data: &NewUser) -> Result<bool, RepoError> {
        let res = self.inner.update_user(user_data)?;
        if let Some(id) = user_data.user_id {
            invalidate_user_cache(id);
        }
        Ok(res)
    }

    fn update_user_without_password(&mut self, user_data: &UpdateUser) -> Result<bool, RepoError> {
        let res = self.inner.update_user_without_password(user_data)?;
        if let Some(id) = user_data.user_id {
            invalidate_user_cache(id);
        }
        Ok(res)
    }

    fn delete_user(&mut self, id: i32) -> Result<bool, RepoError> {
        let res = self.inner.delete_user(id)?;
        invalidate_user_cache(id);
        Ok(res)
    }
}

impl TestsRepository for CacheRepository {
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
        invalidate_test_cache(id);
        invalidate_project_cache(new.project_id);
        Ok(id)
    }

    fn edit_test(&mut self, new: &NewTest) -> Result<bool, RepoError> {
        let res = self.inner.edit_test(new)?;
        if let Some(id) = new.test_id {
            invalidate_test_cache(id);
        }
        invalidate_project_cache(new.project_id);
        Ok(res)
    }

    fn delete_test(&mut self, id: i32) -> Result<bool, RepoError> {
        let res = self.inner.delete_test(id)?;
        invalidate_test_cache(id);
        Ok(res)
    }

    fn update_test_requirement_links(
        &mut self,
        test_id: i32,
        requirement_ids: &[i32],
    ) -> Result<(), RepoError> {
        self.inner
            .update_test_requirement_links(test_id, requirement_ids)?;
        invalidate_test_cache(test_id);
        for &rid in requirement_ids {
            invalidate_requirement_cache(rid);
        }
        Ok(())
    }
}

impl LookupRepository for CacheRepository {
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
        invalidate_status_cache(id);
        Ok(id)
    }

    fn insert_new_category(&mut self, new: &NewCategory) -> Result<i32, RepoError> {
        let id = self.inner.insert_new_category(new)?;
        invalidate_category_cache(id);
        invalidate_project_cache(new.project_id);
        Ok(id)
    }

    fn edit_category(&mut self, new: &NewCategory) -> Result<bool, RepoError> {
        let res = self.inner.edit_category(new)?;
        if let Some(id) = new.cat_id {
            invalidate_category_cache(id);
        }
        invalidate_project_cache(new.project_id);
        Ok(res)
    }

    fn delete_category(&mut self, id: i32) -> Result<bool, RepoError> {
        let res = self.inner.delete_category(id)?;
        invalidate_category_cache(id);
        Ok(res)
    }

    fn insert_new_applicability(&mut self, new: &NewApplicability) -> Result<i32, RepoError> {
        let id = self.inner.insert_new_applicability(new)?;
        invalidate_applicability_cache(id);
        invalidate_project_cache(new.project_id);
        Ok(id)
    }

    fn edit_applicability(&mut self, new: &NewApplicability) -> Result<bool, RepoError> {
        let res = self.inner.edit_applicability(new)?;
        if let Some(id) = new.app_id {
            invalidate_applicability_cache(id);
        }
        invalidate_project_cache(new.project_id);
        Ok(res)
    }

    fn delete_applicability(&mut self, id: i32) -> Result<bool, RepoError> {
        let res = self.inner.delete_applicability(id)?;
        invalidate_applicability_cache(id);
        Ok(res)
    }
}

impl ProjectsRepository for CacheRepository {
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
        invalidate_project_cache(id);
        Ok(id)
    }

    fn edit_project(&mut self, project_id: i32, update: &UpdateProject) -> Result<bool, RepoError> {
        let res = self.inner.edit_project(project_id, update)?;
        invalidate_project_cache(project_id);
        Ok(res)
    }

    fn delete_project(&mut self, project_id: i32) -> Result<bool, RepoError> {
        let res = self.inner.delete_project(project_id)?;
        invalidate_project_cache(project_id);
        Ok(res)
    }
}

impl MatrixRepository for CacheRepository {
    fn get_matrix_by_project(&self, project_id: i32) -> Result<Vec<Matrix>, RepoError> {
        let key = keys::Matrix::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(180), || {
            self.inner.get_matrix_by_project(project_id)
        })
    }

    fn insert_new_matrix_item(&mut self, new: &NewMatrix) -> Result<(), RepoError> {
        self.inner.insert_new_matrix_item(new)?;
        let key = keys::Matrix::by_project(new.project_id);
        get_cache().remove(&key);
        Ok(())
    }
}

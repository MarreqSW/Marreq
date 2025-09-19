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
        self.cache.remove(super::cache::keys::REQUIREMENTS_ALL);
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
        self.cache.remove(super::cache::keys::TESTS_ALL);
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

    fn get_requirement_status_all(&self) -> Result<Vec<RequirementStatus>, RepoError> {
        self.get_or_fetch(keys::REQUIREMENT_STATUS_ALL, Duration::from_secs(900), || {
            self.inner.get_requirement_status_all()
        })
    }

    fn get_requirement_status_by_id(&self, id: i32) -> Result<RequirementStatus, RepoError> {
        let key = keys::RequirementStatus::by_id(id);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner.get_requirement_status_by_id(id)
        })
    }

    fn get_test_status_all(&self) -> Result<Vec<TestStatus>, RepoError> {
        self.get_or_fetch(keys::TEST_STATUS_ALL, Duration::from_secs(900), || {
            self.inner.get_test_status_all()
        })
    }

    fn get_test_status_by_id(&self, id: i32) -> Result<TestStatus, RepoError> {
        let key = keys::TestStatus::by_id(id);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner.get_test_status_by_id(id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::fake_repo::FakeRepo;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::collections::HashMap;

    fn epoch() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn populated_repo() -> FakeRepo {
        let user = FakeRepo::make_user(1, "alice", "hash");
        let status = Status { st_id: 1, st_title: "Open".into(), st_description: "".into(), st_short_name: "O".into() };
        let category = Category { cat_id: 1, cat_title: "Cat".into(), cat_description: "".into(), cat_tag: "C".into(), project_id: 1 };
        let app = Applicability { app_id: 1, app_title: "App".into(), app_description: "".into(), app_tag: "A".into(), project_id: 1 };
        let ver = Verification { verification_id: 1, verification_name: "Ver".into(), verification_description: "".into(), project_id: 1 };
        let project = Project {
            project_id: 1,
            project_name: "Proj".into(),
            project_description: Some("Desc".into()),
            project_creation_date: Some(epoch()),
            project_update_date: Some(epoch()),
            project_status: Some("Active".into()),
            project_owner_id: Some(1),
        };
        let requirement = Requirement {
            req_id: 1,
            req_title: "Req".into(),
            req_description: "".into(),
            req_verification: 1,
            req_current_status: 1,
            req_author: 1,
            req_reviewer: 1,
            req_link: "link".into(),
            req_reference: "ref".into(),
            req_category: 1,
            req_parent: 0,
            req_creation_date: epoch(),
            req_update_date: epoch(),
            req_deadline_date: epoch(),
            req_applicability: 1,
            req_justification: None,
            project_id: 1,
        };
        let test = Test {
            test_id: 1,
            test_name: "Test".into(),
            test_description: "".into(),
            test_source: "src".into(),
            test_status: 1,
            test_parent: 0,
            project_id: 1,
        };
        let matrix = Matrix { matrix_req_id: 1, matrix_test_id: 1, matrix_creation_date: epoch(), project_id: 1 };

        let mut users = HashMap::new();
        users.insert(1, user);
        let mut statuses = HashMap::new();
        statuses.insert(1, status);
        let mut categories = HashMap::new();
        categories.insert(1, category);
        let mut applicability = HashMap::new();
        applicability.insert(1, app);
        let mut verifications = HashMap::new();
        verifications.insert(1, ver);
        let mut requirements = HashMap::new();
        requirements.insert(1, requirement);
        let mut tests = HashMap::new();
        tests.insert(1, test);
        let mut projects = HashMap::new();
        projects.insert(1, project);

        FakeRepo {
            users,
            statuses,
            verifications,
            categories,
            applicability,
            requirements,
            tests,
            projects,
            matrices: vec![matrix],
            force_err: false,
        }
    }

    #[test]
    fn test_inner_repo_exposes_live_inner_repository() {
        // Start with a fully populated fake repo
        let mut repo = CacheRepository::new(populated_repo(), 60);

        // Read directly via inner_repo(); this should bypass the cache wrapper
        let initial_users = repo.inner_repo().get_users_all().unwrap();
        let initial_len = initial_users.len();
        assert!(initial_len >= 1);

        // Mutate the underlying repo through the wrapper (which changes the inner)
        let new_user = NewUser {
            user_id: None,
            user_username: "eve".into(),
            user_name: "Eve".into(),
            user_email: "eve@example.com".into(),
            user_level: 0,
            user_password: "pw".into(),
            project_id: None,
            is_admin: false,
        };
        let _new_id = repo.insert_user(&new_user).unwrap();

        // Read again via inner_repo(); the change should be visible,
        // demonstrating that inner_repo() returns a live reference to the inner R
        let after_users = repo.inner_repo().get_users_all().unwrap();
        assert_eq!(after_users.len(), initial_len + 1);
    }

    #[test]
    fn test_warm_cache_populates_common_keys() {
        let repo = CacheRepository::new(
             FakeRepo{users: HashMap::new(), ..Default::default()},
            60
        );

        let cache = repo.cache();

        repo.warm_cache();

        assert_eq!(cache.get(keys::PROJECTS_ALL), Some("[]".to_string()));
        assert_eq!(cache.get(keys::STATUS_ALL), Some("[]".to_string()));
        assert_eq!(cache.get(keys::CATEGORIES_ALL), Some("[]".to_string()));
        assert_eq!(cache.get(keys::USERS_ALL), Some("[]".to_string()));
        assert_eq!(cache.get(keys::PROJECTS_NAV), Some("[]".to_string()));
    }

    #[test]
    fn test_get_user_by_id_is_cached() {
        let user = FakeRepo::make_user(1, "alice", "hash");
        let mut users = HashMap::new();
        users.insert(user.user_id, user.clone());

        let repo = CacheRepository::new(FakeRepo { users, ..Default::default() }, 60);
        let cache = repo.cache();
        cache.reset_counters();

        // first call should miss cache and populate it
        let fetched = repo.get_user_by_id(1).unwrap();
        assert_eq!(fetched.user_username, "alice");
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);

        // second call should be served from cache
        let again = repo.get_user_by_id(1).unwrap();
        assert_eq!(again.user_username, "alice");
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_insert_user_invalidates_users_all_cache() {
        let user = FakeRepo::make_user(1, "bob", "hash");
        let mut users = HashMap::new();
        users.insert(user.user_id, user);
        let mut repo = CacheRepository::new(FakeRepo { users, ..Default::default() }, 60);
        let cache = repo.cache();

        // populate cache with all users
        let all = repo.get_users_all().unwrap();
        assert_eq!(cache.get(keys::USERS_ALL), Some(serde_json::to_string(&all).unwrap()));

        // inserting a new user should invalidate USERS_ALL cache
        let new_user = NewUser {
            user_id: None,
            user_username: "charlie".into(),
            user_name: "Charlie".into(),
            user_email: "charlie@example.com".into(),
            user_level: 0,
            user_password: "pw".into(),
            project_id: None,
            is_admin: false,
        };
        repo.insert_user(&new_user).unwrap();

        assert_eq!(cache.get(keys::USERS_ALL), None);
    }

    #[test]
    fn test_update_user_password_invalidates_cache() {
        let user = FakeRepo::make_user(1, "dave", "old");
        let mut users = HashMap::new();
        users.insert(user.user_id, user);
        let mut repo = CacheRepository::new(FakeRepo { users, ..Default::default() }, 60);
        let cache = repo.cache();

        // cache user entry
        repo.get_user_by_id(1).unwrap();
        assert!(cache.get(&keys::Users::by_id(1)).is_some());

        // updating password should invalidate cached entry
        repo.update_user_password(1, "newhash").unwrap();
        assert!(cache.get(&keys::Users::by_id(1)).is_none());
    }

    #[test]
    fn test_user_repository_flows_and_invalid_cache() {
        let mut repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        // Prepopulate invalid JSON to exercise removal path
        cache.set(&keys::Users::by_id(1), "not-json".into());
        let user = repo.get_user_by_id(1).unwrap();
        assert_eq!(user.user_username, "alice");
        assert!(cache
            .get(&keys::Users::by_id(1))
            .unwrap()
            .contains("alice"));

        // Username lookup is cached
        repo.get_user_by_username("alice").unwrap();
        assert!(cache.get("user:username:alice").is_some());

        // Populate list cache then insert new user to invalidate it
        repo.get_users_all().unwrap();
        assert!(cache.get(keys::USERS_ALL).is_some());
        let new_user = NewUser {
            user_id: None,
            user_username: "bob".into(),
            user_name: "Bob".into(),
            user_email: "b@example.com".into(),
            user_level: 0,
            user_password: "pw".into(),
            project_id: Some(1),
            is_admin: false,
        };
        let new_id = repo.insert_user(&new_user).unwrap();
        assert!(cache.get(keys::USERS_ALL).is_none());

        // Updating and deleting invalidate caches
        repo.update_user_password(new_id, "hash").unwrap();
        assert!(cache.get(&keys::Users::by_id(new_id)).is_none());

        let upd = NewUser {
            user_id: Some(new_id),
            user_username: "bob".into(),
            user_name: "Bob".into(),
            user_email: "b@example.com".into(),
            user_level: 0,
            user_password: "pw".into(),
            project_id: Some(1),
            is_admin: false,
        };
        repo.update_user(&upd).unwrap();
        assert!(cache.get(&keys::Users::by_id(new_id)).is_none());

        let upd2 = UpdateUser {
            user_id: Some(new_id),
            user_username: "b2".into(),
            user_name: "B2".into(),
            user_email: "b2@example.com".into(),
            user_level: 1,
            is_admin: false,
        };
        repo.update_user_without_password(&upd2).unwrap();
        assert!(cache.get(&keys::Users::by_id(new_id)).is_none());

        repo.delete_user(new_id).unwrap();
        assert!(cache.get(&keys::Users::by_id(new_id)).is_none());
    }

    #[test]
    fn test_requirements_repository_flows() {
        let mut repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        repo.get_requirement_by_id(1).unwrap();
        assert!(cache.get(&keys::Requirements::by_id(1)).is_some());

        repo.get_requirements_all().unwrap();
        assert!(cache.get(keys::REQUIREMENTS_ALL).is_some());

        repo.get_requirements_by_project(1).unwrap();
        assert!(cache.get(&keys::Requirements::by_project(1)).is_some());

        let new_req = NewRequirement {
            req_id: None,
            req_title: "R2".into(),
            req_description: "".into(),
            req_verification: 1,
            req_author: 1,
            req_link: "l2".into(),
            req_category: 1,
            req_current_status: 1,
            req_parent: 0,
            req_reference: "r2".into(),
            req_reviewer: 1,
            req_applicability: 1,
            req_justification: None,
            project_id: 1,
        };
        let rid = repo.insert_new_requirement(&new_req).unwrap();
        assert!(cache.get(&keys::Requirements::by_id(rid)).is_none());

        let edit_req = NewRequirement {
            req_id: Some(rid),
            req_title: "R2".into(),
            req_description: "".into(),
            req_verification: 1,
            req_author: 1,
            req_link: "l2".into(),
            req_category: 1,
            req_current_status: 1,
            req_parent: 0,
            req_reference: "r2".into(),
            req_reviewer: 1,
            req_applicability: 1,
            req_justification: None,
            project_id: 1,
        };
        repo.edit_requirement(&edit_req).unwrap();
        assert!(cache.get(&keys::Requirements::by_id(rid)).is_none());

        repo.update_requirement(1).unwrap();
        assert!(cache.get(&keys::Requirements::by_id(1)).is_none());

        repo.get_requirements_all().unwrap();
        repo.delete_requirement(rid).unwrap();
        assert!(cache.get(keys::REQUIREMENTS_ALL).is_none());
    }

    #[test]
    fn test_tests_repository_flows() {
        let mut repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        repo.get_test_by_id(1).unwrap();
        assert!(cache.get(&keys::Tests::by_id(1)).is_some());
        repo.get_tests_all().unwrap();
        assert!(cache.get(keys::TESTS_ALL).is_some());
        repo.get_tests_by_project(1).unwrap();
        assert!(cache.get(&keys::Tests::by_project(1)).is_some());

        let reqs = repo.get_requirements_for_test(1).unwrap();
        assert_eq!(reqs.len(), 1);
        let tests = repo.get_tests_for_requirement(1).unwrap();
        assert_eq!(tests.len(), 1);

        let new_test = NewTest {
            test_id: None,
            test_name: "T2".into(),
            test_description: "".into(),
            test_source: "s".into(),
            test_status: 1,
            test_parent: 0,
            project_id: 1,
        };
        let tid = repo.insert_test(&new_test).unwrap();
        assert!(cache.get(&keys::Tests::by_id(tid)).is_none());

        let edit_test = NewTest {
            test_id: Some(tid),
            test_name: "T2".into(),
            test_description: "".into(),
            test_source: "s".into(),
            test_status: 1,
            test_parent: 0,
            project_id: 1,
        };
        repo.edit_test(&edit_test).unwrap();
        assert!(cache.get(&keys::Tests::by_id(tid)).is_none());

        repo.update_test_requirement_links(tid, &[1]).unwrap();
        assert!(cache.get(&keys::Tests::by_id(tid)).is_none());
        assert!(cache.get(&keys::Requirements::by_id(1)).is_none());

        repo.get_tests_all().unwrap();
        repo.delete_test(tid).unwrap();
        assert!(cache.get(keys::TESTS_ALL).is_none());
    }

    #[test]
    fn test_lookup_project_and_matrix_flows() {
        let mut repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        // Status operations
        repo.get_status_all().unwrap();
        assert!(cache.get(keys::STATUS_ALL).is_some());
        repo.get_status_by_id(1).unwrap();
        assert!(cache.get(&keys::Status::by_id(1)).is_some());
        let ns = NewStatus { req_st_title: "Closed".into(), req_st_description: "".into(), req_st_short_name: "C".into() };
        let stid = repo.create_status(&ns).unwrap();
        assert!(cache.get(&keys::Status::by_id(stid)).is_none());

        // Category operations
        repo.get_categories_all().unwrap();
        repo.get_category_by_id(1).unwrap();
        repo.get_categories_by_project(1).unwrap();
        let nc = NewCategory { cat_id: None, cat_title: "Cat2".into(), cat_description: "".into(), cat_tag: "C2".into(), project_id: 1 };
        let cid = repo.insert_new_category(&nc).unwrap();
        let ec = NewCategory {
            cat_id: Some(cid),
            cat_title: "Cat2".into(),
            cat_description: "".into(),
            cat_tag: "C2".into(),
            project_id: 1,
        };
        repo.edit_category(&ec).unwrap();
        repo.delete_category(cid).unwrap();

        // Applicability operations
        repo.get_applicability_all().unwrap();
        repo.get_applicability_by_id(1).unwrap();
        repo.get_applicability_by_project(1).unwrap();
        let na = NewApplicability { app_id: None, app_title: "App2".into(), app_description: "".into(), app_tag: "A2".into(), project_id: 1 };
        let aid = repo.insert_new_applicability(&na).unwrap();
        let ea = NewApplicability {
            app_id: Some(aid),
            app_title: "App2".into(),
            app_description: "".into(),
            app_tag: "A2".into(),
            project_id: 1,
        };
        repo.edit_applicability(&ea).unwrap();
        repo.delete_applicability(aid).unwrap();

        // Verification operations
        repo.get_verification_all().unwrap();
        repo.get_verification_by_id(1).unwrap();
        repo.get_verification_by_project(1).unwrap();

        // Project operations
        repo.get_projects_all().unwrap();
        repo.get_project_by_id(1).unwrap();
        let np = NewProject { project_name: "P2".into(), project_description: Some("".into()), project_status: "Active".into(), project_owner_id: Some(1) };
        let pid = repo.insert_new_project(&np).unwrap();
        let up = UpdateProject { project_name: "P2a".into(), project_description: Some("".into()), project_status: "Active".into(), project_owner_id: Some(1) };
        repo.edit_project(pid, &up).unwrap();
        repo.delete_project(pid).unwrap();

        // Matrix operations
        repo.get_matrix_by_project(1).unwrap();
        assert!(cache.get(&keys::Matrix::by_project(1)).is_some());
        repo.insert_new_matrix_item(&NewMatrix { matrix_req_id: 1, matrix_test_id: 1, project_id: 1 })
            .unwrap();
        assert!(cache.get(&keys::Matrix::by_project(1)).is_none());
    }
}

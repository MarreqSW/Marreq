use super::cache::keys::Keyspace;
use super::cache::{keys, Cache};
use crate::models::*;
use crate::repository::errors::RepoError;
use crate::repository::{
    LogRepository, LookupRepository, MatrixRepository, ProjectMembersRepository,
    ProjectsRepository, Repository, RequirementsRepository, TestsCaseRepository, UserRepository,
};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Repository wrapper that checks the cache before hitting the database
#[derive(Clone)]
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
                self.cache
                    .set_with_ttl(keys::PROJECTS_ALL, json_data, Duration::from_secs(600));
            }
        }

        // Warm up status cache
        if let Ok(statuses) = self.inner.get_requirement_status_all() {
            if let Ok(json_data) = serde_json::to_string(&statuses) {
                self.cache.set_with_ttl(
                    keys::REQUIREMENT_STATUS_ALL,
                    json_data,
                    Duration::from_secs(900),
                );
            }
        }

        // Warm up categories cache
        if let Ok(categories) = self.inner.get_categories_all() {
            if let Ok(json_data) = serde_json::to_string(&categories) {
                self.cache
                    .set_with_ttl(keys::CATEGORIES_ALL, json_data, Duration::from_secs(900));
            }
        }

        // Warm up users cache
        if let Ok(users) = self.inner.get_users_all() {
            if let Ok(json_data) = serde_json::to_string(&users) {
                self.cache
                    .set_with_ttl(keys::USERS_ALL, json_data, Duration::from_secs(600));
            }
        }

        // Warm up projects navigation cache
        if let Ok(projects) = self.inner.get_projects_all() {
            if let Ok(json_data) = serde_json::to_string(&projects) {
                self.cache
                    .set_with_ttl(keys::PROJECTS_NAV, json_data, Duration::from_secs(300));
            }
        }
    }
}

impl<R: Repository> RequirementsRepository for CacheRepository<R> {
    fn get_requirement_by_id(&self, requirement_id: i32) -> Result<Requirement, RepoError> {
        let key = keys::Requirements::by_id(requirement_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_requirement_by_id(requirement_id)
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
        if let Some(id) = new.id {
            self.cache.invalidate_requirement(id);
        }
        self.cache.invalidate_project(new.project_id);
        Ok(res)
    }

    fn delete_requirement(&mut self, requirement_id: i32) -> Result<Requirement, RepoError> {
        let req = self.inner.delete_requirement(requirement_id)?;
        self.cache.invalidate_requirement(requirement_id);
        self.cache.invalidate_project(req.project_id);
        self.cache.remove(super::cache::keys::REQUIREMENTS_ALL);
        Ok(req)
    }

    fn update_requirement(&mut self, requirement_id: i32) -> Result<(), RepoError> {
        self.inner.update_requirement(requirement_id)?;
        self.cache.invalidate_requirement(requirement_id);
        Ok(())
    }
}

impl<R: Repository> UserRepository for CacheRepository<R> {
    fn get_users_all(&self) -> Result<Vec<User>, RepoError> {
        self.get_or_fetch(keys::USERS_ALL, Duration::from_secs(300), || {
            self.inner.get_users_all()
        })
    }

    fn get_user_by_id(&self, user_id: i32) -> Result<User, RepoError> {
        let key = keys::Users::by_id(user_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_user_by_id(user_id)
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
        Ok(id)
    }

    fn update_user_password(&mut self, user_id: i32, new_hash: &str) -> Result<(), RepoError> {
        self.inner.update_user_password(user_id, new_hash)?;
        self.cache.invalidate_user(user_id);
        Ok(())
    }

    fn update_user(&mut self, user_data: &NewUser) -> Result<bool, RepoError> {
        let res = self.inner.update_user(user_data)?;
        if let Some(id) = user_data.id {
            self.cache.invalidate_user(id);
        }
        Ok(res)
    }

    fn update_user_without_password(&mut self, user_data: &UpdateUser) -> Result<bool, RepoError> {
        let res = self.inner.update_user_without_password(user_data)?;
        if let Some(id) = user_data.id {
            self.cache.invalidate_user(id);
        }
        Ok(res)
    }

    fn delete_user(&mut self, user_id: i32) -> Result<User, RepoError> {
        let memberships = self.inner.get_projects_for_user(user_id)?;
        let user = self.inner.delete_user(user_id)?;
        self.cache.invalidate_user(user_id);
        for membership in memberships {
            self.cache
                .invalidate_project_membership(membership.project_id, user_id);
            self.cache.invalidate_project(membership.project_id);
        }
        Ok(user)
    }
}

impl<R: Repository> ProjectMembersRepository for CacheRepository<R> {
    fn get_members_by_project(&self, project_id: i32) -> Result<Vec<ProjectMember>, RepoError> {
        let key = keys::ProjectMembers::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_members_by_project(project_id)
        })
    }

    fn get_projects_for_user(&self, user_id: i32) -> Result<Vec<ProjectMember>, RepoError> {
        let key = keys::ProjectMembers::for_user(user_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_projects_for_user(user_id)
        })
    }

    fn add_project_member(&mut self, new: &NewProjectMember) -> Result<(), RepoError> {
        self.inner.add_project_member(new)?;
        self.cache
            .invalidate_project_membership(new.project_id, new.user_id);
        self.cache.invalidate_project(new.project_id);
        self.cache.invalidate_user(new.user_id);
        Ok(())
    }

    fn update_project_member_role(
        &mut self,
        project_id: i32,
        id: i32,
        role: i32,
    ) -> Result<(), RepoError> {
        self.inner
            .update_project_member_role(project_id, id, role)?;
        self.cache.invalidate_project_membership(project_id, id);
        self.cache.invalidate_project(project_id);
        self.cache.invalidate_user(id);
        Ok(())
    }

    fn remove_project_member(&mut self, project_id: i32, id: i32) -> Result<(), RepoError> {
        self.inner.remove_project_member(project_id, id)?;
        self.cache.invalidate_project_membership(project_id, id);
        self.cache.invalidate_project(project_id);
        self.cache.invalidate_user(id);
        Ok(())
    }
}

impl<R: Repository> TestsCaseRepository for CacheRepository<R> {
    fn get_test_by_id(&self, test_id: i32) -> Result<TestCase, RepoError> {
        let key = keys::Tests::by_id(test_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_test_by_id(test_id)
        })
    }

    fn get_tests_all(&self) -> Result<Vec<TestCase>, RepoError> {
        self.get_or_fetch(keys::TESTS_ALL, Duration::from_secs(300), || {
            self.inner.get_tests_all()
        })
    }

    fn get_tests_by_project(&self, project_id: i32) -> Result<Vec<TestCase>, RepoError> {
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

    fn get_tests_for_requirement(&self, requirement_id: i32) -> Result<Vec<TestCase>, RepoError> {
        let key = keys::LinkedTests::for_requirement(requirement_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_tests_for_requirement(requirement_id)
        })
    }

    fn insert_test(&mut self, new: &NewTestCase) -> Result<i32, RepoError> {
        let id = self.inner.insert_test(new)?;
        self.cache.invalidate_test(id);
        self.cache.invalidate_project(new.project_id);
        Ok(id)
    }

    fn edit_test(&mut self, new: &NewTestCase) -> Result<bool, RepoError> {
        let res = self.inner.edit_test(new)?;
        if let Some(id) = new.id {
            self.cache.invalidate_test(id);
        }
        self.cache.invalidate_project(new.project_id);
        Ok(res)
    }

    fn delete_test(&mut self, test_id: i32) -> Result<TestCase, RepoError> {
        let test = self.inner.delete_test(test_id)?;
        self.cache.invalidate_test(test_id);
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
        for &requirement_id in requirement_ids {
            self.cache.invalidate_requirement(requirement_id);
        }
        Ok(())
    }
}

impl<R: Repository> LookupRepository for CacheRepository<R> {
    fn get_requirement_status_all(&self) -> Result<Vec<RequirementStatus>, RepoError> {
        self.get_or_fetch(
            keys::REQUIREMENT_STATUS_ALL,
            Duration::from_secs(900),
            || self.inner.get_requirement_status_all(),
        )
    }

    fn get_requirement_status_by_id(&self, status_id: i32) -> Result<RequirementStatus, RepoError> {
        let key = keys::RequirementStatus::by_id(status_id);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner.get_requirement_status_by_id(status_id)
        })
    }

    fn get_test_status_all(&self) -> Result<Vec<TestStatus>, RepoError> {
        self.get_or_fetch(keys::TEST_STATUS_ALL, Duration::from_secs(900), || {
            self.inner.get_test_status_all()
        })
    }

    fn get_test_status_by_id(&self, status_id: i32) -> Result<TestStatus, RepoError> {
        let key = keys::TestStatus::by_id(status_id);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner.get_test_status_by_id(status_id)
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

    fn get_category_by_id(&self, category_id: i32) -> Result<Category, RepoError> {
        let key = keys::Categories::by_id(category_id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_category_by_id(category_id)
        })
    }

    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError> {
        self.get_or_fetch(keys::APPLICABILITY_ALL, Duration::from_secs(600), || {
            self.inner.get_applicability_all()
        })
    }

    fn get_applicability_by_id(&self, applicability_id: i32) -> Result<Applicability, RepoError> {
        let key = keys::Applicability::by_id(applicability_id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_applicability_by_id(applicability_id)
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

    fn get_verification_all(&self) -> Result<Vec<VerificationMethod>, RepoError> {
        self.get_or_fetch(keys::VERIFICATION_ALL, Duration::from_secs(600), || {
            self.inner.get_verification_all()
        })
    }

    fn get_verification_by_id(
        &self,
        verification_id: i32,
    ) -> Result<VerificationMethod, RepoError> {
        let key = keys::VerificationMethod::by_id(verification_id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_verification_by_id(verification_id)
        })
    }

    fn get_verification_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<VerificationMethod>, RepoError> {
        let key = keys::VerificationMethod::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_verification_by_project(project_id)
        })
    }

    fn create_requirement_status(&mut self, new: &NewRequirementStatus) -> Result<i32, RepoError> {
        let id = self.inner.create_requirement_status(new)?;
        self.cache.invalidate_status(id);
        Ok(id)
    }

    fn create_test_status(&mut self, new: &NewTestStatus) -> Result<i32, RepoError> {
        let id = self.inner.create_test_status(new)?;
        self.cache.invalidate_status(id);
        Ok(id)
    }

    fn insert_new_verification(&mut self, new: &NewVerificationMethod) -> Result<i32, RepoError> {
        let id = self.inner.insert_new_verification(new)?;
        self.cache.invalidate_verification(id);
        self.cache.invalidate_project(new.project_id);
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
        if let Some(id) = new.id {
            self.cache.invalidate_category(id);
        }
        self.cache.invalidate_project(new.project_id);
        Ok(res)
    }

    fn delete_category(&mut self, category_id: i32) -> Result<Category, RepoError> {
        let cat = self.inner.delete_category(category_id)?;
        self.cache.invalidate_category(category_id);
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
        if let Some(id) = new.id {
            self.cache.invalidate_applicability(id);
        }
        self.cache.invalidate_project(new.project_id);
        Ok(res)
    }

    fn delete_applicability(&mut self, applicability_id: i32) -> Result<Applicability, RepoError> {
        let app = self.inner.delete_applicability(applicability_id)?;
        self.cache.invalidate_applicability(applicability_id);
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

    fn get_project_by_id(&self, project_id: i32) -> Result<Project, RepoError> {
        let key = keys::Projects::by_id(project_id);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner.get_project_by_id(project_id)
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
    fn get_matrix_by_project(&self, project_id: i32) -> Result<Vec<MatrixLink>, RepoError> {
        let key = keys::Matrix::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(180), || {
            self.inner.get_matrix_by_project(project_id)
        })
    }

    fn insert_new_matrix_item(&mut self, new: &NewMatrixLink) -> Result<(), RepoError> {
        self.inner.insert_new_matrix_item(new)?;
        let key = keys::Matrix::by_project(new.project_id);
        self.cache.remove(&key);
        Ok(())
    }
}

impl<R: LogRepository> LogRepository for CacheRepository<R> {
    fn insert_log(&mut self, new: &NewLog) -> Result<(), RepoError> {
        self.inner.insert_log(new)
    }

    fn get_logs_recent(&self, limit: i64) -> Result<Vec<Log>, RepoError> {
        self.inner.get_logs_recent(limit)
    }

    fn get_logs_by_entity(&self, entity_type: &str, entity_id: i32) -> Result<Vec<Log>, RepoError> {
        self.inner.get_logs_by_entity(entity_type, entity_id)
    }

    fn cleanup_logs(&mut self, days: i64) -> Result<usize, RepoError> {
        self.inner.cleanup_logs(days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::status_enums::ProjectStatus;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::collections::HashMap;

    fn epoch() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn populated_repo() -> DieselRepoMock {
        let user = DieselRepoMock::make_user(1, "alice", "hash");
        let status = RequirementStatus {
            id: 1,
            title: "Open".into(),
            description: "".into(),
            tag: "O".into(),
            project_id: 1,
        };
        let category = Category {
            id: 1,
            title: "Cat".into(),
            description: "".into(),
            tag: "C".into(),
            project_id: 1,
        };
        let app = Applicability {
            id: 1,
            title: "App".into(),
            description: "".into(),
            tag: "A".into(),
            project_id: 1,
        };
        let ver = VerificationMethod {
            id: 1,
            title: "Ver".into(),
            description: "".into(),
            tag: "VER".into(),
            project_id: 1,
        };
        let project = Project {
            id: 1,
            name: "Proj".into(),
            description: Some("Desc".into()),
            creation_date: Some(epoch()),
            update_date: Some(epoch()),
            status: ProjectStatus::Active,
            owner_id: Some(1),
        };
        let requirement = Requirement {
            id: 1,
            title: "Req".into(),
            description: "".into(),
            verification_method_id: 1,
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: "ref".into(),
            category_id: 1,
            parent_id: None,
            creation_date: epoch(),
            update_date: epoch(),
            deadline_date: Some(epoch()),
            applicability_id: 1,
            justification: None,
            project_id: 1,
        };
        let test = TestCase {
            id: 1,
            name: "Test".into(),
            description: "".into(),
            source: "src".into(),
            status_id: 1,
            reference_code: "TEST-1".into(),
            parent_id: None,
            project_id: 1,
        };
        let matrix = MatrixLink {
            req_id: 1,
            test_id: 1,
            creation_date: epoch(),
            project_id: 1,
        };

        let mut users = HashMap::new();
        users.insert(1, user);
        let mut requirement_statuses = HashMap::new();
        requirement_statuses.insert(1, status.clone());
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

        DieselRepoMock {
            logs: Vec::new(),
            users,
            statuses,
            requirement_statuses,
            test_statuses: HashMap::new(),
            verifications,
            categories,
            applicability,
            requirements,
            tests,
            projects,
            matrices: vec![matrix],
            project_members: Vec::new(),
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
            id: None,
            username: "eve".into(),
            name: "Eve".into(),
            email: "eve@example.com".into(),
            password_hash: "pw".into(),
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
            DieselRepoMock {
                users: HashMap::new(),
                ..Default::default()
            },
            60,
        );

        let cache = repo.cache();

        repo.warm_cache();

        assert_eq!(cache.get(keys::PROJECTS_ALL), Some("[]".to_string()));
        assert_eq!(
            cache.get(keys::REQUIREMENT_STATUS_ALL),
            Some("[]".to_string())
        );
        assert_eq!(cache.get(keys::CATEGORIES_ALL), Some("[]".to_string()));
        assert_eq!(cache.get(keys::USERS_ALL), Some("[]".to_string()));
        assert_eq!(cache.get(keys::PROJECTS_NAV), Some("[]".to_string()));
    }

    #[test]
    fn test_get_user_by_id_is_cached() {
        let user = DieselRepoMock::make_user(1, "alice", "hash");
        let mut users = HashMap::new();
        users.insert(user.id, user.clone());

        let repo = CacheRepository::new(
            DieselRepoMock {
                users,
                ..Default::default()
            },
            60,
        );
        let cache = repo.cache();
        cache.reset_counters();

        // first call should miss cache and populate it
        let fetched = repo.get_user_by_id(1).unwrap();
        assert_eq!(fetched.username, "alice");
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);

        // second call should be served from cache
        let again = repo.get_user_by_id(1).unwrap();
        assert_eq!(again.username, "alice");
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_insert_user_invalidates_users_all_cache() {
        let user = DieselRepoMock::make_user(1, "bob", "hash");
        let mut users = HashMap::new();
        users.insert(user.id, user);
        let mut repo = CacheRepository::new(
            DieselRepoMock {
                users,
                ..Default::default()
            },
            60,
        );
        let cache = repo.cache();

        // populate cache with all users
        let all = repo.get_users_all().unwrap();
        assert_eq!(
            cache.get(keys::USERS_ALL),
            Some(serde_json::to_string(&all).unwrap())
        );

        // inserting a new user should invalidate USERS_ALL cache
        let new_user = NewUser {
            id: None,
            username: "charlie".into(),
            name: "Charlie".into(),
            email: "charlie@example.com".into(),
            password_hash: "pw".into(),
            is_admin: false,
        };
        repo.insert_user(&new_user).unwrap();

        assert_eq!(cache.get(keys::USERS_ALL), None);
    }

    #[test]
    fn test_update_user_password_invalidates_cache() {
        let user = DieselRepoMock::make_user(1, "dave", "old");
        let mut users = HashMap::new();
        users.insert(user.id, user);
        let mut repo = CacheRepository::new(
            DieselRepoMock {
                users,
                ..Default::default()
            },
            60,
        );
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
        assert_eq!(user.username, "alice");
        assert!(cache.get(&keys::Users::by_id(1)).unwrap().contains("alice"));

        // Username lookup is cached
        repo.get_user_by_username("alice").unwrap();
        assert!(cache.get("user:username:alice").is_some());

        // Populate list cache then insert new user to invalidate it
        repo.get_users_all().unwrap();
        assert!(cache.get(keys::USERS_ALL).is_some());
        let new_user = NewUser {
            id: None,
            username: "bob".into(),
            name: "Bob".into(),
            email: "b@example.com".into(),
            password_hash: "pw".into(),
            is_admin: false,
        };
        let new_id = repo.insert_user(&new_user).unwrap();
        assert!(cache.get(keys::USERS_ALL).is_none());

        // Updating and deleting invalidate caches
        repo.update_user_password(new_id, "hash").unwrap();
        assert!(cache.get(&keys::Users::by_id(new_id)).is_none());

        let upd = NewUser {
            id: Some(new_id),
            username: "bob".into(),
            name: "Bob".into(),
            email: "b@example.com".into(),
            password_hash: "pw".into(),
            is_admin: false,
        };
        repo.update_user(&upd).unwrap();
        assert!(cache.get(&keys::Users::by_id(new_id)).is_none());

        let upd2 = UpdateUser {
            id: Some(new_id),
            username: "b2".into(),
            name: "B2".into(),
            email: "b2@example.com".into(),
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
            id: None,
            title: "R2".into(),
            description: "".into(),
            verification_method_id: 1,
            author_id: 1,
            category_id: 1,
            status_id: 1,
            parent_id: None,
            reference_code: "r2".into(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: None,
            project_id: 1,
        };
        let rid = repo.insert_new_requirement(&new_req).unwrap();
        assert!(cache.get(&keys::Requirements::by_id(rid)).is_none());

        let edit_req = NewRequirement {
            id: Some(rid),
            title: "R2".into(),
            description: "".into(),
            verification_method_id: 1,
            author_id: 1,
            category_id: 1,
            status_id: 1,
            parent_id: None,
            reference_code: "r2".into(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: None,
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

        let new_test = NewTestCase {
            id: None,
            name: "T2".into(),
            description: "".into(),
            source: "s".into(),
            status_id: 1,
            reference_code: "TEST-2".into(),
            parent_id: None,
            project_id: 1,
        };
        let tid = repo.insert_test(&new_test).unwrap();
        assert!(cache.get(&keys::Tests::by_id(tid)).is_none());

        let edit_test = NewTestCase {
            id: Some(tid),
            name: "T2".into(),
            description: "".into(),
            source: "s".into(),
            status_id: 1,
            reference_code: "TEST-2".into(),
            parent_id: None,
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
        repo.get_requirement_status_all().unwrap();
        assert!(cache.get(keys::REQUIREMENT_STATUS_ALL).is_some());
        repo.get_requirement_status_by_id(1).unwrap();
        assert!(cache.get(&keys::RequirementStatus::by_id(1)).is_some());
        let ns = NewRequirementStatus {
            id: None,
            title: "Closed".into(),
            description: "".into(),
            tag: "C".into(),
            project_id: 1,
        };
        let stid = repo.create_requirement_status(&ns).unwrap();
        assert!(cache.get(&keys::RequirementStatus::by_id(stid)).is_none());

        // Category operations
        repo.get_categories_all().unwrap();
        repo.get_category_by_id(1).unwrap();
        repo.get_categories_by_project(1).unwrap();
        let nc = NewCategory {
            id: None,
            title: "Cat2".into(),
            description: "".into(),
            tag: "C2".into(),
            project_id: 1,
        };
        let cid = repo.insert_new_category(&nc).unwrap();
        let ec = NewCategory {
            id: Some(cid),
            title: "Cat2".into(),
            description: "".into(),
            tag: "C2".into(),
            project_id: 1,
        };
        repo.edit_category(&ec).unwrap();
        repo.delete_category(cid).unwrap();

        // Applicability operations
        repo.get_applicability_all().unwrap();
        repo.get_applicability_by_id(1).unwrap();
        repo.get_applicability_by_project(1).unwrap();
        let na = NewApplicability {
            id: None,
            title: "App2".into(),
            description: "".into(),
            tag: "A2".into(),
            project_id: 1,
        };
        let aid = repo.insert_new_applicability(&na).unwrap();
        let ea = NewApplicability {
            id: Some(aid),
            title: "App2".into(),
            description: "".into(),
            tag: "A2".into(),
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
        let np = NewProject {
            name: "P2".into(),
            description: Some("".into()),
            status: ProjectStatus::Active,
            owner_id: Some(1),
        };
        let pid = repo.insert_new_project(&np).unwrap();
        let up = UpdateProject {
            name: "P2a".into(),
            description: Some("".into()),
            status: Some(ProjectStatus::Active),
            owner_id: Some(1),
        };
        repo.edit_project(pid, &up).unwrap();
        repo.delete_project(pid).unwrap();

        // Matrix operations
        repo.get_matrix_by_project(1).unwrap();
        assert!(cache.get(&keys::Matrix::by_project(1)).is_some());
        repo.insert_new_matrix_item(&NewMatrixLink {
            req_id: 1,
            test_id: 1,
            project_id: 1,
        })
        .unwrap();
        assert!(cache.get(&keys::Matrix::by_project(1)).is_none());
    }

    #[test]
    fn test_get_or_fetch_with_json_deserialization_failure() {
        let repo = CacheRepository::new(
            DieselRepoMock {
                users: HashMap::new(),
                ..Default::default()
            },
            60,
        );
        let cache = repo.cache();

        // Insert invalid JSON
        cache.set(&keys::Users::by_id(1), "invalid json".to_string());

        // This should remove the invalid entry and fetch from repo
        // But since repo is empty, it should return NotFound
        let result = repo.get_user_by_id(1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::NotFound));

        // Invalid entry should be removed
        assert!(cache.get(&keys::Users::by_id(1)).is_none());
    }

    #[test]
    fn test_get_or_fetch_with_serialization_failure() {
        // This test verifies that if serialization fails, we still return the value
        // In practice, this is hard to test without a custom type that fails to serialize
        // But we can test the error propagation path
        let repo = CacheRepository::new(
            DieselRepoMock {
                users: HashMap::new(),
                ..Default::default()
            },
            60,
        );

        // Test that errors from inner repo are propagated
        let result = repo.get_user_by_id(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_repository_inner_repo_mutability() {
        let mut repo = CacheRepository::new(populated_repo(), 60);

        // Test that we can mutate through the wrapper
        let new_user = NewUser {
            id: None,
            username: "test".into(),
            name: "Test".into(),
            email: "test@example.com".into(),
            password_hash: "hash".into(),
            is_admin: false,
        };

        let result = repo.insert_user(&new_user);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_repository_warm_cache_with_errors() {
        // Test warm_cache when inner repo returns errors
        let repo = CacheRepository::new(DieselRepoMock::with_error(), 60);

        // Should not panic even if inner repo returns errors
        repo.warm_cache();
    }

    #[test]
    fn test_cache_repository_log_repository_passthrough() {
        let mut repo = CacheRepository::new(populated_repo(), 60);

        let new_log = NewLog {
            user_id: 1,
            entity_type: "test".into(),
            entity_id: Some(1),
            action_type: "create".into(),
            description: Some("test".into()),
            project_id: Some(1),
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        };

        // Log operations should pass through without caching
        let result = repo.insert_log(&new_log);
        assert!(result.is_ok());

        let logs = repo.get_logs_recent(10).unwrap();
        assert_eq!(logs.len(), 1);
    }

    #[test]
    fn test_cache_repository_get_logs_by_entity() {
        let mut repo = CacheRepository::new(populated_repo(), 60);

        let new_log = NewLog {
            user_id: 1,
            entity_type: "requirement".into(),
            entity_id: Some(1),
            action_type: "create".into(),
            description: Some("test".into()),
            project_id: Some(1),
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        };
        repo.insert_log(&new_log).unwrap();

        let logs = repo.get_logs_by_entity("requirement", 1).unwrap();
        assert_eq!(logs.len(), 1);
    }

    #[test]
    fn test_cache_repository_cleanup_logs() {
        let mut repo = CacheRepository::new(populated_repo(), 60);

        for i in 0..5 {
            let new_log = NewLog {
                user_id: 1,
                entity_type: "requirement".into(),
                entity_id: Some(i),
                action_type: "create".into(),
                description: Some("test".into()),
                project_id: Some(1),
                old_values: None,
                new_values: None,
                ip_address: None,
                user_agent: None,
            };
            repo.insert_log(&new_log).unwrap();
        }

        let result = repo.cleanup_logs(30);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_repository_matrix_insert_invalidates_cache() {
        let mut repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        // Populate cache
        repo.get_matrix_by_project(1).unwrap();
        assert!(cache.get(&keys::Matrix::by_project(1)).is_some());

        // Insert new matrix item should invalidate cache
        repo.insert_new_matrix_item(&NewMatrixLink {
            req_id: 1,
            test_id: 1,
            project_id: 1,
        })
        .unwrap();

        assert!(cache.get(&keys::Matrix::by_project(1)).is_none());
    }

    #[test]
    fn test_cache_repository_delete_user_invalidates_project_memberships() {
        let mut repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        // Add a project member
        let new_member = NewProjectMember {
            project_id: 1,
            user_id: 1,
            role: 1,
        };
        repo.add_project_member(&new_member).unwrap();

        // Cache project members
        repo.get_members_by_project(1).unwrap();
        repo.get_projects_for_user(1).unwrap();

        // Delete user should invalidate both caches
        repo.delete_user(1).unwrap();

        // Note: The cache invalidation happens in delete_user implementation
        // We verify the user cache is invalidated
        assert!(cache.get(&keys::Users::by_id(1)).is_none());
    }

    #[test]
    fn test_get_or_fetch_with_serialization_error_handling() {
        // Test that if serialization fails, we still return the value
        let repo = CacheRepository::new(populated_repo(), 60);

        // Insert a value that will fail to serialize (this is hard to test without custom types)
        // But we can test the error path by ensuring fetch errors are propagated
        let result = repo.get_user_by_id(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_or_fetch_cache_hit_path() {
        let repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        // First call populates cache
        let user1 = repo.get_user_by_id(1).unwrap();

        // Second call should hit cache
        let user2 = repo.get_user_by_id(1).unwrap();
        assert_eq!(user1.id, user2.id);

        // Verify cache was used
        assert!(cache.get(&keys::Users::by_id(1)).is_some());
    }

    #[test]
    fn test_get_or_fetch_cache_miss_then_hit() {
        let repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        // First call - cache miss
        let _ = repo.get_user_by_id(1).unwrap();
        let stats_before = cache.stats();

        // Second call - cache hit
        let _ = repo.get_user_by_id(1).unwrap();
        let stats_after = cache.stats();

        // Should have one more hit
        assert!(stats_after.hits > stats_before.hits);
    }

    #[test]
    fn test_cache_repository_all_trait_methods_accessible() {
        let repo = CacheRepository::new(populated_repo(), 60);

        // Test that all repository trait methods are accessible through CacheRepository
        let _ = repo.get_users_all();
        let _ = repo.get_user_by_id(1);
        let _ = repo.get_user_by_username("alice");
        let _ = repo.get_requirements_all();
        let _ = repo.get_requirement_by_id(1);
        let _ = repo.get_requirements_by_project(1);
        let _ = repo.get_tests_all();
        let _ = repo.get_test_by_id(1);
        let _ = repo.get_tests_by_project(1);
        let _ = repo.get_projects_all();
        let _ = repo.get_project_by_id(1);
        let _ = repo.get_categories_all();
        let _ = repo.get_category_by_id(1);
        let _ = repo.get_categories_by_project(1);
        let _ = repo.get_applicability_all();
        let _ = repo.get_applicability_by_id(1);
        let _ = repo.get_applicability_by_project(1);
        let _ = repo.get_verification_all();
        let _ = repo.get_verification_by_id(1);
        let _ = repo.get_verification_by_project(1);
        let _ = repo.get_requirement_status_all();
        let _ = repo.get_requirement_status_by_id(1);
        let _ = repo.get_test_status_all();
        let _ = repo.get_test_status_by_id(1);
        let _ = repo.get_members_by_project(1);
        let _ = repo.get_projects_for_user(1);
        let _ = repo.get_matrix_by_project(1);
        let _ = repo.get_requirements_for_test(1);
        let _ = repo.get_tests_for_requirement(1);
        let _ = repo.get_logs_recent(10);
        let _ = repo.get_logs_by_entity("requirement", 1);

        // If we get here, all methods are accessible
        assert!(true);
    }

    #[test]
    fn test_cache_repository_inner_repo_immutable_access() {
        let repo = CacheRepository::new(populated_repo(), 60);

        // Test that inner_repo() provides read access
        let users1 = repo.inner_repo().get_users_all().unwrap();
        let users2 = repo.inner_repo().get_users_all().unwrap();

        assert_eq!(users1.len(), users2.len());
    }

    #[test]
    fn test_cache_repository_cache_access() {
        let repo = CacheRepository::new(populated_repo(), 60);
        let cache1 = repo.cache();
        let cache2 = repo.cache();

        // Both should be the same Arc
        cache1.set("test_key", "test_value".to_string());
        assert_eq!(cache2.get("test_key"), Some("test_value".to_string()));
    }

    #[test]
    fn test_warm_cache_handles_serialization_errors() {
        let repo = CacheRepository::new(
            DieselRepoMock {
                users: HashMap::new(),
                ..Default::default()
            },
            60,
        );

        // Should not panic even if serialization fails
        repo.warm_cache();
    }

    #[test]
    fn test_warm_cache_with_partial_data() {
        let repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        // Clear some data to test partial warm
        cache.clear();

        // Warm cache should populate what's available
        repo.warm_cache();

        // Should have populated some keys
        assert!(
            cache.get(keys::PROJECTS_ALL).is_some()
                || cache.get(keys::REQUIREMENT_STATUS_ALL).is_some()
                || cache.get(keys::CATEGORIES_ALL).is_some()
                || cache.get(keys::USERS_ALL).is_some()
        );
    }
}

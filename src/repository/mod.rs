pub mod cache;
pub mod cache_middleware;
pub mod diesel_repo;
// Make mock available for both unit tests and integration tests
// This module is only used in test code and does not affect production builds
pub mod diesel_repo_mock;
pub mod errors;

pub use cache::*;
pub use cache_middleware::CacheRepository;
pub use diesel_repo::*;

use crate::models::*;
use errors::RepoError;
use rocket::async_trait;
use rocket::tokio::task;
use std::sync::{Arc, RwLock};

pub trait UserRepository {
    fn get_users_all(&self) -> Result<Vec<User>, RepoError>;
    fn get_user_by_id(&self, user_id: i32) -> Result<User, RepoError>;
    fn get_user_by_username(&self, uname: &str) -> Result<Option<User>, RepoError>;

    fn insert_user(&mut self, new: &NewUser) -> Result<i32, RepoError>;
    fn update_user_password(&mut self, user_id: i32, new_hash: &str) -> Result<(), RepoError>;
    fn update_user(&mut self, user_data: &NewUser) -> Result<bool, RepoError>;
    fn update_user_without_password(&mut self, user_data: &UpdateUser) -> Result<bool, RepoError>;
    fn delete_user(&mut self, user_id: i32) -> Result<User, RepoError>;
}

pub trait RequirementsRepository {
    fn get_requirement_by_id(&self, requirement_id: i32) -> Result<Requirement, RepoError>;
    fn get_requirements_all(&self) -> Result<Vec<Requirement>, RepoError>;
    fn get_requirements_by_project(&self, project_id: i32) -> Result<Vec<Requirement>, RepoError>;

    fn insert_new_requirement(&mut self, new: &NewRequirement) -> Result<i32, RepoError>;
    fn edit_requirement(&mut self, new: &NewRequirement) -> Result<bool, RepoError>;
    fn delete_requirement(&mut self, requirement_id: i32) -> Result<Requirement, RepoError>;
    fn update_requirement(&mut self, requirement_id: i32) -> Result<(), RepoError>;
}

pub trait TestsCaseRepository {
    fn get_test_by_id(&self, test_id: i32) -> Result<TestCase, RepoError>;
    fn get_tests_all(&self) -> Result<Vec<TestCase>, RepoError>;
    fn get_tests_by_project(&self, project_id: i32) -> Result<Vec<TestCase>, RepoError>;
    fn get_requirements_for_test(&self, test_id: i32) -> Result<Vec<Requirement>, RepoError>;
    fn get_tests_for_requirement(&self, requirement_id: i32) -> Result<Vec<TestCase>, RepoError>;

    fn insert_test(&mut self, new: &NewTestCase) -> Result<i32, RepoError>;
    fn edit_test(&mut self, new: &NewTestCase) -> Result<bool, RepoError>;
    fn delete_test(&mut self, test_id: i32) -> Result<TestCase, RepoError>;
    fn update_test_requirement_links(
        &mut self,
        test_id: i32,
        requirement_ids: &[i32],
    ) -> Result<(), RepoError>;
}

pub trait LookupRepository {
    fn get_requirement_status_all(&self) -> Result<Vec<RequirementStatus>, RepoError>;
    fn get_requirement_status_by_id(&self, status_id: i32) -> Result<RequirementStatus, RepoError>;

    fn get_test_status_all(&self) -> Result<Vec<TestStatus>, RepoError>;
    fn get_test_status_by_id(&self, status_id: i32) -> Result<TestStatus, RepoError>;

    fn get_categories_all(&self) -> Result<Vec<Category>, RepoError>;
    fn get_categories_by_project(&self, project_id: i32) -> Result<Vec<Category>, RepoError>;
    fn get_category_by_id(&self, category_id: i32) -> Result<Category, RepoError>;

    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError>;
    fn get_applicability_by_id(&self, applicability_id: i32) -> Result<Applicability, RepoError>;
    fn get_applicability_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<Applicability>, RepoError>;

    fn get_verification_all(&self) -> Result<Vec<VerificationMethod>, RepoError>;
    fn get_verification_by_id(&self, verification_id: i32)
        -> Result<VerificationMethod, RepoError>;
    fn get_verification_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<VerificationMethod>, RepoError>;

    fn create_requirement_status(&mut self, new: &NewRequirementStatus) -> Result<i32, RepoError>;
    fn create_test_status(&mut self, new: &NewTestStatus) -> Result<i32, RepoError>;

    fn insert_new_verification(&mut self, new: &NewVerificationMethod) -> Result<i32, RepoError>;
    fn insert_new_category(&mut self, new: &NewCategory) -> Result<i32, RepoError>;
    fn edit_category(&mut self, new: &NewCategory) -> Result<bool, RepoError>;
    fn delete_category(&mut self, category_id: i32) -> Result<Category, RepoError>;

    fn insert_new_applicability(&mut self, new: &NewApplicability) -> Result<i32, RepoError>;
    fn edit_applicability(&mut self, new: &NewApplicability) -> Result<bool, RepoError>;
    fn delete_applicability(&mut self, applicability_id: i32) -> Result<Applicability, RepoError>;
}

pub trait ProjectsRepository {
    fn get_projects_all(&self) -> Result<Vec<Project>, RepoError>;
    fn get_project_by_id(&self, project_id: i32) -> Result<Project, RepoError>;

    fn insert_new_project(&mut self, new: &NewProject) -> Result<i32, RepoError>;
    fn edit_project(&mut self, project_id: i32, update: &UpdateProject) -> Result<bool, RepoError>;
    fn delete_project(&mut self, project_id: i32) -> Result<Project, RepoError>;
}

pub trait ProjectMembersRepository {
    fn get_members_by_project(&self, project_id: i32) -> Result<Vec<ProjectMember>, RepoError>;
    fn get_projects_for_user(&self, user_id: i32) -> Result<Vec<ProjectMember>, RepoError>;

    fn add_project_member(&mut self, new: &NewProjectMember) -> Result<(), RepoError>;
    fn update_project_member_role(
        &mut self,
        project_id: i32,
        user_id: i32,
        role: i32,
    ) -> Result<(), RepoError>;
    fn remove_project_member(&mut self, project_id: i32, user_id: i32) -> Result<(), RepoError>;
}

pub trait MatrixRepository {
    fn get_matrix_by_project(&self, project_id: i32) -> Result<Vec<MatrixLink>, RepoError>;
    fn insert_new_matrix_item(&mut self, new: &NewMatrixLink) -> Result<(), RepoError>;
}

pub trait LogRepository {
    fn insert_log(&mut self, new: &NewLog) -> Result<(), RepoError>;
    fn get_logs_recent(&self, limit: i64) -> Result<Vec<Log>, RepoError>;
    fn get_logs_by_entity(&self, entity_type: &str, entity_id: i32) -> Result<Vec<Log>, RepoError>;
    fn cleanup_logs(&mut self, days: i64) -> Result<usize, RepoError>;
}

pub trait Repository:
    UserRepository
    + LookupRepository
    + RequirementsRepository
    + TestsCaseRepository
    + ProjectsRepository
    + ProjectMembersRepository
    + MatrixRepository
    + LogRepository
{
}

impl<T> Repository for T where
    T: UserRepository
        + LookupRepository
        + RequirementsRepository
        + TestsCaseRepository
        + ProjectsRepository
        + ProjectMembersRepository
        + MatrixRepository
        + LogRepository
{
}

#[async_trait]
pub trait RepoLockExt<R>: Send + Sync {
    async fn async_read<F, T>(&self, f: F) -> Result<T, RepoError>
    where
        F: FnOnce(&R) -> Result<T, RepoError> + Send + 'static,
        T: Send + 'static;

    async fn async_write<F, T>(&self, f: F) -> Result<T, RepoError>
    where
        F: FnOnce(&mut R) -> Result<T, RepoError> + Send + 'static,
        T: Send + 'static;
}

#[async_trait]
impl<R> RepoLockExt<R> for Arc<RwLock<R>>
where
    R: Send + Sync + 'static,
{
    async fn async_read<F, T>(&self, f: F) -> Result<T, RepoError>
    where
        F: FnOnce(&R) -> Result<T, RepoError> + Send + 'static,
        T: Send + 'static,
    {
        let repo = Arc::clone(self);
        task::spawn_blocking(move || {
            let guard = repo
                .read()
                .map_err(|_| RepoError::Pool("repo lock poisoned".into()))?;
            f(&*guard)
        })
        .await
        .map_err(|_| RepoError::Pool("async task join error".into()))?
    }

    async fn async_write<F, T>(&self, f: F) -> Result<T, RepoError>
    where
        F: FnOnce(&mut R) -> Result<T, RepoError> + Send + 'static,
        T: Send + 'static,
    {
        let repo = Arc::clone(self);
        task::spawn_blocking(move || {
            let mut guard = repo
                .write()
                .map_err(|_| RepoError::Pool("repo lock poisoned".into()))?;
            f(&mut *guard)
        })
        .await
        .map_err(|_| RepoError::Pool("async task join error".into()))?
    }
}

pub mod cache;
pub mod cache_middleware;
pub mod diesel_repo;
pub mod errors;
pub mod fake_repo;

pub use cache::*;
pub use cache_middleware::CacheRepository;
pub use diesel_repo::*;

use crate::models::*;
use errors::RepoError;

pub trait UserRepository {
    fn get_users_all(&self) -> Result<Vec<User>, RepoError>;
    fn get_user_by_id(&self, id: i32) -> Result<User, RepoError>;
    fn get_user_by_username(&self, uname: &str) -> Result<Option<User>, RepoError>;

    fn insert_user(&mut self, new: &NewUser) -> Result<i32, RepoError>;
    fn update_user_password(&mut self, id: i32, new_hash: &str) -> Result<(), RepoError>;
    fn update_user(&mut self, user_data: &NewUser) -> Result<bool, RepoError>;
    fn update_user_without_password(&mut self, user_data: &UpdateUser) -> Result<bool, RepoError>;
    fn delete_user(&mut self, id: i32) -> Result<bool, RepoError>;
}

pub trait RequirementsRepository {
    fn get_requirement_by_id(&self, id: i32) -> Result<Requirement, RepoError>;
    fn get_requirements_all(&self) -> Result<Vec<Requirement>, RepoError>;
    fn get_requirements_by_project(&self, project_id: i32) -> Result<Vec<Requirement>, RepoError>;

    fn insert_new_requirement(&mut self, new: &NewRequirement) -> Result<i32, RepoError>;
    fn edit_requirement(&mut self, new: &NewRequirement) -> Result<bool, RepoError>;
    fn delete_requirement(&mut self, id: i32) -> Result<bool, RepoError>;
    fn update_requirement(&mut self, req: i32) -> Result<(), RepoError>;
}

pub trait TestsRepository {
    fn get_test_by_id(&self, id: i32) -> Result<Test, RepoError>;
    fn get_tests_all(&self) -> Result<Vec<Test>, RepoError>;
    fn get_tests_by_project(&self, project_id: i32) -> Result<Vec<Test>, RepoError>;
    fn get_requirements_for_test(&self, test_id: i32) -> Result<Vec<Requirement>, RepoError>;
    fn get_tests_for_requirement(&self, req_id: i32) -> Result<Vec<Test>, RepoError>;

    fn insert_test(&mut self, new: &NewTest) -> Result<i32, RepoError>;
    fn edit_test(&mut self, new: &NewTest) -> Result<bool, RepoError>;
    fn delete_test(&mut self, id: i32) -> Result<bool, RepoError>;
    fn update_test_requirement_links(
        &mut self,
        test_id: i32,
        requirement_ids: &[i32],
    ) -> Result<(), RepoError>;
}

pub trait LookupRepository {
    fn get_status_all(&self) -> Result<Vec<Status>, RepoError>;
    fn get_status_by_id(&self, id: i32) -> Result<Status, RepoError>;

    fn get_categories_all(&self) -> Result<Vec<Category>, RepoError>;
    fn get_categories_by_project(&self, project_id: i32) -> Result<Vec<Category>, RepoError>;
    fn get_category_by_id(&self, id: i32) -> Result<Category, RepoError>;

    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError>;
    fn get_applicability_by_id(&self, id: i32) -> Result<Applicability, RepoError>;
    fn get_applicability_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<Applicability>, RepoError>;

    fn get_verification_all(&self) -> Result<Vec<Verification>, RepoError>;
    fn get_verification_by_id(&self, id: i32) -> Result<Verification, RepoError>;
    fn get_verification_by_project(&self, project_id: i32) -> Result<Vec<Verification>, RepoError>;

    fn create_status(&mut self, new: &NewStatus) -> Result<i32, RepoError>;

    fn insert_new_category(&mut self, new: &NewCategory) -> Result<i32, RepoError>;
    fn edit_category(&mut self, new: &NewCategory) -> Result<bool, RepoError>;
    fn delete_category(&mut self, id: i32) -> Result<bool, RepoError>;

    fn insert_new_applicability(&mut self, new: &NewApplicability) -> Result<i32, RepoError>;
    fn edit_applicability(&mut self, new: &NewApplicability) -> Result<bool, RepoError>;
    fn delete_applicability(&mut self, id: i32) -> Result<bool, RepoError>;
}

pub trait ProjectsRepository {
    fn get_projects_all(&self) -> Result<Vec<Project>, RepoError>;
    fn get_project_by_id(&self, id: i32) -> Result<Project, RepoError>;

    fn insert_new_project(&mut self, new: &NewProject) -> Result<i32, RepoError>;
    fn edit_project(&mut self, project_id: i32, update: &UpdateProject) -> Result<bool, RepoError>;
    fn delete_project(&mut self, project_id: i32) -> Result<bool, RepoError>;
}

pub trait MatrixRepository {
    fn get_matrix_by_project(&self, project_id: i32) -> Result<Vec<Matrix>, RepoError>;
    fn insert_new_matrix_item(&mut self, new: &NewMatrix) -> Result<(), RepoError>;
}

pub trait Repository:
    UserRepository
    + LookupRepository
    + RequirementsRepository
    + TestsRepository
    + ProjectsRepository
    + MatrixRepository
{
}

impl<T> Repository for T where
    T: UserRepository
        + LookupRepository
        + RequirementsRepository
        + TestsRepository
        + ProjectsRepository
        + MatrixRepository
{
}

pub mod errors;
pub mod diesel_repo;
pub mod fake_repo;

pub use diesel_repo::*;

use crate::models::*;
use errors::RepoError;

pub trait UserRepository {
    fn get_users_all(&self) -> Result<Vec<User>, RepoError>;
    fn get_user_by_id(&self, id: i32) -> Result<User, RepoError>;
    fn get_user_by_username(&self, uname: &str) -> Result<Option<User>, RepoError>;
    fn update_user_password(&mut self, id: i32, new_hash: &str) -> Result<(), RepoError>;
}

pub trait RequirementsRepository {
    fn get_requirement_by_id(&self, id: i32) -> Result<Requirement, RepoError>;
    fn get_requirements_all(&self) -> Result<Vec<Requirement>, RepoError>;
    fn get_requirements_by_project(&self, project_id: i32) -> Result<Vec<Requirement>, RepoError>;
}

pub trait TestsRepository {
    fn get_test_by_id(&self, id: i32) -> Result<Test, RepoError>;
    fn get_tests_all(&self) -> Result<Vec<Test>, RepoError>;
    fn get_tests_by_project(&self, project_id: i32) -> Result<Vec<Test>, RepoError>;
    fn get_requirements_for_test(&self, test_id: i32) -> Result<Vec<Requirement>, RepoError>;
}

pub trait LookupRepository {
    fn get_status_all(&self) -> Result<Vec<Status>, RepoError>;
    fn get_status_by_id(&self, id: i32) -> Result<Status, RepoError>;

    fn get_categories_all(&self) -> Result<Vec<Category>, RepoError>;
    fn get_categories_by_project(&self, project_id: i32) -> Result<Vec<Category>, RepoError>;
    fn get_category_by_id(&self, id: i32) -> Result<Category, RepoError>;

    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError>;
    fn get_applicability_by_id(&self, id: i32) -> Result<Applicability, RepoError>;
    fn get_applicability_by_project(&self, project_id: i32,) -> Result<Vec<Applicability>, RepoError>;

    fn get_verification_all(&self) -> Result<Vec<Verification>, RepoError>;
    fn get_verification_by_id(&self, id: i32) -> Result<Verification, RepoError>;
    fn get_verification_by_project(&self, project_id: i32) -> Result<Vec<Verification>, RepoError>;
}

pub trait ProjectsRepository {
    fn get_projects_all(&self) -> Result<Vec<Project>, RepoError>;
    fn get_project_by_id(&self, id: i32) -> Result<Project, RepoError>;
}

pub trait MatrixRepository {
    fn get_matrix_by_project(&self, project_id: i32) -> Result<Vec<Matrix>, RepoError>;
}

pub trait Repository:
    UserRepository
    + LookupRepository
    + RequirementsRepository
    + TestsRepository
    + ProjectsRepository
    + MatrixRepository
{ }

impl<T> Repository for T where
    T: UserRepository
        + LookupRepository
        + RequirementsRepository
        + TestsRepository
        + ProjectsRepository
        + MatrixRepository
{ }

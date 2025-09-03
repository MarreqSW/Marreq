pub mod errors;
pub mod diesel_repo;
pub mod fake_repo;

pub use diesel_repo::*;

use crate::models::*;
use errors::RepoError;

pub trait UserRepository {
    fn get_user_by_id(&self, id: i32) -> Result<User, RepoError>;
    fn get_user_by_username(&self, uname: &str) -> Result<Option<User>, RepoError>;
    fn update_user_password(&mut self, id: i32, new_hash: &str) -> Result<(), RepoError>;
}

pub trait LookupRepository {
    fn get_status_all(&self) -> Result<Vec<Status>, RepoError>;
    fn get_status_by_id(&self, id: i32) -> Result<Status, RepoError>;
    fn get_categories_all(&self) -> Result<Vec<Category>, RepoError>;
    fn get_category_by_id(&self, id: i32) -> Result<Category, RepoError>;
    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError>;
    fn get_applicability_by_id(&self, id: i32) -> Result<Applicability, RepoError>;
    fn get_verification_all(&self) -> Result<Vec<Verification>, RepoError>;
    fn get_verification_by_id(&self, id: i32) -> Result<Verification, RepoError>;
}

pub trait Repository: UserRepository + LookupRepository {}
impl<T: UserRepository + LookupRepository> Repository for T {}

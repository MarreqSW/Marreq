pub mod errors;
pub mod diesel_repo;
pub mod fake_repo;

pub use diesel_repo::*;

use crate::models::*;
use errors::RepoError;

pub trait Repository {

    fn get_user_by_id(&self, id: i32) -> Result<User, RepoError>;
    fn get_user_by_username(&self, uname: &str) -> Result<Option<User>, RepoError>;
    // TODO: Migrate other queries to this trait

    fn update_user_password(&mut self, id: i32, new_hash: &str) -> Result<(), RepoError>;
}

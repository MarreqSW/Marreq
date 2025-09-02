pub mod errors;
pub mod diesel_repo;

pub use diesel_repo::*;

use crate::models::*;

pub trait Repository {
    fn get_user_by_id(&self, id: i32) -> Result<User, errors::RepoError>;
    // TODO: Migrate other queries to this trait
}

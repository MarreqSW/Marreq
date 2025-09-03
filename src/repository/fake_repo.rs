// This is just for testing purposes

use super::*;
use crate::repository::errors::RepoError;
use std::collections::HashMap;
use chrono::{NaiveDate, NaiveDateTime};

#[derive(Default)]
pub struct FakeRepo {
    pub users: HashMap<i32, User>,
    pub force_err: bool,
}

fn epoch() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()
        .and_hms_opt(0, 0, 0).unwrap()
}

impl FakeRepo {
    pub fn with_users(users: impl IntoIterator<Item = User>) -> Self {
        let mut map = HashMap::new();
        for u in users {
            map.insert(u.user_id, u);
        }
        Self { users: map, force_err: false }
    }
    pub fn with_error() -> Self {
        Self { users: HashMap::new(), force_err: true }
    }

    pub fn make_user(id: i32, username: &str, stored_pw: &str) -> User {
        User {
            user_id: id,
            user_username: username.to_string(),
            user_name: "name".into(),
            user_email: "email@example.com".into(),
            user_level: 0,
            user_creation_date: epoch(),
            user_last_login: epoch(),
            user_password: stored_pw.into(),
            project_id: None,
            is_admin: false,
        }
    }

}

impl UserRepository for FakeRepo {
    fn get_user_by_id(&self, id: i32) -> Result<User, RepoError> {
        self.users.get(&id).cloned().ok_or(RepoError::NotFound)
    }
    fn get_user_by_username(&self, uname: &str) -> Result<Option<User>, RepoError> {
        if self.force_err {
            return Err(RepoError::Pool("forced test error".into()));
        }
        Ok(self.users.values().find(|u| u.user_username == uname).cloned())
    }
    fn update_user_password(&mut self, id: i32, new_hash: &str) -> Result<(), RepoError> {
        if self.force_err {
            return Err(RepoError::Db(diesel::result::Error::RollbackTransaction));
        }
        match self.users.get_mut(&id) {
            Some(user) => {
                user.user_password = new_hash.to_string();
                Ok(())
            }
            None => Err(RepoError::NotFound),
        }
    }
}

impl LookupRepository for FakeRepo {
    fn get_status_all(&self) -> Result<Vec<Status>, RepoError> {
        Ok(Vec::new())
    }
    fn get_status_by_id(&self, _id: i32) -> Result<Status, RepoError> {
        Err(RepoError::NotFound)
    }
    fn get_categories_all(&self) -> Result<Vec<Category>, RepoError> {
        Ok(Vec::new())
    }
    fn get_category_by_id(&self, _id: i32) -> Result<Category, RepoError> {
        Err(RepoError::NotFound)
    }
    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError> {
        Ok(Vec::new())
    }
    fn get_applicability_by_id(&self, _id: i32) -> Result<Applicability, RepoError> {
        Err(RepoError::NotFound)
    }
    fn get_verification_all(&self) -> Result<Vec<Verification>, RepoError> {
        Ok(Vec::new())
    }
    fn get_verification_by_id(&self, _id: i32) -> Result<Verification, RepoError> {
        Err(RepoError::NotFound)
    }
}

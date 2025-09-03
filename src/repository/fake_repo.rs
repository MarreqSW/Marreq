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

    fn get_users_all(&self) -> Result<Vec<User>, RepoError> {
        Ok(self.users.values().cloned().collect())
    }

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

    fn insert_new_user(&mut self, new: &NewUser) -> Result<i32, RepoError> {
        Ok(new.user_id.unwrap_or(0))
    }

    fn update_user(&mut self, _user_data: &NewUser) -> Result<bool, RepoError> {
        Ok(true)
    }

    fn update_user_without_password(
        &mut self,
        _user_data: &UpdateUser,
    ) -> Result<bool, RepoError> {
        Ok(true)
    }

    fn delete_user(&mut self, _id: i32) -> Result<bool, RepoError> {
        Ok(true)
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

    fn get_categories_by_project(&self, _project_id: i32) -> Result<Vec<Category>, RepoError> {
        Ok(Vec::new())
    }


    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError> {
        Ok(Vec::new())
    }

    fn get_applicability_by_id(&self, _id: i32) -> Result<Applicability, RepoError> {
        Err(RepoError::NotFound)
    }

    fn get_applicability_by_project(&self, _project_id: i32,) -> Result<Vec<Applicability>, RepoError> {
        Ok(Vec::new())
    }


    fn get_verification_all(&self) -> Result<Vec<Verification>, RepoError> {
        Ok(Vec::new())
    }

    fn get_verification_by_id(&self, _id: i32) -> Result<Verification, RepoError> {
        Err(RepoError::NotFound)
    }

    fn get_verification_by_project(&self, _project_id: i32) -> Result<Vec<Verification>, RepoError> {
        Ok(Vec::new())
    }

    fn insert_new_category(&mut self, _new: &NewCategory) -> Result<i32, RepoError> {
        Ok(0)
    }
    fn edit_category(&mut self, _new: &NewCategory) -> Result<bool, RepoError> {
        Ok(false)
    }
    fn delete_category(&mut self, _id: i32) -> Result<bool, RepoError> {
        Ok(false)
    }
    fn insert_new_applicability(&mut self, _new: &NewApplicability) -> Result<i32, RepoError> {
        Ok(0)
    }
    fn edit_applicability(&mut self, _new: &NewApplicability) -> Result<bool, RepoError> {
        Ok(false)
    }
    fn delete_applicability(&mut self, _id: i32) -> Result<bool, RepoError> {
        Ok(false)
    }
    fn create_status(&mut self, _new: &NewStatus) -> Result<i32, RepoError> {
        Ok(0)
    }

}

impl RequirementsRepository for FakeRepo {

    fn get_requirement_by_id(&self, _id: i32) -> Result<Requirement, RepoError> {
        Err(RepoError::NotFound)
    }

    fn get_requirements_all(&self) -> Result<Vec<Requirement>, RepoError> {
        Ok(Vec::new())
    }

    fn get_requirements_by_project(&self, _project_id: i32) -> Result<Vec<Requirement>, RepoError> {
        Ok(Vec::new())
    }

    fn insert_new_requirement(&mut self, _new: &NewRequirement) -> Result<i32, RepoError> {
        Ok(0)
    }

    fn edit_requirement(&mut self, _new: &NewRequirement) -> Result<bool, RepoError> {
        Ok(false)
    }

    fn delete_requirement(&mut self, _id: i32) -> Result<bool, RepoError> {
        Ok(false)
    }

    fn update_requirement(&mut self, _req: i32) -> Result<(), RepoError> {
        Ok(())
    }

}

impl TestsRepository for FakeRepo {

    fn get_test_by_id(&self, _id: i32) -> Result<Test, RepoError> {
        Err(RepoError::NotFound)
    }

    fn get_tests_all(&self) -> Result<Vec<Test>, RepoError> {
        Ok(Vec::new())
    }

    fn get_tests_by_project(&self, _project_id: i32) -> Result<Vec<Test>, RepoError> {
        Ok(Vec::new())
    }

    fn get_requirements_for_test(&self, _test_id: i32) -> Result<Vec<Requirement>, RepoError> {
        Ok(Vec::new())
    }

    fn insert_new_test(&mut self, _new: &NewTest) -> Result<i32, RepoError> {
        Ok(0)
    }

    fn edit_test(&mut self, _new: &NewTest) -> Result<bool, RepoError> {
        Ok(false)
    }

    fn delete_test(&mut self, _id: i32) -> Result<bool, RepoError> {
        Ok(false)
    }

    fn update_test_requirement_links(
        &mut self,
        _test_id: i32,
        _requirement_ids: &[i32],
    ) -> Result<(), RepoError> {
        Ok(())
    }

}

impl ProjectsRepository for FakeRepo {

    fn get_projects_all(&self) -> Result<Vec<Project>, RepoError> {
        Ok(Vec::new())
    }

    fn get_project_by_id(&self, _id: i32) -> Result<Project, RepoError> {
        Err(RepoError::NotFound)
    }

    fn insert_new_project(&mut self, _new: &NewProject) -> Result<i32, RepoError> {
        Ok(0)
    }

    fn edit_project(&mut self, _project_id: i32, _update: &UpdateProject) -> Result<bool, RepoError> {
        Ok(false)
    }

    fn delete_project(&mut self, _project_id: i32) -> Result<bool, RepoError> {
        Ok(false)
    }
}

impl MatrixRepository for FakeRepo {

    fn get_matrix_by_project(&self, _project_id: i32) -> Result<Vec<Matrix>, RepoError> {
        Ok(Vec::new())
    }

    fn insert_new_matrix_item(&mut self, _new: &NewMatrix) -> Result<(), RepoError> {
        Ok(())
    }
}

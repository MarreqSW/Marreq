// This is just for testing purposes

use super::*;
use crate::repository::errors::RepoError;
use chrono::{NaiveDate, NaiveDateTime};
use std::collections::HashMap;

#[derive(Default)]
pub struct FakeRepo {
    pub users: HashMap<i32, User>,
    pub statuses: HashMap<i32, Status>,
    pub requirement_statuses: HashMap<i32, RequirementStatus>,
    pub test_statuses: HashMap<i32, TestStatus>,
    pub verifications: HashMap<i32, Verification>,
    pub categories: HashMap<i32, Category>,
    pub applicability: HashMap<i32, Applicability>,
    pub requirements: HashMap<i32, Requirement>,
    pub tests: HashMap<i32, Test>,
    pub projects: HashMap<i32, Project>,
    pub matrices: Vec<Matrix>,
    pub project_members: Vec<ProjectMember>,
    pub force_err: bool,
}

fn epoch() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(1970, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

impl FakeRepo {
    pub fn with_users(users: impl IntoIterator<Item = User>) -> Self {
        let mut map = HashMap::new();
        for u in users {
            map.insert(u.user_id, u);
        }
        Self {
            users: map,
            statuses: HashMap::new(),
            requirement_statuses: HashMap::new(),
            test_statuses: HashMap::new(),
            verifications: HashMap::new(),
            categories: HashMap::new(),
            applicability: HashMap::new(),
            requirements: HashMap::new(),
            tests: HashMap::new(),
            projects: HashMap::new(),
            matrices: Vec::new(),
            project_members: Vec::new(),
            force_err: false,
        }
    }
    pub fn with_error() -> Self {
        Self {
            users: HashMap::new(),
            statuses: HashMap::new(),
            requirement_statuses: HashMap::new(),
            test_statuses: HashMap::new(),
            verifications: HashMap::new(),
            categories: HashMap::new(),
            applicability: HashMap::new(),
            requirements: HashMap::new(),
            tests: HashMap::new(),
            projects: HashMap::new(),
            matrices: Vec::new(),
            project_members: Vec::new(),
            force_err: true,
        }
    }

    pub fn make_user(id: i32, username: &str, stored_pw: &str) -> User {
        User {
            user_id: id,
            user_username: username.to_string(),
            user_name: "name".into(),
            user_email: "email@example.com".into(),
            user_creation_date: epoch(),
            user_last_login: epoch(),
            user_password: stored_pw.into(),
            is_admin: false,
        }
    }

    pub fn get_conn(&self) -> Result<PooledConnectionWrapper, RepoError> {
        Err(RepoError::Pool(
            "fake repository has no database connection".into(),
        ))
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
        Ok(self
            .users
            .values()
            .find(|u| u.user_username == uname)
            .cloned())
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

    fn insert_user(&mut self, new: &NewUser) -> Result<i32, RepoError> {
        let id = new
            .user_id
            .unwrap_or_else(|| self.users.keys().max().map(|i| i + 1).unwrap_or(1));
        let user = User {
            user_id: id,
            user_username: new.user_username.clone(),
            user_name: new.user_name.clone(),
            user_email: new.user_email.clone(),
            user_creation_date: epoch(),
            user_last_login: epoch(),
            user_password: new.user_password.clone(),
            is_admin: new.is_admin,
        };
        self.users.insert(id, user);
        Ok(id)
    }

    fn update_user(&mut self, user_data: &NewUser) -> Result<bool, RepoError> {
        let id = user_data.user_id.ok_or(RepoError::NotFound)?;
        match self.users.get_mut(&id) {
            Some(user) => {
                user.user_username = user_data.user_username.clone();
                user.user_name = user_data.user_name.clone();
                user.user_email = user_data.user_email.clone();
                user.user_password = user_data.user_password.clone();
                user.is_admin = user_data.is_admin;
                Ok(true)
            }
            None => Err(RepoError::NotFound),
        }
    }

    fn update_user_without_password(&mut self, user_data: &UpdateUser) -> Result<bool, RepoError> {
        let id = user_data.user_id.ok_or(RepoError::NotFound)?;
        match self.users.get_mut(&id) {
            Some(user) => {
                user.user_username = user_data.user_username.clone();
                user.user_name = user_data.user_name.clone();
                user.user_email = user_data.user_email.clone();
                user.is_admin = user_data.is_admin;
                Ok(true)
            }
            None => Err(RepoError::NotFound),
        }
    }

    fn delete_user(&mut self, id: i32) -> Result<User, RepoError> {
        let user = self.users.remove(&id).ok_or(RepoError::NotFound)?;
        self.project_members.retain(|pm| pm.user_id != id);
        Ok(user)
    }
}

impl LookupRepository for FakeRepo {
    fn get_status_all(&self) -> Result<Vec<Status>, RepoError> {
        Ok(self.statuses.values().cloned().collect())
    }

    fn get_status_by_id(&self, id: i32) -> Result<Status, RepoError> {
        self.statuses.get(&id).cloned().ok_or(RepoError::NotFound)
    }

    fn get_requirement_status_all(&self) -> Result<Vec<RequirementStatus>, RepoError> {
        Ok(self.requirement_statuses.values().cloned().collect())
    }

    fn get_requirement_status_by_id(&self, id: i32) -> Result<RequirementStatus, RepoError> {
        self.requirement_statuses
            .get(&id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    fn get_test_status_all(&self) -> Result<Vec<TestStatus>, RepoError> {
        Ok(self.test_statuses.values().cloned().collect())
    }

    fn get_test_status_by_id(&self, id: i32) -> Result<TestStatus, RepoError> {
        self.test_statuses
            .get(&id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    fn get_categories_all(&self) -> Result<Vec<Category>, RepoError> {
        Ok(self.categories.values().cloned().collect())
    }

    fn get_category_by_id(&self, id: i32) -> Result<Category, RepoError> {
        self.categories.get(&id).cloned().ok_or(RepoError::NotFound)
    }

    fn get_categories_by_project(&self, project_id: i32) -> Result<Vec<Category>, RepoError> {
        Ok(self
            .categories
            .values()
            .filter(|c| c.project_id == project_id)
            .cloned()
            .collect())
    }

    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError> {
        Ok(self.applicability.values().cloned().collect())
    }

    fn get_applicability_by_id(&self, id: i32) -> Result<Applicability, RepoError> {
        self.applicability
            .get(&id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    fn get_applicability_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<Applicability>, RepoError> {
        Ok(self
            .applicability
            .values()
            .filter(|a| a.project_id == project_id)
            .cloned()
            .collect())
    }

    fn get_verification_all(&self) -> Result<Vec<Verification>, RepoError> {
        Ok(self.verifications.values().cloned().collect())
    }

    fn get_verification_by_id(&self, id: i32) -> Result<Verification, RepoError> {
        self.verifications
            .get(&id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    fn get_verification_by_project(&self, project_id: i32) -> Result<Vec<Verification>, RepoError> {
        Ok(self
            .verifications
            .values()
            .filter(|v| v.project_id == project_id)
            .cloned()
            .collect())
    }

    fn insert_new_category(&mut self, _new: &NewCategory) -> Result<i32, RepoError> {
        let id = _new
            .cat_id
            .unwrap_or_else(|| self.categories.keys().max().map(|i| i + 1).unwrap_or(1));
        let cat = Category {
            cat_id: id,
            cat_title: _new.cat_title.clone(),
            cat_description: _new.cat_description.clone(),
            cat_tag: _new.cat_tag.clone(),
            project_id: _new.project_id,
        };
        self.categories.insert(id, cat);
        Ok(id)
    }
    fn edit_category(&mut self, _new: &NewCategory) -> Result<bool, RepoError> {
        let id = _new.cat_id.ok_or(RepoError::NotFound)?;
        match self.categories.get_mut(&id) {
            Some(cat) => {
                cat.cat_title = _new.cat_title.clone();
                cat.cat_description = _new.cat_description.clone();
                cat.cat_tag = _new.cat_tag.clone();
                cat.project_id = _new.project_id;
                Ok(true)
            }
            None => Err(RepoError::NotFound),
        }
    }
    fn delete_category(&mut self, id: i32) -> Result<Category, RepoError> {
        self.categories.remove(&id).ok_or(RepoError::NotFound)
    }
    fn insert_new_applicability(&mut self, _new: &NewApplicability) -> Result<i32, RepoError> {
        let id = _new
            .app_id
            .unwrap_or_else(|| self.applicability.keys().max().map(|i| i + 1).unwrap_or(1));
        let app = Applicability {
            app_id: id,
            app_title: _new.app_title.clone(),
            app_description: _new.app_description.clone(),
            app_tag: _new.app_tag.clone(),
            project_id: _new.project_id,
        };
        self.applicability.insert(id, app);
        Ok(id)
    }
    fn edit_applicability(&mut self, _new: &NewApplicability) -> Result<bool, RepoError> {
        let id = _new.app_id.ok_or(RepoError::NotFound)?;
        match self.applicability.get_mut(&id) {
            Some(app) => {
                app.app_title = _new.app_title.clone();
                app.app_description = _new.app_description.clone();
                app.app_tag = _new.app_tag.clone();
                app.project_id = _new.project_id;
                Ok(true)
            }
            None => Err(RepoError::NotFound),
        }
    }
    fn delete_applicability(&mut self, id: i32) -> Result<Applicability, RepoError> {
        self.applicability.remove(&id).ok_or(RepoError::NotFound)
    }
    fn create_status(&mut self, _new: &NewStatus) -> Result<i32, RepoError> {
        let id = self.statuses.keys().max().map(|i| i + 1).unwrap_or(1);
        let status = Status {
            st_id: id,
            st_title: _new.req_st_title.clone(),
            st_description: _new.req_st_description.clone(),
            st_short_name: _new.req_st_short_name.clone(),
        };
        self.statuses.insert(id, status);
        Ok(id)
    }
}

impl RequirementsRepository for FakeRepo {
    fn get_requirement_by_id(&self, id: i32) -> Result<Requirement, RepoError> {
        self.requirements
            .get(&id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    fn get_requirements_all(&self) -> Result<Vec<Requirement>, RepoError> {
        Ok(self.requirements.values().cloned().collect())
    }

    fn get_requirements_by_project(&self, project_id: i32) -> Result<Vec<Requirement>, RepoError> {
        Ok(self
            .requirements
            .values()
            .filter(|r| r.project_id == project_id)
            .cloned()
            .collect())
    }

    fn insert_new_requirement(&mut self, _new: &NewRequirement) -> Result<i32, RepoError> {
        let id = _new
            .req_id
            .unwrap_or_else(|| self.requirements.keys().max().map(|i| i + 1).unwrap_or(1));
        let now = epoch();
        let req = Requirement {
            req_id: id,
            req_title: _new.req_title.clone(),
            req_description: _new.req_description.clone(),
            req_verification: _new.req_verification,
            req_current_status: _new.req_current_status,
            req_author: _new.req_author,
            req_reviewer: _new.req_reviewer,
            req_link: _new.req_link.clone(),
            req_reference: _new.req_reference.clone(),
            req_category: _new.req_category,
            req_parent: _new.req_parent,
            req_creation_date: now,
            req_update_date: now,
            req_deadline_date: now,
            req_applicability: _new.req_applicability,
            req_justification: _new.req_justification.clone(),
            project_id: _new.project_id,
        };
        self.requirements.insert(id, req);
        Ok(id)
    }

    fn edit_requirement(&mut self, _new: &NewRequirement) -> Result<bool, RepoError> {
        let id = _new.req_id.ok_or(RepoError::NotFound)?;
        match self.requirements.get_mut(&id) {
            Some(req) => {
                req.req_title = _new.req_title.clone();
                req.req_description = _new.req_description.clone();
                req.req_verification = _new.req_verification;
                req.req_current_status = _new.req_current_status;
                req.req_author = _new.req_author;
                req.req_reviewer = _new.req_reviewer;
                req.req_link = _new.req_link.clone();
                req.req_reference = _new.req_reference.clone();
                req.req_category = _new.req_category;
                req.req_parent = _new.req_parent;
                req.req_applicability = _new.req_applicability;
                req.req_justification = _new.req_justification.clone();
                req.project_id = _new.project_id;
                req.req_update_date = epoch();
                Ok(true)
            }
            None => Err(RepoError::NotFound),
        }
    }

    fn delete_requirement(&mut self, id: i32) -> Result<Requirement, RepoError> {
        self.requirements.remove(&id).ok_or(RepoError::NotFound)
    }

    fn update_requirement(&mut self, _req: i32) -> Result<(), RepoError> {
        match self.requirements.get_mut(&_req) {
            Some(req) => {
                req.req_update_date = epoch();
                Ok(())
            }
            None => Err(RepoError::NotFound),
        }
    }
}

impl TestsRepository for FakeRepo {
    fn get_test_by_id(&self, id: i32) -> Result<Test, RepoError> {
        self.tests.get(&id).cloned().ok_or(RepoError::NotFound)
    }

    fn get_tests_all(&self) -> Result<Vec<Test>, RepoError> {
        Ok(self.tests.values().cloned().collect())
    }

    fn get_tests_by_project(&self, project_id: i32) -> Result<Vec<Test>, RepoError> {
        Ok(self
            .tests
            .values()
            .filter(|t| t.project_id == project_id)
            .cloned()
            .collect())
    }

    fn get_requirements_for_test(&self, test_id: i32) -> Result<Vec<Requirement>, RepoError> {
        let ids: Vec<i32> = self
            .matrices
            .iter()
            .filter(|m| m.matrix_test_id == test_id)
            .map(|m| m.matrix_req_id)
            .collect();
        Ok(ids
            .into_iter()
            .filter_map(|id| self.requirements.get(&id).cloned())
            .collect())
    }

    fn get_tests_for_requirement(&self, req_id: i32) -> Result<Vec<Test>, RepoError> {
        let ids: Vec<i32> = self
            .matrices
            .iter()
            .filter(|m| m.matrix_req_id == req_id)
            .map(|m| m.matrix_test_id)
            .collect();
        Ok(ids
            .into_iter()
            .filter_map(|id| self.tests.get(&id).cloned())
            .collect())
    }

    fn insert_test(&mut self, _new: &NewTest) -> Result<i32, RepoError> {
        let id = _new
            .test_id
            .unwrap_or_else(|| self.tests.keys().max().map(|i| i + 1).unwrap_or(1));
        let test = Test {
            test_id: id,
            test_name: _new.test_name.clone(),
            test_description: _new.test_description.clone(),
            test_source: _new.test_source.clone(),
            test_status: _new.test_status,
            test_reference: _new.test_reference.clone(),
            test_parent: _new.test_parent,
            project_id: _new.project_id,
        };
        self.tests.insert(id, test);
        Ok(id)
    }

    fn edit_test(&mut self, _new: &NewTest) -> Result<bool, RepoError> {
        let id = _new.test_id.ok_or(RepoError::NotFound)?;
        match self.tests.get_mut(&id) {
            Some(test) => {
                test.test_name = _new.test_name.clone();
                test.test_description = _new.test_description.clone();
                test.test_source = _new.test_source.clone();
                test.test_status = _new.test_status;
                test.test_parent = _new.test_parent;
                test.project_id = _new.project_id;
                Ok(true)
            }
            None => Err(RepoError::NotFound),
        }
    }

    fn delete_test(&mut self, id: i32) -> Result<Test, RepoError> {
        self.tests.remove(&id).ok_or(RepoError::NotFound)
    }

    fn update_test_requirement_links(
        &mut self,
        _test_id: i32,
        _requirement_ids: &[i32],
    ) -> Result<(), RepoError> {
        // Remove existing links for this test
        self.matrices.retain(|m| m.matrix_test_id != _test_id);
        let project_id = self.tests.get(&_test_id).map(|t| t.project_id).unwrap_or(0);
        for &req_id in _requirement_ids {
            self.matrices.push(Matrix {
                matrix_req_id: req_id,
                matrix_test_id: _test_id,
                matrix_creation_date: epoch(),
                project_id,
            });
        }
        Ok(())
    }
}

impl ProjectsRepository for FakeRepo {
    fn get_projects_all(&self) -> Result<Vec<Project>, RepoError> {
        Ok(self.projects.values().cloned().collect())
    }

    fn get_project_by_id(&self, _id: i32) -> Result<Project, RepoError> {
        self.projects.get(&_id).cloned().ok_or(RepoError::NotFound)
    }

    fn insert_new_project(&mut self, _new: &NewProject) -> Result<i32, RepoError> {
        let id = self.projects.keys().max().map(|i| i + 1).unwrap_or(1);
        let now = epoch();
        let proj = Project {
            project_id: id,
            project_name: _new.project_name.clone(),
            project_description: _new.project_description.clone(),
            project_creation_date: Some(now),
            project_update_date: Some(now),
            project_status: Some(_new.project_status.clone()),
            project_owner_id: _new.project_owner_id,
        };
        self.projects.insert(id, proj);
        Ok(id)
    }

    fn edit_project(
        &mut self,
        _project_id: i32,
        _update: &UpdateProject,
    ) -> Result<bool, RepoError> {
        match self.projects.get_mut(&_project_id) {
            Some(proj) => {
                proj.project_name = _update.project_name.clone();
                proj.project_description = _update.project_description.clone();
                proj.project_status = Some(_update.project_status.clone());
                proj.project_owner_id = _update.project_owner_id;
                proj.project_update_date = Some(epoch());
                Ok(true)
            }
            None => Err(RepoError::NotFound),
        }
    }

    fn delete_project(&mut self, _project_id: i32) -> Result<Project, RepoError> {
        self.projects
            .remove(&_project_id)
            .ok_or(RepoError::NotFound)
    }
}

impl MatrixRepository for FakeRepo {
    fn get_matrix_by_project(&self, project_id: i32) -> Result<Vec<Matrix>, RepoError> {
        Ok(self
            .matrices
            .iter()
            .filter(|m| m.project_id == project_id)
            .cloned()
            .collect())
    }

    fn insert_new_matrix_item(&mut self, new: &NewMatrix) -> Result<(), RepoError> {
        self.matrices.push(Matrix {
            matrix_req_id: new.matrix_req_id,
            matrix_test_id: new.matrix_test_id,
            matrix_creation_date: epoch(),
            project_id: new.project_id,
        });
        Ok(())
    }
}

impl ProjectMembersRepository for FakeRepo {
    fn get_members_by_project(&self, project_id: i32) -> Result<Vec<ProjectMember>, RepoError> {
        Ok(self
            .project_members
            .iter()
            .filter(|pm| pm.project_id == project_id)
            .cloned()
            .collect())
    }

    fn get_projects_for_user(&self, user_id: i32) -> Result<Vec<ProjectMember>, RepoError> {
        Ok(self
            .project_members
            .iter()
            .filter(|pm| pm.user_id == user_id)
            .cloned()
            .collect())
    }

    fn add_project_member(&mut self, new: &NewProjectMember) -> Result<(), RepoError> {
        if self.force_err {
            return Err(RepoError::Db(diesel::result::Error::RollbackTransaction));
        }

        self.project_members
            .retain(|pm| !(pm.project_id == new.project_id && pm.user_id == new.user_id));
        self.project_members.push(ProjectMember {
            project_id: new.project_id,
            user_id: new.user_id,
            role: new.role,
            created_at: epoch(),
            updated_at: epoch(),
        });
        Ok(())
    }

    fn update_project_member_role(
        &mut self,
        project_id: i32,
        user_id: i32,
        role: i32,
    ) -> Result<(), RepoError> {
        if self.force_err {
            return Err(RepoError::Db(diesel::result::Error::RollbackTransaction));
        }

        match self
            .project_members
            .iter_mut()
            .find(|pm| pm.project_id == project_id && pm.user_id == user_id)
        {
            Some(pm) => {
                pm.role = role;
                pm.updated_at = epoch();
                Ok(())
            }
            None => Err(RepoError::NotFound),
        }
    }

    fn remove_project_member(&mut self, project_id: i32, user_id: i32) -> Result<(), RepoError> {
        let len_before = self.project_members.len();
        self.project_members
            .retain(|pm| !(pm.project_id == project_id && pm.user_id == user_id));
        if self.project_members.len() == len_before {
            Err(RepoError::NotFound)
        } else {
            Ok(())
        }
    }
}

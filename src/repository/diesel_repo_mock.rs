// This is just for testing purposes

use super::*;
use crate::repository::errors::RepoError;
use chrono::{NaiveDate, NaiveDateTime};
use std::collections::HashMap;

#[derive(Default)]
pub struct DieselRepoMock {
    pub users: HashMap<i32, User>,
    pub statuses: HashMap<i32, RequirementStatus>,
    pub requirement_statuses: HashMap<i32, RequirementStatus>,
    pub test_statuses: HashMap<i32, TestStatus>,
    pub verifications: HashMap<i32, VerificationMethod>,
    pub categories: HashMap<i32, Category>,
    pub applicability: HashMap<i32, Applicability>,
    pub requirements: HashMap<i32, Requirement>,
    /// (requirement_id, verification_method_id) pairs for many-to-many
    pub requirement_verification_methods: Vec<(i32, i32)>,
    pub tests: HashMap<i32, TestCase>,
    pub projects: HashMap<i32, Project>,
    pub matrices: Vec<MatrixLink>,
    pub project_members: Vec<ProjectMember>,
    pub logs: Vec<Log>,
    pub force_err: bool,
}

fn epoch() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(1970, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

impl DieselRepoMock {
    pub fn with_users(users: impl IntoIterator<Item = User>) -> Self {
        let mut map = HashMap::new();
        for u in users {
            map.insert(u.id, u);
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
            requirement_verification_methods: Vec::new(),
            tests: HashMap::new(),
            projects: HashMap::new(),
            matrices: Vec::new(),
            project_members: Vec::new(),
            logs: Vec::new(),
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
            requirement_verification_methods: Vec::new(),
            tests: HashMap::new(),
            projects: HashMap::new(),
            matrices: Vec::new(),
            project_members: Vec::new(),
            logs: Vec::new(),
            force_err: true,
        }
    }

    pub fn with_admin_user(mut self) -> Self {
        let mut admin = Self::make_user(1, "admin", "");
        admin.is_admin = true;
        if !self.users.contains_key(&admin.id) {
            self.users.insert(admin.id, admin);
        }
        self
    }

    pub fn make_user(id: i32, username: &str, stored_pw: &str) -> User {
        User {
            id: id,
            username: username.to_string(),
            name: "name".into(),
            email: "email@example.com".into(),
            creation_date: epoch(),
            last_login: epoch(),
            password_hash: stored_pw.into(),
            is_admin: false,
        }
    }

    pub fn get_conn(&self) -> Result<PooledConnectionWrapper, RepoError> {
        Err(RepoError::Pool(
            "fake repository has no database connection".into(),
        ))
    }
}

impl UserRepository for DieselRepoMock {
    fn get_users_all(&self) -> Result<Vec<User>, RepoError> {
        Ok(self.users.values().cloned().collect())
    }

    fn get_user_by_id(&self, user_id: i32) -> Result<User, RepoError> {
        self.users.get(&user_id).cloned().ok_or(RepoError::NotFound)
    }

    fn get_user_by_username(&self, uname: &str) -> Result<Option<User>, RepoError> {
        if self.force_err {
            return Err(RepoError::Pool("forced test error".into()));
        }
        Ok(self.users.values().find(|u| u.username == uname).cloned())
    }

    fn update_user_password(&mut self, user_id: i32, new_hash: &str) -> Result<(), RepoError> {
        if self.force_err {
            return Err(RepoError::Db(diesel::result::Error::RollbackTransaction));
        }
        match self.users.get_mut(&user_id) {
            Some(user) => {
                user.password_hash = new_hash.to_string();
                Ok(())
            }
            None => Err(RepoError::NotFound),
        }
    }

    fn insert_user(&mut self, new: &NewUser) -> Result<i32, RepoError> {
        let id = new
            .id
            .unwrap_or_else(|| self.users.keys().max().map(|i| i + 1).unwrap_or(1));
        let user = User {
            id: id,
            username: new.username.clone(),
            name: new.name.clone(),
            email: new.email.clone(),
            creation_date: epoch(),
            last_login: epoch(),
            password_hash: new.password_hash.clone(),
            is_admin: new.is_admin,
        };
        self.users.insert(id, user);
        Ok(id)
    }

    fn update_user(&mut self, user_data: &NewUser) -> Result<bool, RepoError> {
        let id = user_data.id.ok_or(RepoError::NotFound)?;
        match self.users.get_mut(&id) {
            Some(user) => {
                user.username = user_data.username.clone();
                user.name = user_data.name.clone();
                user.email = user_data.email.clone();
                user.password_hash = user_data.password_hash.clone();
                user.is_admin = user_data.is_admin;
                Ok(true)
            }
            None => Err(RepoError::NotFound),
        }
    }

    fn update_user_without_password(&mut self, user_data: &UpdateUser) -> Result<bool, RepoError> {
        let id = user_data.id.ok_or(RepoError::NotFound)?;
        match self.users.get_mut(&id) {
            Some(user) => {
                user.username = user_data.username.clone();
                user.name = user_data.name.clone();
                user.email = user_data.email.clone();
                user.is_admin = user_data.is_admin;
                Ok(true)
            }
            None => Err(RepoError::NotFound),
        }
    }

    fn delete_user(&mut self, user_id: i32) -> Result<User, RepoError> {
        let user = self.users.remove(&user_id).ok_or(RepoError::NotFound)?;
        self.project_members.retain(|pm| pm.user_id != user_id);
        Ok(user)
    }
}

impl LookupRepository for DieselRepoMock {
    fn get_requirement_status_all(&self) -> Result<Vec<RequirementStatus>, RepoError> {
        Ok(self.requirement_statuses.values().cloned().collect())
    }

    fn get_requirement_status_by_id(&self, status_id: i32) -> Result<RequirementStatus, RepoError> {
        self.requirement_statuses
            .get(&status_id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    fn get_test_status_all(&self) -> Result<Vec<TestStatus>, RepoError> {
        Ok(self.test_statuses.values().cloned().collect())
    }

    fn get_test_status_by_id(&self, status_id: i32) -> Result<TestStatus, RepoError> {
        self.test_statuses
            .get(&status_id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    fn get_categories_all(&self) -> Result<Vec<Category>, RepoError> {
        Ok(self.categories.values().cloned().collect())
    }

    fn get_category_by_id(&self, category_id: i32) -> Result<Category, RepoError> {
        self.categories
            .get(&category_id)
            .cloned()
            .ok_or(RepoError::NotFound)
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

    fn get_applicability_by_id(&self, applicability_id: i32) -> Result<Applicability, RepoError> {
        self.applicability
            .get(&applicability_id)
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

    fn get_verification_all(&self) -> Result<Vec<VerificationMethod>, RepoError> {
        Ok(self.verifications.values().cloned().collect())
    }

    fn get_verification_by_id(
        &self,
        verification_id: i32,
    ) -> Result<VerificationMethod, RepoError> {
        self.verifications
            .get(&verification_id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    fn get_verification_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<VerificationMethod>, RepoError> {
        Ok(self
            .verifications
            .values()
            .filter(|v| v.project_id == project_id)
            .cloned()
            .collect())
    }

    fn insert_new_verification(&mut self, new: &NewVerificationMethod) -> Result<i32, RepoError> {
        let id = new
            .id
            .unwrap_or_else(|| self.verifications.keys().max().map(|i| i + 1).unwrap_or(1));
        let verification = VerificationMethod {
            id: id,
            title: new.title.clone(),
            description: new.description.clone(),
            tag: new.tag.clone(),
            project_id: new.project_id,
        };
        self.verifications.insert(id, verification);
        Ok(id)
    }

    fn insert_new_category(&mut self, _new: &NewCategory) -> Result<i32, RepoError> {
        let id = _new
            .id
            .unwrap_or_else(|| self.categories.keys().max().map(|i| i + 1).unwrap_or(1));
        let cat = Category {
            id: id,
            title: _new.title.clone(),
            description: _new.description.clone(),
            tag: _new.tag.clone(),
            project_id: _new.project_id,
        };
        self.categories.insert(id, cat);
        Ok(id)
    }
    fn edit_category(&mut self, _new: &NewCategory) -> Result<bool, RepoError> {
        let id = _new.id.ok_or(RepoError::NotFound)?;
        match self.categories.get_mut(&id) {
            Some(cat) => {
                cat.title = _new.title.clone();
                cat.description = _new.description.clone();
                cat.tag = _new.tag.clone();
                cat.project_id = _new.project_id;
                Ok(true)
            }
            None => Err(RepoError::NotFound),
        }
    }
    fn delete_category(&mut self, category_id: i32) -> Result<Category, RepoError> {
        self.categories
            .remove(&category_id)
            .ok_or(RepoError::NotFound)
    }
    fn insert_new_applicability(&mut self, _new: &NewApplicability) -> Result<i32, RepoError> {
        let id = _new
            .id
            .unwrap_or_else(|| self.applicability.keys().max().map(|i| i + 1).unwrap_or(1));
        let app = Applicability {
            id: id,
            title: _new.title.clone(),
            description: _new.description.clone(),
            tag: _new.tag.clone(),
            project_id: _new.project_id,
        };
        self.applicability.insert(id, app);
        Ok(id)
    }
    fn edit_applicability(&mut self, _new: &NewApplicability) -> Result<bool, RepoError> {
        let id = _new.id.ok_or(RepoError::NotFound)?;
        match self.applicability.get_mut(&id) {
            Some(app) => {
                app.title = _new.title.clone();
                app.description = _new.description.clone();
                app.tag = _new.tag.clone();
                app.project_id = _new.project_id;
                Ok(true)
            }
            None => Err(RepoError::NotFound),
        }
    }
    fn delete_applicability(&mut self, applicability_id: i32) -> Result<Applicability, RepoError> {
        self.applicability
            .remove(&applicability_id)
            .ok_or(RepoError::NotFound)
    }

    fn create_requirement_status(&mut self, new: &NewRequirementStatus) -> Result<i32, RepoError> {
        let id = new.id.unwrap_or_else(|| {
            self.requirement_statuses
                .keys()
                .max()
                .map(|i| i + 1)
                .unwrap_or(1)
        });
        let status = RequirementStatus {
            id,
            title: new.title.clone(),
            description: new.description.clone(),
            tag: new.tag.clone(),
            project_id: new.project_id,
        };
        self.requirement_statuses.insert(id, status.clone());
        self.statuses.insert(id, status); // Keep backward compat with legacy field
        Ok(id)
    }

    fn create_test_status(&mut self, new: &NewTestStatus) -> Result<i32, RepoError> {
        let id = new
            .id
            .unwrap_or_else(|| self.test_statuses.keys().max().map(|i| i + 1).unwrap_or(1));
        let status = TestStatus {
            id,
            title: new.title.clone(),
            description: new.description.clone(),
            tag: new.tag.clone(),
            project_id: new.project_id,
        };
        self.test_statuses.insert(id, status);
        Ok(id)
    }
}

impl RequirementsRepository for DieselRepoMock {
    fn get_requirement_by_id(&self, requirement_id: i32) -> Result<Requirement, RepoError> {
        self.requirements
            .get(&requirement_id)
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

    fn get_verification_method_ids_for_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<i32>, RepoError> {
        let mut ids: Vec<i32> = self
            .requirement_verification_methods
            .iter()
            .filter(|(req_id, _)| *req_id == requirement_id)
            .map(|(_, ver_id)| *ver_id)
            .collect();
        ids.sort_unstable();
        Ok(ids)
    }

    fn get_requirement_ids_by_verification_method(
        &self,
        verification_method_id: i32,
    ) -> Result<Vec<i32>, RepoError> {
        Ok(self
            .requirement_verification_methods
            .iter()
            .filter(|(_, ver_id)| *ver_id == verification_method_id)
            .map(|(req_id, _)| *req_id)
            .collect())
    }

    fn set_requirement_verification_methods(
        &mut self,
        requirement_id: i32,
        verification_method_ids: &[i32],
    ) -> Result<(), RepoError> {
        self.requirement_verification_methods
            .retain(|(req_id, _)| *req_id != requirement_id);
        for &ver_id in verification_method_ids {
            if ver_id > 0 {
                self.requirement_verification_methods
                    .push((requirement_id, ver_id));
            }
        }
        Ok(())
    }

    fn insert_new_requirement(&mut self, _new: &NewRequirement) -> Result<i32, RepoError> {
        let id = _new
            .id
            .unwrap_or_else(|| self.requirements.keys().max().map(|i| i + 1).unwrap_or(1));
        let now = epoch();
        let req = Requirement {
            id,
            title: _new.title.clone(),
            description: _new.description.clone(),
            status_id: _new.status_id,
            author_id: _new.author_id,
            reviewer_id: _new.reviewer_id,
            reference_code: _new.reference_code.clone(),
            category_id: _new.category_id,
            parent_id: _new.parent_id,
            creation_date: now,
            update_date: now,
            deadline_date: Some(now),
            applicability_id: _new.applicability_id,
            justification: _new.justification.clone(),
            project_id: _new.project_id,
        };
        self.requirements.insert(id, req);
        Ok(id)
    }

    fn edit_requirement(&mut self, _new: &NewRequirement) -> Result<bool, RepoError> {
        let id = _new.id.ok_or(RepoError::NotFound)?;
        match self.requirements.get_mut(&id) {
            Some(req) => {
                req.title = _new.title.clone();
                req.description = _new.description.clone();
                req.status_id = _new.status_id;
                req.author_id = _new.author_id;
                req.reviewer_id = _new.reviewer_id;
                req.reference_code = _new.reference_code.clone();
                req.category_id = _new.category_id;
                req.parent_id = _new.parent_id;
                req.applicability_id = _new.applicability_id;
                req.justification = _new.justification.clone();
                req.project_id = _new.project_id;
                req.update_date = epoch();
                Ok(true)
            }
            None => Err(RepoError::NotFound),
        }
    }

    fn delete_requirement(&mut self, requirement_id: i32) -> Result<Requirement, RepoError> {
        self.requirements
            .remove(&requirement_id)
            .ok_or(RepoError::NotFound)
    }

    fn update_requirement(&mut self, _req: i32) -> Result<(), RepoError> {
        match self.requirements.get_mut(&_req) {
            Some(req) => {
                req.update_date = epoch();
                Ok(())
            }
            None => Err(RepoError::NotFound),
        }
    }
}

impl TestsCaseRepository for DieselRepoMock {
    fn get_test_by_id(&self, test_id: i32) -> Result<TestCase, RepoError> {
        self.tests.get(&test_id).cloned().ok_or(RepoError::NotFound)
    }

    fn get_tests_all(&self) -> Result<Vec<TestCase>, RepoError> {
        Ok(self.tests.values().cloned().collect())
    }

    fn get_tests_by_project(&self, project_id: i32) -> Result<Vec<TestCase>, RepoError> {
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
            .filter(|m| m.test_id == test_id)
            .map(|m| m.req_id)
            .collect();
        Ok(ids
            .into_iter()
            .filter_map(|id| self.requirements.get(&id).cloned())
            .collect())
    }

    fn get_tests_for_requirement(&self, requirement_id: i32) -> Result<Vec<TestCase>, RepoError> {
        let ids: Vec<i32> = self
            .matrices
            .iter()
            .filter(|m| m.req_id == requirement_id)
            .map(|m| m.test_id)
            .collect();
        Ok(ids
            .into_iter()
            .filter_map(|id| self.tests.get(&id).cloned())
            .collect())
    }

    fn insert_test(&mut self, _new: &NewTestCase) -> Result<i32, RepoError> {
        let id = _new
            .id
            .unwrap_or_else(|| self.tests.keys().max().map(|i| i + 1).unwrap_or(1));
        let test = TestCase {
            id: id,
            name: _new.name.clone(),
            description: _new.description.clone(),
            source: _new.source.clone(),
            status_id: _new.status_id,
            reference_code: _new.reference_code.clone(),
            parent_id: _new.parent_id,
            project_id: _new.project_id,
        };
        self.tests.insert(id, test);
        Ok(id)
    }

    fn edit_test(&mut self, _new: &NewTestCase) -> Result<bool, RepoError> {
        let id = _new.id.ok_or(RepoError::NotFound)?;
        match self.tests.get_mut(&id) {
            Some(test) => {
                test.name = _new.name.clone();
                test.description = _new.description.clone();
                test.source = _new.source.clone();
                test.status_id = _new.status_id;
                test.parent_id = _new.parent_id;
                test.project_id = _new.project_id;
                Ok(true)
            }
            None => Err(RepoError::NotFound),
        }
    }

    fn delete_test(&mut self, test_id: i32) -> Result<TestCase, RepoError> {
        self.tests.remove(&test_id).ok_or(RepoError::NotFound)
    }

    fn update_test_requirement_links(
        &mut self,
        _test_id: i32,
        _requirement_ids: &[i32],
    ) -> Result<(), RepoError> {
        // Remove existing links for this test
        self.matrices.retain(|m| m.test_id != _test_id);
        let project_id = self.tests.get(&_test_id).map(|t| t.project_id).unwrap_or(0);
        for &id in _requirement_ids {
            self.matrices.push(MatrixLink {
                req_id: id,
                test_id: _test_id,
                creation_date: epoch(),
                project_id,
            });
        }
        Ok(())
    }
}

impl ProjectsRepository for DieselRepoMock {
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
            id: id,
            name: _new.name.clone(),
            description: _new.description.clone(),
            creation_date: Some(now),
            update_date: Some(now),
            owner_id: _new.owner_id,
            status: _new.status.clone(),
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
                proj.name = _update.name.clone();
                proj.description = _update.description.clone();
                proj.owner_id = _update.owner_id;
                if let Some(status) = _update.status {
                    proj.status = status;
                }
                proj.update_date = Some(epoch());
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

impl MatrixRepository for DieselRepoMock {
    fn get_matrix_by_project(&self, project_id: i32) -> Result<Vec<MatrixLink>, RepoError> {
        Ok(self
            .matrices
            .iter()
            .filter(|m| m.project_id == project_id)
            .cloned()
            .collect())
    }

    fn insert_new_matrix_item(&mut self, new: &NewMatrixLink) -> Result<(), RepoError> {
        self.matrices.push(MatrixLink {
            req_id: new.req_id,
            test_id: new.test_id,
            creation_date: epoch(),
            project_id: new.project_id,
        });
        Ok(())
    }
}

impl ProjectMembersRepository for DieselRepoMock {
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
        id: i32,
        role: i32,
    ) -> Result<(), RepoError> {
        if self.force_err {
            return Err(RepoError::Db(diesel::result::Error::RollbackTransaction));
        }

        match self
            .project_members
            .iter_mut()
            .find(|pm| pm.project_id == project_id && pm.user_id == id)
        {
            Some(pm) => {
                pm.role = role;
                pm.updated_at = epoch();
                Ok(())
            }
            None => Err(RepoError::NotFound),
        }
    }

    fn remove_project_member(&mut self, project_id: i32, id: i32) -> Result<(), RepoError> {
        let len_before = self.project_members.len();
        self.project_members
            .retain(|pm| !(pm.project_id == project_id && pm.user_id == id));
        if self.project_members.len() == len_before {
            Err(RepoError::NotFound)
        } else {
            Ok(())
        }
    }
}

impl LogRepository for DieselRepoMock {
    fn insert_log(&mut self, new_log: &NewLog) -> Result<(), RepoError> {
        let id = self.logs.len() as i32 + 1;
        self.logs.push(Log {
            log_id: id,
            created_at: epoch(),
            user_id: new_log.user_id,
            entity_type: new_log.entity_type.clone(),
            entity_id: new_log.entity_id,
            action_type: new_log.action_type.clone(),
            description: new_log.description.clone(),
            project_id: new_log.project_id,
            old_values: new_log.old_values.clone(),
            new_values: new_log.new_values.clone(),
            ip_address: new_log.ip_address.clone(),
            user_agent: new_log.user_agent.clone(),
        });
        Ok(())
    }

    fn get_logs_recent(&self, limit: i64) -> Result<Vec<Log>, RepoError> {
        Ok(self
            .logs
            .iter()
            .rev()
            .take(limit as usize)
            .cloned()
            .collect())
    }

    fn get_logs_by_entity(&self, entity_type: &str, entity_id: i32) -> Result<Vec<Log>, RepoError> {
        Ok(self
            .logs
            .iter()
            .filter(|l| l.entity_type == entity_type && l.entity_id == Some(entity_id))
            .cloned()
            .collect())
    }

    fn cleanup_logs(&mut self, _days: i64) -> Result<usize, RepoError> {
        let len_before = self.logs.len();
        // In mock, we just clear everything for simplicity or keep it all
        // Let's say we remove nothing as dates are all epoch
        Ok(len_before - self.logs.len())
    }
}

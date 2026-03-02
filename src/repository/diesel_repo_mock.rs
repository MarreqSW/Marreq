// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

// This is just for testing purposes

use super::*;
use crate::models::{
    CustomFieldDefinition, CustomFieldDefinitionPayload, NewRequirementComment,
    NewRequirementVersionLink, RequirementComment, RequirementVersion, RequirementVersionLink,
};
use crate::repository::errors::RepoError;
use chrono::{NaiveDate, NaiveDateTime};
use std::collections::HashMap;

pub struct DieselRepoMock {
    pub users: HashMap<i32, User>,
    pub statuses: HashMap<i32, RequirementStatus>,
    pub requirement_statuses: HashMap<i32, RequirementStatus>,
    pub test_statuses: HashMap<i32, TestStatus>,
    pub verifications: HashMap<i32, VerificationMethod>,
    pub categories: HashMap<i32, Category>,
    pub applicability: HashMap<i32, Applicability>,
    pub requirements: HashMap<i32, Requirement>,
    /// (requirement_id, verification_method_id) pairs for current version (mock)
    pub requirement_verification_methods: Vec<(i32, i32)>,
    /// Version history for tests (version id -> RequirementVersion)
    pub requirement_versions: HashMap<i32, RequirementVersion>,
    /// Next version id when creating versions
    pub next_version_id: i32,
    pub tests: HashMap<i32, TestCase>,
    pub projects: HashMap<i32, Project>,
    pub matrices: Vec<MatrixLink>,
    pub project_members: Vec<ProjectMember>,
    pub logs: Vec<Log>,
    pub force_err: bool,
    pub baselines: Vec<crate::models::Baseline>,
    pub baseline_requirements: Vec<crate::models::BaselineRequirement>,
    pub baseline_traceability: Vec<crate::models::BaselineTraceability>,
    pub next_baseline_id: i32,
    pub custom_field_definitions: HashMap<i32, CustomFieldDefinition>,
    /// (requirement_version_id, custom_field_definition_id, value)
    pub custom_field_values: Vec<(i32, i32, Option<String>)>,
    pub next_custom_field_id: i32,
    pub requirement_comments: Vec<RequirementComment>,
    pub next_comment_id: i32,
    pub requirement_version_links: Vec<RequirementVersionLink>,
    pub next_link_id: i32,
}

fn epoch() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(1970, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

fn version_created_at(version_id: i32) -> NaiveDateTime {
    epoch() + chrono::Duration::seconds(version_id as i64)
}

impl Default for DieselRepoMock {
    fn default() -> Self {
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
            requirement_versions: HashMap::new(),
            next_version_id: 1,
            tests: HashMap::new(),
            projects: HashMap::new(),
            matrices: Vec::new(),
            project_members: Vec::new(),
            logs: Vec::new(),
            force_err: false,
            baselines: Vec::new(),
            baseline_requirements: Vec::new(),
            baseline_traceability: Vec::new(),
            next_baseline_id: 1,
            custom_field_definitions: HashMap::new(),
            custom_field_values: Vec::new(),
            next_custom_field_id: 1,
            requirement_comments: Vec::new(),
            next_comment_id: 1,
            requirement_version_links: Vec::new(),
            next_link_id: 1,
        }
    }
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
            requirement_versions: HashMap::new(),
            next_version_id: 1,
            tests: HashMap::new(),
            projects: HashMap::new(),
            matrices: Vec::new(),
            project_members: Vec::new(),
            logs: Vec::new(),
            force_err: false,
            baselines: Vec::new(),
            baseline_requirements: Vec::new(),
            baseline_traceability: Vec::new(),
            next_baseline_id: 1,
            custom_field_definitions: HashMap::new(),
            custom_field_values: Vec::new(),
            next_custom_field_id: 1,
            requirement_comments: Vec::new(),
            next_comment_id: 1,
            requirement_version_links: Vec::new(),
            next_link_id: 1,
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
            requirement_versions: HashMap::new(),
            next_version_id: 1,
            tests: HashMap::new(),
            projects: HashMap::new(),
            matrices: Vec::new(),
            project_members: Vec::new(),
            logs: Vec::new(),
            force_err: true,
            baselines: Vec::new(),
            baseline_requirements: Vec::new(),
            baseline_traceability: Vec::new(),
            next_baseline_id: 1,
            custom_field_definitions: HashMap::new(),
            custom_field_values: Vec::new(),
            next_custom_field_id: 1,
            requirement_comments: Vec::new(),
            next_comment_id: 1,
            requirement_version_links: Vec::new(),
            next_link_id: 1,
        }
    }

    pub fn with_admin_user(mut self) -> Self {
        let mut admin = Self::make_user(1, "admin", "");
        admin.is_admin = true;
        self.users.entry(admin.id).or_insert(admin);
        self
    }

    pub fn make_user(id: i32, username: &str, stored_pw: &str) -> User {
        User {
            id,
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

impl ApiTokensRepository for DieselRepoMock {
    fn get_user_by_token_hash(&self, _token_hash: &str) -> Result<(User, Option<i32>), RepoError> {
        Err(RepoError::NotFound)
    }

    fn update_api_token_last_used_at(&mut self, _token_hash: &str) -> Result<(), RepoError> {
        Ok(())
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
        let lower = uname.to_lowercase();
        Ok(self
            .users
            .values()
            .find(|u| u.username.to_lowercase() == lower)
            .cloned())
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
        // Enforce case-insensitive uniqueness
        let lower_username = new.username.to_lowercase();
        let lower_email = new.email.to_lowercase();
        if self
            .users
            .values()
            .any(|u| u.username.to_lowercase() == lower_username)
        {
            return Err(RepoError::Duplicate("username is already taken".into()));
        }
        if self
            .users
            .values()
            .any(|u| u.email.to_lowercase() == lower_email)
        {
            return Err(RepoError::Duplicate("email is already in use".into()));
        }
        let id = new
            .id
            .unwrap_or_else(|| self.users.keys().max().map(|i| i + 1).unwrap_or(1));
        let user = User {
            id,
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
            id,
            title: new.title.clone(),
            description: new.description.clone(),
            tag: new.tag.clone(),
            project_id: new.project_id,
        };
        self.verifications.insert(id, verification);
        Ok(id)
    }

    fn edit_verification(&mut self, new: &NewVerificationMethod) -> Result<bool, RepoError> {
        let id = new.id.ok_or(RepoError::NotFound)?;
        match self.verifications.get_mut(&id) {
            Some(v) => {
                v.title = new.title.clone();
                v.description = new.description.clone();
                v.tag = new.tag.clone();
                v.project_id = new.project_id;
                Ok(true)
            }
            None => Err(RepoError::NotFound),
        }
    }

    fn delete_verification(
        &mut self,
        verification_id: i32,
    ) -> Result<VerificationMethod, RepoError> {
        let verification = self
            .verifications
            .remove(&verification_id)
            .ok_or(RepoError::NotFound)?;
        self.requirement_verification_methods
            .retain(|(_, vid)| *vid != verification_id);
        Ok(verification)
    }

    fn insert_new_category(&mut self, _new: &NewCategory) -> Result<i32, RepoError> {
        let id = _new
            .id
            .unwrap_or_else(|| self.categories.keys().max().map(|i| i + 1).unwrap_or(1));
        let cat = Category {
            id,
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
            id,
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
            is_system: new.is_system,
            tag_color: new.tag_color.clone(),
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
            is_system: new.is_system,
            tag_color: new.tag_color.clone(),
        };
        self.test_statuses.insert(id, status);
        Ok(id)
    }

    fn update_requirement_status(
        &mut self,
        id: i32,
        payload: &NewRequirementStatus,
    ) -> Result<bool, RepoError> {
        let status = self
            .requirement_statuses
            .get_mut(&id)
            .ok_or(RepoError::NotFound)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot modify system status".into()));
        }
        status.title = payload.title.clone();
        status.description = payload.description.clone();
        status.tag = payload.tag.clone();
        status.tag_color = payload.tag_color.clone();
        if let Some(ref mut s) = self.statuses.get_mut(&id) {
            s.title = payload.title.clone();
            s.description = payload.description.clone();
            s.tag = payload.tag.clone();
            s.tag_color = payload.tag_color.clone();
        }
        Ok(true)
    }

    fn delete_requirement_status(&mut self, id: i32) -> Result<RequirementStatus, RepoError> {
        let status = self
            .requirement_statuses
            .get(&id)
            .cloned()
            .ok_or(RepoError::NotFound)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot delete system status".into()));
        }
        let in_use = self
            .requirement_versions
            .values()
            .any(|v| v.status_id == id);
        if in_use {
            return Err(RepoError::BadInput(
                "Cannot delete status: it is in use by requirement versions".into(),
            ));
        }
        self.requirement_statuses.remove(&id);
        self.statuses.remove(&id);
        Ok(status)
    }

    fn update_test_status(&mut self, id: i32, payload: &NewTestStatus) -> Result<bool, RepoError> {
        let status = self.test_statuses.get_mut(&id).ok_or(RepoError::NotFound)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot modify system status".into()));
        }
        status.title = payload.title.clone();
        status.description = payload.description.clone();
        status.tag = payload.tag.clone();
        status.tag_color = payload.tag_color.clone();
        Ok(true)
    }

    fn delete_test_status(&mut self, id: i32) -> Result<TestStatus, RepoError> {
        let status = self
            .test_statuses
            .get(&id)
            .cloned()
            .ok_or(RepoError::NotFound)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot delete system status".into()));
        }
        let in_use = self.tests.values().any(|t| t.status_id == id);
        if in_use {
            return Err(RepoError::BadInput(
                "Cannot delete status: it is in use by tests".into(),
            ));
        }
        self.test_statuses.remove(&id).ok_or(RepoError::NotFound)
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

    fn get_requirements_by_project_filtered_paginated(
        &self,
        project_id: i32,
        status_filter: Option<i32>,
        verification_filter: Option<i32>,
        category_filter: Option<i32>,
        applicability_filter: Option<i32>,
        custom_field_filters: Option<&[(i32, String)]>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Requirement>, RepoError> {
        let verification_ids = verification_filter
            .map(|vid| {
                self.get_requirement_ids_by_verification_method(vid)
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        let mut reqs: Vec<Requirement> = self
            .requirements
            .values()
            .filter(|r| {
                r.project_id == project_id
                    && status_filter.is_none_or(|s| r.status_id == s)
                    && verification_filter.is_none_or(|_| verification_ids.contains(&r.id))
                    && category_filter.is_none_or(|c| r.category_id == c)
                    && applicability_filter.is_none_or(|a| r.applicability_id == a)
                    && custom_field_filters.is_none_or(|filters| {
                        r.current_version_id.is_some_and(|vid| {
                            filters.iter().all(|(field_id, value)| {
                                self.custom_field_values.iter().any(|(v, fid, val)| {
                                    *v == vid
                                        && *fid == *field_id
                                        && val.as_deref().unwrap_or("") == value.as_str()
                                })
                            })
                        })
                    })
            })
            .cloned()
            .collect();
        reqs.sort_by(|a, b| {
            match (
                a.reference_code.trim().is_empty(),
                b.reference_code.trim().is_empty(),
            ) {
                (false, false) => a.reference_code.cmp(&b.reference_code),
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                (true, true) => a.id.cmp(&b.id),
            }
        });
        let offset = offset as usize;
        let limit = limit as usize;
        Ok(reqs.into_iter().skip(offset).take(limit).collect())
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

    fn get_verification_method_ids_for_version(
        &self,
        version_id: i32,
    ) -> Result<Vec<i32>, RepoError> {
        let version = self
            .requirement_versions
            .get(&version_id)
            .ok_or(RepoError::NotFound)?;
        let requirement_id = version.requirement_id;
        let is_current = self
            .requirements
            .get(&requirement_id)
            .map(|r| r.current_version_id == Some(version_id))
            .unwrap_or(false);
        if !is_current {
            return Ok(vec![]);
        }
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
        let version_id = self.next_version_id;
        self.next_version_id += 1;
        let created_at = version_created_at(version_id);
        let version = RequirementVersion {
            id: version_id,
            requirement_id: id,
            title: _new.title.clone(),
            description: _new.description.clone(),
            status_id: _new.status_id,
            author_id: _new.author_id,
            reviewer_id: _new.reviewer_id,
            category_id: _new.category_id,
            applicability_id: _new.applicability_id,
            justification: _new.justification.clone(),
            deadline_date: Some(now),
            created_at,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
        };
        self.requirement_versions.insert(version_id, version);
        let req = Requirement {
            id,
            current_version_id: Some(version_id),
            same_as_current: None,
            title: _new.title.clone(),
            description: _new.description.clone(),
            status_id: _new.status_id,
            author_id: _new.author_id,
            reviewer_id: _new.reviewer_id,
            reference_code: _new.reference_code.clone(),
            category_id: _new.category_id,
            parent_id: None,
            creation_date: now,
            update_date: now,
            deadline_date: Some(now),
            applicability_id: _new.applicability_id,
            justification: _new.justification.clone(),
            project_id: _new.project_id,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        };
        self.requirements.insert(id, req);
        Ok(id)
    }

    fn edit_requirement(&mut self, _new: &NewRequirement) -> Result<bool, RepoError> {
        let id = _new.id.ok_or(RepoError::NotFound)?;
        match self.requirements.get_mut(&id) {
            Some(req) => {
                let now = epoch();
                let version_id = self.next_version_id;
                self.next_version_id += 1;
                let created_at = version_created_at(version_id);
                let version = RequirementVersion {
                    id: version_id,
                    requirement_id: id,
                    title: _new.title.clone(),
                    description: _new.description.clone(),
                    status_id: _new.status_id,
                    author_id: _new.author_id,
                    reviewer_id: _new.reviewer_id,
                    category_id: _new.category_id,
                    applicability_id: _new.applicability_id,
                    justification: _new.justification.clone(),
                    deadline_date: Some(now),
                    created_at,
                    approval_state: "draft".to_string(),
                    approved_by: None,
                    approved_at: None,
                };
                self.requirement_versions.insert(version_id, version);
                req.current_version_id = Some(version_id);
                req.title = _new.title.clone();
                req.description = _new.description.clone();
                req.status_id = _new.status_id;
                req.author_id = _new.author_id;
                req.reviewer_id = _new.reviewer_id;
                req.reference_code = _new.reference_code.clone();
                req.category_id = _new.category_id;
                req.applicability_id = _new.applicability_id;
                req.justification = _new.justification.clone();
                req.project_id = _new.project_id;
                req.update_date = now;
                req.approval_state = "draft".to_string();
                req.approved_by = None;
                req.approved_at = None;
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

    fn list_requirement_versions(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<RequirementVersion>, RepoError> {
        let mut versions: Vec<RequirementVersion> = self
            .requirement_versions
            .values()
            .filter(|v| v.requirement_id == requirement_id)
            .cloned()
            .collect();
        versions.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        Ok(versions)
    }

    fn get_requirement_version_by_id(
        &self,
        version_id: i32,
    ) -> Result<RequirementVersion, RepoError> {
        self.requirement_versions
            .get(&version_id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    fn set_requirement_version_approval(
        &mut self,
        version_id: i32,
        new_state: &str,
        approved_by_user_id: i32,
    ) -> Result<RequirementVersion, RepoError> {
        use crate::status_enums::ApprovalState;
        let mut version = self
            .requirement_versions
            .get(&version_id)
            .cloned()
            .ok_or(RepoError::NotFound)?;
        let current = ApprovalState::from_db_string(&version.approval_state).ok_or_else(|| {
            RepoError::BadInput(format!(
                "invalid approval_state: {}",
                version.approval_state
            ))
        })?;
        let target = ApprovalState::from_db_string(new_state)
            .ok_or_else(|| RepoError::BadInput(format!("invalid approval_state: {}", new_state)))?;
        if !current.can_transition_to(target) {
            return Err(RepoError::BadInput(format!(
                "invalid transition: {} -> {}",
                version.approval_state, new_state
            )));
        }
        if current == target {
            return Ok(version);
        }
        version.approval_state = target.to_db_string().to_string();
        if target == ApprovalState::Approved {
            version.approved_by = Some(approved_by_user_id);
            version.approved_at = Some(epoch());
        }
        self.requirement_versions
            .insert(version_id, version.clone());
        if let Some(req) = self.requirements.get_mut(&version.requirement_id) {
            if req.current_version_id == Some(version_id) {
                req.approval_state = version.approval_state.clone();
                req.approved_by = version.approved_by;
                req.approved_at = version.approved_at;
            }
        }
        let _ = self.mark_links_suspect_for_requirement(
            version.requirement_id,
            "Approval state changed",
            Some(version_id),
            Some(approved_by_user_id),
        )?;
        Ok(version)
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

    fn get_impacted_tests_for_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<TestCase>, RepoError> {
        let ids: Vec<i32> = self
            .matrices
            .iter()
            .filter(|m| m.req_id == requirement_id && m.suspect)
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
            id,
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
                suspect: false,
                suspect_at: None,
                suspect_reason: None,
                cleared_by: None,
                cleared_at: None,
                triggering_version_id: None,
                triggering_user_id: None,
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
            id,
            name: _new.name.clone(),
            description: _new.description.clone(),
            creation_date: Some(now),
            update_date: Some(now),
            owner_id: _new.owner_id,
            status: _new.status,
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
            suspect: false,
            suspect_at: None,
            suspect_reason: None,
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: new.triggering_version_id,
            triggering_user_id: new.triggering_user_id,
        });
        Ok(())
    }

    fn mark_links_suspect_for_requirement(
        &mut self,
        requirement_id: i32,
        reason: &str,
        triggering_version_id: Option<i32>,
        triggering_user_id: Option<i32>,
    ) -> Result<Vec<i32>, RepoError> {
        let now = epoch();
        let mut project_ids = Vec::new();
        for link in self.matrices.iter_mut() {
            if link.req_id == requirement_id {
                link.suspect = true;
                link.suspect_at = Some(now);
                link.suspect_reason = Some(reason.to_string());
                link.cleared_by = None;
                link.cleared_at = None;
                link.triggering_version_id = triggering_version_id;
                link.triggering_user_id = triggering_user_id;
                if !project_ids.contains(&link.project_id) {
                    project_ids.push(link.project_id);
                }
            }
        }
        Ok(project_ids)
    }

    fn clear_suspect(
        &mut self,
        req_id: i32,
        test_id: i32,
        cleared_by_user_id: i32,
    ) -> Result<(bool, Option<i32>), RepoError> {
        let now = epoch();
        for link in self.matrices.iter_mut() {
            if link.req_id == req_id && link.test_id == test_id {
                link.suspect = false;
                link.suspect_at = None;
                link.suspect_reason = None;
                link.cleared_by = Some(cleared_by_user_id);
                link.cleared_at = Some(now);
                return Ok((true, Some(link.project_id)));
            }
        }
        Ok((false, None))
    }
}

impl crate::repository::CustomFieldRepository for DieselRepoMock {
    fn list_custom_field_definitions_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<CustomFieldDefinition>, RepoError> {
        let mut defs: Vec<_> = self
            .custom_field_definitions
            .values()
            .filter(|d| d.project_id == project_id)
            .cloned()
            .collect();
        defs.sort_by_key(|d| (d.sort_order, d.id));
        Ok(defs)
    }

    fn get_custom_field_definition_by_id(
        &self,
        id: i32,
    ) -> Result<CustomFieldDefinition, RepoError> {
        self.custom_field_definitions
            .get(&id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    fn create_custom_field_definition(
        &mut self,
        project_id: i32,
        payload: &CustomFieldDefinitionPayload,
    ) -> Result<i32, RepoError> {
        let id = self.next_custom_field_id;
        self.next_custom_field_id += 1;
        let enum_values = payload
            .enum_values
            .as_ref()
            .map(|v| serde_json::to_value(v).unwrap());
        let def = CustomFieldDefinition {
            id,
            project_id,
            label: payload.label.trim().to_string(),
            field_type: payload.field_type.trim().to_lowercase(),
            enum_values,
            sort_order: payload.sort_order.unwrap_or(0),
            created_at: epoch(),
        };
        self.custom_field_definitions.insert(id, def);
        Ok(id)
    }

    fn update_custom_field_definition(
        &mut self,
        id: i32,
        payload: &CustomFieldDefinitionPayload,
    ) -> Result<(), RepoError> {
        let def = self
            .custom_field_definitions
            .get_mut(&id)
            .ok_or(RepoError::NotFound)?;
        def.label = payload.label.trim().to_string();
        def.field_type = payload.field_type.trim().to_lowercase();
        def.enum_values = payload
            .enum_values
            .as_ref()
            .map(|v| serde_json::to_value(v).unwrap());
        def.sort_order = payload.sort_order.unwrap_or(0);
        Ok(())
    }

    fn count_requirement_versions_using_field(&self, field_id: i32) -> Result<i64, RepoError> {
        let count = self
            .custom_field_values
            .iter()
            .filter(|(_, fid, _)| *fid == field_id)
            .count();
        Ok(count as i64)
    }

    fn delete_custom_field_definition(&mut self, id: i32) -> Result<(), RepoError> {
        self.custom_field_definitions
            .remove(&id)
            .map(|_| ())
            .ok_or(RepoError::NotFound)
    }

    fn get_custom_field_values_for_version(
        &self,
        version_id: i32,
    ) -> Result<Vec<crate::models::CustomFieldValueDisplay>, RepoError> {
        let defs = &self.custom_field_definitions;
        let values: Vec<_> = self
            .custom_field_values
            .iter()
            .filter(|(vid, _, _)| *vid == version_id)
            .filter_map(|(_, fid, value)| {
                defs.get(fid)
                    .map(|d| crate::models::CustomFieldValueDisplay {
                        field_id: d.id,
                        label: d.label.clone(),
                        value: value.clone(),
                    })
            })
            .collect();
        Ok(values)
    }

    fn set_custom_field_values_for_version(
        &mut self,
        version_id: i32,
        values: &[(i32, Option<String>)],
    ) -> Result<(), RepoError> {
        self.custom_field_values
            .retain(|(vid, _, _)| *vid != version_id);
        for &(field_id, ref value) in values {
            if field_id > 0 {
                self.custom_field_values
                    .push((version_id, field_id, value.clone()));
            }
        }
        Ok(())
    }
}

impl crate::repository::BaselineRepository for DieselRepoMock {
    fn create_baseline(
        &mut self,
        project_id: i32,
        created_by: i32,
        payload: &crate::models::NewBaseline,
    ) -> Result<crate::models::Baseline, RepoError> {
        if self.force_err {
            return Err(RepoError::Db(diesel::result::Error::RollbackTransaction));
        }
        let id = self.next_baseline_id;
        self.next_baseline_id += 1;
        let baseline = crate::models::Baseline {
            id,
            project_id,
            name: payload.name.clone(),
            description: payload.description.clone(),
            created_at: epoch(),
            created_by,
        };
        self.baselines.push(baseline.clone());
        for req in self
            .requirements
            .values()
            .filter(|r| r.project_id == project_id)
        {
            if let Some(version_id) = req.current_version_id {
                // Include all current versions in baseline (point-in-time snapshot)
                self.baseline_requirements
                    .push(crate::models::BaselineRequirement {
                        baseline_id: id,
                        requirement_id: req.id,
                        version_id,
                    });
            }
        }
        for link in self.matrices.iter().filter(|m| m.project_id == project_id) {
            self.baseline_traceability
                .push(crate::models::BaselineTraceability {
                    baseline_id: id,
                    requirement_id: link.req_id,
                    test_id: link.test_id,
                    suspect: link.suspect,
                    suspect_at: link.suspect_at,
                    suspect_reason: link.suspect_reason.clone(),
                });
        }
        Ok(baseline)
    }

    fn list_baselines_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<crate::models::Baseline>, RepoError> {
        Ok(self
            .baselines
            .iter()
            .filter(|b| b.project_id == project_id)
            .cloned()
            .collect())
    }

    fn get_baseline_by_id(&self, baseline_id: i32) -> Result<crate::models::Baseline, RepoError> {
        self.baselines
            .iter()
            .find(|b| b.id == baseline_id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }

    fn get_requirements_for_baseline(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<Requirement>, RepoError> {
        let mut out = Vec::new();
        for br in self
            .baseline_requirements
            .iter()
            .filter(|br| br.baseline_id == baseline_id)
        {
            let req = self
                .requirements
                .get(&br.requirement_id)
                .ok_or(RepoError::NotFound)?;
            let version = self
                .requirement_versions
                .get(&br.version_id)
                .ok_or(RepoError::NotFound)?;
            out.push(Requirement {
                id: req.id,
                current_version_id: Some(version.id),
                same_as_current: None,
                title: version.title.clone(),
                description: version.description.clone(),
                status_id: version.status_id,
                author_id: version.author_id,
                reviewer_id: version.reviewer_id,
                reference_code: req.reference_code.clone(),
                category_id: version.category_id,
                parent_id: None, // populated from requirement_version_links by service/decorator layer
                creation_date: version.created_at,
                update_date: version.created_at,
                deadline_date: version.deadline_date,
                applicability_id: version.applicability_id,
                justification: version.justification.clone(),
                project_id: req.project_id,
                approval_state: version.approval_state.clone(),
                approved_by: version.approved_by,
                approved_at: version.approved_at,
                custom_fields: None,
            });
        }
        Ok(out)
    }

    fn get_baseline_requirement_version_id(
        &self,
        baseline_id: i32,
        requirement_id: i32,
    ) -> Result<Option<i32>, RepoError> {
        Ok(self
            .baseline_requirements
            .iter()
            .find(|br| br.baseline_id == baseline_id && br.requirement_id == requirement_id)
            .map(|br| br.version_id))
    }

    fn get_baseline_traceability(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<crate::models::BaselineTraceability>, RepoError> {
        Ok(self
            .baseline_traceability
            .iter()
            .filter(|bt| bt.baseline_id == baseline_id)
            .cloned()
            .collect())
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

impl RequirementCommentsRepository for DieselRepoMock {
    fn insert_requirement_comment(
        &mut self,
        new: &NewRequirementComment,
    ) -> Result<RequirementComment, RepoError> {
        if self.force_err {
            return Err(RepoError::Pool("force_err".into()));
        }
        let id = self.next_comment_id;
        self.next_comment_id += 1;
        let comment = RequirementComment {
            id,
            requirement_id: new.requirement_id,
            requirement_version_id: new.requirement_version_id,
            author_id: new.author_id,
            body: new.body.clone(),
            created_at: epoch(),
        };
        self.requirement_comments.push(comment.clone());
        Ok(comment)
    }

    fn list_comments_by_requirement(
        &self,
        requirement_id: i32,
        version_id: Option<i32>,
    ) -> Result<Vec<RequirementComment>, RepoError> {
        let mut out: Vec<RequirementComment> = self
            .requirement_comments
            .iter()
            .filter(|c| {
                c.requirement_id == requirement_id
                    && match version_id {
                        Some(vid) => {
                            c.requirement_version_id.is_none()
                                || c.requirement_version_id == Some(vid)
                        }
                        None => true,
                    }
            })
            .cloned()
            .collect();
        out.sort_by_key(|c| c.created_at);
        Ok(out)
    }
}

impl RequirementVersionLinksRepository for DieselRepoMock {
    fn list_links_by_source_version(
        &self,
        source_version_id: i32,
    ) -> Result<Vec<RequirementVersionLink>, RepoError> {
        let mut out: Vec<_> = self
            .requirement_version_links
            .iter()
            .filter(|l| l.source_version_id == source_version_id)
            .cloned()
            .collect();
        out.sort_by_key(|l| l.created_at);
        Ok(out)
    }

    fn list_links_by_target_version(
        &self,
        target_version_id: i32,
    ) -> Result<Vec<RequirementVersionLink>, RepoError> {
        let mut out: Vec<_> = self
            .requirement_version_links
            .iter()
            .filter(|l| l.target_version_id == target_version_id)
            .cloned()
            .collect();
        out.sort_by_key(|l| l.created_at);
        Ok(out)
    }

    fn list_links_by_project(
        &self,
        project_id: i32,
        source_version_id: Option<i32>,
        target_version_id: Option<i32>,
        link_type: Option<&str>,
    ) -> Result<Vec<RequirementVersionLink>, RepoError> {
        let mut out: Vec<_> = self
            .requirement_version_links
            .iter()
            .filter(|l| {
                l.project_id == project_id
                    && source_version_id.is_none_or(|s| l.source_version_id == s)
                    && target_version_id.is_none_or(|t| l.target_version_id == t)
                    && link_type.is_none_or(|lt| l.link_type.as_str() == lt)
            })
            .cloned()
            .collect();
        out.sort_by_key(|l| l.created_at);
        Ok(out)
    }

    fn insert_requirement_version_link(
        &mut self,
        new: &NewRequirementVersionLink,
    ) -> Result<RequirementVersionLink, RepoError> {
        if self.force_err {
            return Err(RepoError::Pool("force_err".into()));
        }
        let id = self.next_link_id;
        self.next_link_id += 1;
        let link = RequirementVersionLink {
            id,
            source_version_id: new.source_version_id,
            target_version_id: new.target_version_id,
            link_type: new.link_type.clone(),
            rationale: new.rationale.clone(),
            project_id: new.project_id,
            created_at: epoch(),
            metadata: new.metadata.clone(),
        };
        self.requirement_version_links.push(link.clone());
        Ok(link)
    }

    fn delete_requirement_version_link(
        &mut self,
        link_id: i32,
    ) -> Result<RequirementVersionLink, RepoError> {
        if self.force_err {
            return Err(RepoError::Pool("force_err".into()));
        }
        let pos = self
            .requirement_version_links
            .iter()
            .position(|l| l.id == link_id)
            .ok_or(RepoError::NotFound)?;
        Ok(self.requirement_version_links.remove(pos))
    }

    fn delete_requirement_version_links_by_source_version(
        &mut self,
        source_version_id: i32,
    ) -> Result<Vec<RequirementVersionLink>, RepoError> {
        if self.force_err {
            return Err(RepoError::Pool("force_err".into()));
        }
        let (kept, removed): (Vec<_>, Vec<_>) = self
            .requirement_version_links
            .drain(..)
            .partition(|l| l.source_version_id != source_version_id);
        self.requirement_version_links = kept;
        Ok(removed)
    }

    fn get_requirement_version_link_by_id(
        &self,
        link_id: i32,
    ) -> Result<RequirementVersionLink, RepoError> {
        self.requirement_version_links
            .iter()
            .find(|l| l.id == link_id)
            .cloned()
            .ok_or(RepoError::NotFound)
    }
}

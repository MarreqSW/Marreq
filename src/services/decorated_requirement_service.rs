//! Service providing requirement related operations.
//!
//! The service is intentionally lightweight and wraps repository calls with
//! validation and logging so that route handlers can remain focused on HTTP
//! concerns.

use super::{
    ApplicabilityService, CategoryService, RequirementService, StatusService, UserService,
    VerificationService,
};
use crate::app::{AppState, DieselCachedRepo};
use crate::models::{DecoratedRequirement, NewRequirement, Requirement, TestCase, User};
use crate::repository::errors::RepoError;

/// High level operations for requirements backed by the shared [`AppState`].
pub struct DecoratedRequirementService<'a> {
    requirement_service: RequirementService<'a>,
    verification_service: VerificationService<'a>,
    category_service: CategoryService<'a>,
    status_service: StatusService<'a>,
    user_service: UserService<'a>,
    applicability_service: ApplicabilityService<'a>,
}

impl<'a> DecoratedRequirementService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self {
            requirement_service: RequirementService::new(state),
            verification_service: VerificationService::new(state),
            category_service: CategoryService::new(state),
            status_service: StatusService::new(state),
            user_service: UserService::new(state),
            applicability_service: ApplicabilityService::new(state),
        }
    }

    /// Retrieve all requirements.
    pub fn list_all(&self) -> Result<Vec<DecoratedRequirement>, RepoError> {
        self.decorate_vec(self.requirement_service.list_all()?)
    }

    /// Retrieve requirements scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<DecoratedRequirement>, RepoError> {
        self.decorate_vec(self.requirement_service.list_by_project(project_id)?)
    }

    pub fn list_by_project_filtered(
        &self,
        project_id: i32,
        status_filter: Option<i32>,
        verification_filter: Option<i32>,
        category_filter: Option<i32>,
        applicability_filter: Option<i32>,
    ) -> Result<Vec<DecoratedRequirement>, RepoError> {
        self.decorate_vec(self.requirement_service.list_by_project_filtered(
            project_id,
            status_filter,
            verification_filter,
            category_filter,
            applicability_filter,
        )?)
    }

    /// Retrieve a single requirement by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<DecoratedRequirement, RepoError> {
        let req = self.requirement_service.get_by_id(id)?;
        self.decorate(&req)
    }

    pub fn get_by_parent_id(&self, parent_id: i32) -> Result<Vec<DecoratedRequirement>, RepoError> {
        self.decorate_vec(self.requirement_service.get_by_parent_id(parent_id)?)
    }

    pub fn get_linked_tests(&self, id: i32) -> Result<Vec<TestCase>, RepoError> {
        self.requirement_service.get_linked_tests(id)
    }

    /// Create a new requirement entry and log the action.
    pub fn create(&self, actor: &User, payload: NewRequirement) -> Result<i32, RepoError> {
        self.requirement_service.create(actor, payload)
    }

    /// Update an existing requirement entry and log the change.
    pub fn update(
        &self,
        actor: &User,
        id: i32,
        payload: NewRequirement,
    ) -> Result<Requirement, RepoError> {
        self.requirement_service.update(actor, id, payload)
    }

    /// Delete an requirement entry and log the removal.
    pub fn delete(&self, actor: &User, id: i32) -> Result<Requirement, RepoError> {
        self.requirement_service.delete(actor, id)
    }

    fn decorate_vec(&self, req: Vec<Requirement>) -> Result<Vec<DecoratedRequirement>, RepoError> {
        req.iter().map(|r| self.decorate(r)).collect()
    }

    fn decorate(&self, req: &Requirement) -> Result<DecoratedRequirement, RepoError> {
        let verification = self
            .verification_service
            .get_by_id(req.verification_method_id)
            .map(|v| v.verification_name)
            .unwrap_or_else(|_| format!("Unknown Verification ({})", req.verification_method_id));

        let status = self
            .status_service
            .get_requirement_status(req.current_status_id)
            .map(|s| s.req_st_title)
            .unwrap_or_else(|_| format!("Unknown Status ({})", req.current_status_id));

        let author = self
            .user_service
            .get_by_id(req.author_id)
            .map(|u| u.user_name)
            .unwrap_or_else(|_| format!("Unknown User ({})", req.author_id));

        let reviewer = self
            .user_service
            .get_by_id(req.reviewer_id)
            .map(|u| u.user_name)
            .unwrap_or_else(|_| format!("Unknown User ({})", req.reviewer_id));

        let category = self
            .category_service
            .get_by_id(req.category_id)
            .map(|c| c.cat_title)
            .unwrap_or_else(|_| format!("Unknown Category ({})", req.category_id));

        let applicability = self
            .applicability_service
            .get_by_id(req.applicability_id)
            .map(|a| a.app_title)
            .unwrap_or_else(|_| format!("Unknown Applicability ({})", req.applicability_id));

        let parent_title = if req.parent_id != 0 {
            match self.requirement_service.get_by_id(req.parent_id) {
                Ok(parent_req) => parent_req.title,
                Err(_) => "[Deleted Parent]".to_string(),
            }
        } else {
            String::new()
        };

        Ok(DecoratedRequirement {
            id: req.id,
            title: req.title.clone(),
            verification_method_id: verification,
            req_verification_id: req.verification_method_id,
            description: req.description.clone(),
            current_status_id: status,
            req_current_status_id: req.current_status_id,
            author_id: author,
            req_author_id: req.author_id,
            reviewer_id: reviewer,
            req_reviewer_id: req.reviewer_id,
            reference_code: req.reference_code.clone(),
            category_id: category,
            req_category_id: req.category_id,
            applicability_id: applicability,
            req_applicability_id: req.applicability_id,
            req_parent_id: req.parent_id,
            req_parent_title: parent_title,
            creation_date: req
                .creation_date
                .format("%d-%m-%Y %H:%M:%S")
                .to_string(),
            update_date: req.update_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            deadline_date: req
                .deadline_date
                .format("%d-%m-%Y %H:%M:%S")
                .to_string(),
            justification: req.justification.clone(),
            project_id: req.project_id,
        })
    }
}

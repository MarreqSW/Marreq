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
use crate::models::{DecoratedRequirement, NewRequirement, Requirement, Test, User};
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
    ) -> Result<Vec<DecoratedRequirement>, RepoError> {
        self.decorate_vec(self.requirement_service.list_by_project_filtered(
            project_id,
            status_filter,
            verification_filter,
            category_filter,
        )?)
    }

    /// Retrieve a single requirement by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<DecoratedRequirement, RepoError> {
        let req = self.requirement_service.get_by_id(id)?;
        self.decorate(&req)
    }

    pub fn get_linked_tests(&self, id: i32) -> Result<Vec<Test>, RepoError> {
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
            .get_by_id(req.req_verification)
            .map(|v| v.verification_name)
            .unwrap_or_else(|_| format!("Unknown Verification ({})", req.req_verification));

        let status = self
            .status_service
            .get_requirement_status(req.req_current_status)
            .map(|s| s.req_st_title)
            .unwrap_or_else(|_| format!("Unknown Status ({})", req.req_current_status));

        let author = self
            .user_service
            .get_by_id(req.req_author)
            .map(|u| u.user_name)
            .unwrap_or_else(|_| format!("Unknown User ({})", req.req_author));

        let reviewer = self
            .user_service
            .get_by_id(req.req_reviewer)
            .map(|u| u.user_name)
            .unwrap_or_else(|_| format!("Unknown User ({})", req.req_reviewer));

        let category = self
            .category_service
            .get_by_id(req.req_category)
            .map(|c| c.cat_title)
            .unwrap_or_else(|_| format!("Unknown Category ({})", req.req_category));

        let applicability = self
            .applicability_service
            .get_by_id(req.req_applicability)
            .map(|a| a.app_title)
            .unwrap_or_else(|_| format!("Unknown Applicability ({})", req.req_applicability));

        let parent_title = if req.req_parent != 0 {
            match self.requirement_service.get_by_id(req.req_parent) {
                Ok(parent_req) => parent_req.req_title,
                Err(_) => "[Deleted Parent]".to_string(),
            }
        } else {
            String::new()
        };

        Ok(DecoratedRequirement {
            req_id: req.req_id,
            req_title: req.req_title.clone(),
            req_verification: verification,
            req_verification_id: req.req_verification,
            req_description: req.req_description.clone(),
            req_current_status: status,
            req_current_status_id: req.req_current_status,
            req_author: author,
            req_author_id: req.req_author,
            req_reviewer: reviewer,
            req_reviewer_id: req.req_reviewer,
            req_link: req.req_link.clone(),
            req_reference: req.req_reference.clone(),
            req_category: category,
            req_category_id: req.req_category,
            req_applicability: applicability,
            req_applicability_id: req.req_applicability,
            req_parent_id: req.req_parent,
            req_parent_title: parent_title,
            req_creation_date: req
                .req_creation_date
                .format("%d-%m-%Y %H:%M:%S")
                .to_string(),
            req_update_date: req.req_update_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            req_deadline_date: req
                .req_deadline_date
                .format("%d-%m-%Y %H:%M:%S")
                .to_string(),
            req_justification: req.req_justification.clone(),
            project_id: req.project_id,
        })
    }
}

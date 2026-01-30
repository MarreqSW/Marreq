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
            .map(|v| v.title)
            .unwrap_or_else(|_| format!("Unknown Verification ({})", req.verification_method_id));

        let status = self
            .status_service
            .get_requirement_status(req.status_id)
            .map(|s| s.title)
            .unwrap_or_else(|_| format!("Unknown Status ({})", req.status_id));

        let author = self
            .user_service
            .get_by_id(req.author_id)
            .map(|u| u.name)
            .unwrap_or_else(|_| format!("Unknown User ({})", req.author_id));

        let reviewer = self
            .user_service
            .get_by_id(req.reviewer_id)
            .map(|u| u.name)
            .unwrap_or_else(|_| format!("Unknown User ({})", req.reviewer_id));

        let category = self
            .category_service
            .get_by_id(req.category_id)
            .map(|c| c.title)
            .unwrap_or_else(|_| format!("Unknown Category ({})", req.category_id));

        let applicability = self
            .applicability_service
            .get_by_id(req.applicability_id)
            .map(|a| a.title)
            .unwrap_or_else(|_| format!("Unknown Applicability ({})", req.applicability_id));

        let parent_title = if let Some(parent_id) = req.parent_id {
            match self.requirement_service.get_by_id(parent_id) {
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
            status_id: status,
            req_current_status_id: req.status_id,
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
            creation_date: req.creation_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            update_date: req.update_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            deadline_date: req
                .deadline_date
                .map(|d| d.format("%d-%m-%Y %H:%M:%S").to_string())
                .unwrap_or_default(),
            justification: req.justification.clone(),
            project_id: req.project_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Applicability, Category, RequirementStatus, User, VerificationMethod};
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::sync::{Arc, RwLock};

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn state_with_repo(repo: DieselRepoMock) -> AppState<DieselCachedRepo> {
        AppState {
            repo: Arc::new(RwLock::new(DieselCachedRepo::new(repo, 0))),
        }
    }

    fn requirement(id: i32, project_id: i32) -> Requirement {
        Requirement {
            id,
            title: format!("Requirement {id}"),
            description: "Description".into(),
            verification_method_id: 1,
            status_id: 1,
            author_id: 1,
            reviewer_id: 2,
            reference_code: format!("REQ-{id:03}"),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            applicability_id: 1,
            justification: Some("Justification".into()),
            project_id,
        }
    }

    fn setup_repo_with_lookup_data() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        // Add verification method
        repo.verifications.insert(
            1,
            VerificationMethod {
                id: 1,
                title: "Test Verification".into(),
                description: "Test".into(),
                tag: "TEST".into(),
                project_id: 1,
            },
        );

        // Add status
        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                id: 1,
                title: "Draft".into(),
                description: "Draft status".into(),
                tag: "DRAFT".into(),
                project_id: 1,
            },
        );

        // Add users
        repo.users.insert(
            1,
            User {
                id: 1,
                username: "author".into(),
                name: "Author Name".into(),
                email: "author@example.com".into(),
                creation_date: timestamp(),
                last_login: timestamp(),
                password_hash: "hash".into(),
                is_admin: false,
            },
        );
        repo.users.insert(
            2,
            User {
                id: 2,
                username: "reviewer".into(),
                name: "Reviewer Name".into(),
                email: "reviewer@example.com".into(),
                creation_date: timestamp(),
                last_login: timestamp(),
                password_hash: "hash".into(),
                is_admin: false,
            },
        );

        // Add category
        repo.categories.insert(
            1,
            Category {
                id: 1,
                title: "Functional".into(),
                description: "Functional requirements".into(),
                tag: "FUNC".into(),
                project_id: 1,
            },
        );

        // Add applicability
        repo.applicability.insert(
            1,
            Applicability {
                id: 1,
                title: "All Systems".into(),
                description: "Applies to all systems".into(),
                tag: "ALL".into(),
                project_id: 1,
            },
        );

        repo
    }

    #[test]
    fn decorate_includes_all_related_data() {
        let mut repo = setup_repo_with_lookup_data();
        repo.requirements.insert(1, requirement(1, 1));

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.get_by_id(1).unwrap();

        assert_eq!(decorated.id, 1);
        assert_eq!(decorated.title, "Requirement 1");
        assert_eq!(decorated.verification_method_id, "Test Verification");
        assert_eq!(decorated.status_id, "Draft");
        assert_eq!(decorated.author_id, "Author Name");
        assert_eq!(decorated.reviewer_id, "Reviewer Name");
        assert_eq!(decorated.category_id, "Functional");
        assert_eq!(decorated.applicability_id, "All Systems");
    }

    #[test]
    fn decorate_handles_missing_verification() {
        let mut repo = setup_repo_with_lookup_data();
        repo.verifications.remove(&1);
        repo.requirements.insert(1, requirement(1, 1));

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.get_by_id(1).unwrap();
        assert_eq!(decorated.verification_method_id, "Unknown Verification (1)");
    }

    #[test]
    fn decorate_handles_missing_status() {
        let mut repo = setup_repo_with_lookup_data();
        repo.requirement_statuses.remove(&1);
        repo.requirements.insert(1, requirement(1, 1));

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.get_by_id(1).unwrap();
        assert_eq!(decorated.status_id, "Unknown Status (1)");
    }

    #[test]
    fn decorate_handles_missing_author() {
        let mut repo = setup_repo_with_lookup_data();
        repo.users.remove(&1);
        repo.requirements.insert(1, requirement(1, 1));

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.get_by_id(1).unwrap();
        assert_eq!(decorated.author_id, "Unknown User (1)");
    }

    #[test]
    fn decorate_handles_missing_reviewer() {
        let mut repo = setup_repo_with_lookup_data();
        repo.users.remove(&2);
        repo.requirements.insert(1, requirement(1, 1));

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.get_by_id(1).unwrap();
        assert_eq!(decorated.reviewer_id, "Unknown User (2)");
    }

    #[test]
    fn decorate_handles_missing_category() {
        let mut repo = setup_repo_with_lookup_data();
        repo.categories.remove(&1);
        repo.requirements.insert(1, requirement(1, 1));

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.get_by_id(1).unwrap();
        assert_eq!(decorated.category_id, "Unknown Category (1)");
    }

    #[test]
    fn decorate_handles_missing_applicability() {
        let mut repo = setup_repo_with_lookup_data();
        repo.applicability.remove(&1);
        repo.requirements.insert(1, requirement(1, 1));

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.get_by_id(1).unwrap();
        assert_eq!(decorated.applicability_id, "Unknown Applicability (1)");
    }

    #[test]
    fn decorate_includes_parent_title_when_parent_exists() {
        let mut repo = setup_repo_with_lookup_data();
        let mut parent = requirement(1, 1);
        parent.title = "Parent Requirement".into();
        let mut child = requirement(2, 1);
        child.parent_id = Some(1);

        repo.requirements.insert(1, parent);
        repo.requirements.insert(2, child);

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.get_by_id(2).unwrap();
        assert_eq!(decorated.req_parent_id, Some(1));
        assert_eq!(decorated.req_parent_title, "Parent Requirement");
    }

    #[test]
    fn decorate_handles_deleted_parent() {
        let mut repo = setup_repo_with_lookup_data();
        let mut child = requirement(2, 1);
        child.parent_id = Some(999); // Parent doesn't exist

        repo.requirements.insert(2, child);

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.get_by_id(2).unwrap();
        assert_eq!(decorated.req_parent_id, Some(999));
        assert_eq!(decorated.req_parent_title, "[Deleted Parent]");
    }

    #[test]
    fn decorate_handles_no_parent() {
        let mut repo = setup_repo_with_lookup_data();
        let mut req = requirement(1, 1);
        req.parent_id = None;

        repo.requirements.insert(1, req);

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.get_by_id(1).unwrap();
        assert_eq!(decorated.req_parent_id, None);
        assert_eq!(decorated.req_parent_title, "");
    }

    #[test]
    fn decorate_formats_dates_correctly() {
        let mut repo = setup_repo_with_lookup_data();
        repo.requirements.insert(1, requirement(1, 1));

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.get_by_id(1).unwrap();
        // Date format should be "dd-mm-yyyy HH:MM:SS"
        assert!(decorated.creation_date.contains("2024"));
        assert!(decorated.update_date.contains("2024"));
        assert!(decorated.deadline_date.contains("2024"));
    }

    #[test]
    fn decorate_handles_none_deadline() {
        let mut repo = setup_repo_with_lookup_data();
        let mut req = requirement(1, 1);
        req.deadline_date = None;

        repo.requirements.insert(1, req);

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.get_by_id(1).unwrap();
        assert_eq!(decorated.deadline_date, "");
    }

    #[test]
    fn list_all_decorates_all_requirements() {
        let mut repo = setup_repo_with_lookup_data();
        repo.requirements.insert(1, requirement(1, 1));
        repo.requirements.insert(2, requirement(2, 1));

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.list_all().unwrap();
        assert_eq!(decorated.len(), 2);
        // Order may vary, so check that both IDs are present
        let ids: Vec<i32> = decorated.iter().map(|r| r.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
    }

    #[test]
    fn list_by_project_decorates_filtered_requirements() {
        let mut repo = setup_repo_with_lookup_data();
        repo.requirements.insert(1, requirement(1, 1));
        repo.requirements.insert(2, requirement(2, 2)); // Different project

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.list_by_project(1).unwrap();
        assert_eq!(decorated.len(), 1);
        assert_eq!(decorated[0].project_id, 1);
    }

    #[test]
    fn list_by_project_filtered_decorates_filtered_results() {
        let mut repo = setup_repo_with_lookup_data();
        let mut req1 = requirement(1, 1);
        req1.status_id = 1;
        let mut req2 = requirement(2, 1);
        req2.status_id = 2;

        repo.requirements.insert(1, req1);
        repo.requirements.insert(2, req2);

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service
            .list_by_project_filtered(1, Some(1), None, None, None)
            .unwrap();
        assert_eq!(decorated.len(), 1);
        assert_eq!(decorated[0].id, 1);
    }

    #[test]
    fn get_by_parent_id_decorates_children() {
        let mut repo = setup_repo_with_lookup_data();
        let parent = requirement(1, 1);
        let mut child1 = requirement(2, 1);
        child1.parent_id = Some(1);
        let mut child2 = requirement(3, 1);
        child2.parent_id = Some(1);

        repo.requirements.insert(1, parent);
        repo.requirements.insert(2, child1);
        repo.requirements.insert(3, child2);

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let decorated = service.get_by_parent_id(1).unwrap();
        assert_eq!(decorated.len(), 2);
        assert!(decorated.iter().any(|r| r.id == 2));
        assert!(decorated.iter().any(|r| r.id == 3));
    }

    #[test]
    fn create_delegates_to_requirement_service() {
        let repo = setup_repo_with_lookup_data();
        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let actor = DieselRepoMock::make_user(1, "actor", "");
        let payload = NewRequirement {
            id: None,
            title: "New Requirement".into(),
            description: "Description".into(),
            verification_method_id: 1,
            author_id: 1,
            category_id: 1,
            status_id: 1,
            parent_id: None,
            reference_code: "REQ-NEW".into(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: None,
            project_id: 1,
        };

        let id = service.create(&actor, payload).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn update_delegates_to_requirement_service() {
        let mut repo = setup_repo_with_lookup_data();
        repo.requirements.insert(1, requirement(1, 1));

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let actor = DieselRepoMock::make_user(1, "actor", "");
        let payload = NewRequirement {
            id: Some(1),
            title: "Updated".into(),
            description: "Updated Description".into(),
            verification_method_id: 1,
            author_id: 1,
            category_id: 1,
            status_id: 1,
            parent_id: None,
            reference_code: "REQ-001".into(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: None,
            project_id: 1,
        };

        let updated = service.update(&actor, 1, payload).unwrap();
        assert_eq!(updated.title, "Updated");
    }

    #[test]
    fn delete_delegates_to_requirement_service() {
        let mut repo = setup_repo_with_lookup_data();
        repo.requirements.insert(1, requirement(1, 1));

        let state = state_with_repo(repo);
        let service = DecoratedRequirementService::new(&state);

        let actor = DieselRepoMock::make_user(1, "actor", "");
        let deleted = service.delete(&actor, 1).unwrap();
        assert_eq!(deleted.id, 1);
    }
}

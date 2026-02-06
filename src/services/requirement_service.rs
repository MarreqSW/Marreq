//! Service providing requirement related operations.
//!
//! The service is intentionally lightweight and wraps repository calls with
//! validation and logging so that route handlers can remain focused on HTTP
//! concerns.
//!
//! When semantic search is enabled, requirements are automatically queued for
//! embedding generation on create/update operations.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Loggable, Logger};
use crate::models::{EntityType, NewRequirement, Requirement, TestCase, User};
use crate::repository::errors::RepoError;
use crate::repository::{PooledConnectionWrapper, RequirementsRepository, TestsCaseRepository};
use crate::services::semantic_search::{IndexingService, SemanticSearchConfig};
use crate::validation::{sanitize_optional_string, sanitize_string, validate_requirement};
use serde::Serialize;

/// Wrapper used when logging requirement updates so that verification method IDs
/// are included in old_values/new_values (they live in a separate junction table).
#[derive(Serialize)]
struct RequirementWithVerification<'a> {
    #[serde(flatten)]
    requirement: &'a Requirement,
    verification_method_ids: Vec<i32>,
}

impl Loggable for RequirementWithVerification<'_> {
    fn entity_type() -> EntityType {
        EntityType::Requirement
    }
    fn id(&self) -> i32 {
        self.requirement.id
    }
    fn project_id(&self) -> Option<i32> {
        Some(self.requirement.project_id)
    }
    fn display_name(&self) -> String {
        self.requirement.reference_code.clone()
    }
}

/// High level operations for requirements backed by the shared [`AppState`].
pub struct RequirementService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> RequirementService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve all requirements.
    pub fn list_all(&self) -> Result<Vec<Requirement>, RepoError> {
        self.repo_read().get_requirements_all()
    }

    /// Retrieve requirements scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<Requirement>, RepoError> {
        self.repo_read().get_requirements_by_project(project_id)
    }

    pub fn list_by_project_filtered(
        &self,
        project_id: i32,
        status_filter: Option<i32>,
        verification_filter: Option<i32>,
        category_filter: Option<i32>,
        applicability_filter: Option<i32>,
    ) -> Result<Vec<Requirement>, RepoError> {
        let reqs = self.list_by_project(project_id)?;
        let verification_requirement_ids = match verification_filter {
            Some(vid) => self
                .repo_read()
                .get_requirement_ids_by_verification_method(vid)
                .ok()
                .unwrap_or_default(),
            None => vec![],
        };
        Ok(Self::filter(
            reqs,
            status_filter,
            verification_filter,
            &verification_requirement_ids,
            category_filter,
            applicability_filter,
        ))
    }

    /// Paginated filtered list; loads only one page from the database.
    #[allow(clippy::too_many_arguments)]
    pub fn list_by_project_filtered_paginated(
        &self,
        project_id: i32,
        status_filter: Option<i32>,
        verification_filter: Option<i32>,
        category_filter: Option<i32>,
        applicability_filter: Option<i32>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Requirement>, RepoError> {
        self.repo_read()
            .get_requirements_by_project_filtered_paginated(
                project_id,
                status_filter,
                verification_filter,
                category_filter,
                applicability_filter,
                limit,
                offset,
            )
    }

    /// Retrieve a single requirement by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<Requirement, RepoError> {
        self.repo_read().get_requirement_by_id(id)
    }

    /// Verification method IDs linked to this requirement.
    pub fn get_verification_method_ids(&self, requirement_id: i32) -> Result<Vec<i32>, RepoError> {
        self.repo_read()
            .get_verification_method_ids_for_requirement(requirement_id)
    }

    pub fn get_by_parent_id(&self, parent_id: i32) -> Result<Vec<Requirement>, RepoError> {
        Ok(self
            .list_all()?
            .into_iter()
            .filter(|r| r.parent_id == Some(parent_id))
            .collect())
    }

    pub fn get_linked_tests(&self, id: i32) -> Result<Vec<TestCase>, RepoError> {
        self.repo_read().get_tests_for_requirement(id)
    }

    /// Create a new requirement entry and log the action.
    ///
    /// If semantic search is enabled, the requirement is queued for embedding generation.
    /// `verification_method_ids` are written to the requirement–verification junction table.
    pub fn create(
        &self,
        actor: &User,
        mut payload: NewRequirement,
        verification_method_ids: &[i32],
    ) -> Result<i32, RepoError> {
        self.prepare_payload(&mut payload)?;

        let project_id = payload.project_id;
        let id = {
            let mut repo = self.repo_write();
            let id = repo.insert_new_requirement(&payload)?;
            repo.set_requirement_verification_methods(id, verification_method_ids)?;
            id
        };

        self.log_created(actor, id, &payload);
        self.queue_for_indexing(id, project_id);
        Ok(id)
    }

    /// Update an existing requirement entry and log the change.
    ///
    /// If semantic search is enabled, the requirement is queued for re-embedding.
    /// `verification_method_ids` replace the requirement's verification methods.
    pub fn update(
        &self,
        actor: &User,
        id: i32,
        mut payload: NewRequirement,
        verification_method_ids: &[i32],
    ) -> Result<Requirement, RepoError> {
        self.prepare_payload(&mut payload)?;
        payload.id = Some(id);

        let before = self.get_by_id(id)?;
        let before_verification_ids = self.get_verification_method_ids(id)?;

        {
            let mut repo = self.repo_write();
            let updated = repo.edit_requirement(&payload)?;
            if !updated {
                return Err(RepoError::NotFound);
            }
            repo.set_requirement_verification_methods(id, verification_method_ids)?;
        }

        let after = self.get_by_id(id)?;
        let after_verification_ids = verification_method_ids.to_vec();
        self.log_updated(
            actor,
            &RequirementWithVerification {
                requirement: &before,
                verification_method_ids: before_verification_ids,
            },
            &RequirementWithVerification {
                requirement: &after,
                verification_method_ids: after_verification_ids,
            },
        );
        self.queue_for_indexing(id, after.project_id);
        Ok(after)
    }

    /// Delete an requirement entry and log the removal.
    pub fn delete(&self, actor: &User, id: i32) -> Result<Requirement, RepoError> {
        let removed = {
            let mut repo = self.repo_write();
            repo.delete_requirement(id)?
        };

        self.log_deleted(actor, &removed);
        Ok(removed)
    }

    fn filter(
        requirements: Vec<Requirement>,
        status_filter: Option<i32>,
        verification_filter: Option<i32>,
        verification_requirement_ids: &[i32],
        category_filter: Option<i32>,
        applicability_filter: Option<i32>,
    ) -> Vec<Requirement> {
        let mut filtered_requirements: Vec<Requirement> = requirements
            .into_iter()
            .filter(|req| {
                let status_match = status_filter.is_none_or(|status_id| req.status_id == status_id);
                let verification_match = verification_filter
                    .is_none_or(|_| verification_requirement_ids.contains(&req.id));
                let category_match =
                    category_filter.is_none_or(|category_id| req.category_id == category_id);
                let applicability_match = applicability_filter
                    .is_none_or(|applicability_id| req.applicability_id == applicability_id);
                status_match && verification_match && category_match && applicability_match
            })
            .collect();

        filtered_requirements.sort_by(|a, b| {
            match (a.reference_code.is_empty(), b.reference_code.is_empty()) {
                (false, false) => a.reference_code.cmp(&b.reference_code),
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                (true, true) => a.id.cmp(&b.id),
            }
        });

        filtered_requirements
    }

    fn repo_read(&self) -> std::sync::RwLockReadGuard<'_, DieselCachedRepo> {
        self.state.repo.read().expect("repo lock poisoned")
    }

    fn repo_write(&self) -> std::sync::RwLockWriteGuard<'_, DieselCachedRepo> {
        self.state.repo.write().expect("repo lock poisoned")
    }

    fn prepare_payload(&self, payload: &mut NewRequirement) -> Result<(), RepoError> {
        sanitize_string(&mut payload.title);
        sanitize_string(&mut payload.description);
        sanitize_string(&mut payload.reference_code);
        sanitize_optional_string(&mut payload.justification);

        validate_requirement(payload).map_err(|err| RepoError::BadInput(err.to_string()))
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    /// Queue a requirement for semantic search indexing if enabled.
    ///
    /// This is a best-effort operation; failures are logged but don't affect
    /// the main CRUD operation.
    fn queue_for_indexing(&self, requirement_id: i32, project_id: i32) {
        let config = SemanticSearchConfig::global();
        if !config.embeddings_enabled {
            return;
        }

        let indexing_service = IndexingService::new(self.state);
        if let Err(_e) = indexing_service.queue_for_indexing(requirement_id, project_id) {
            #[cfg(debug_assertions)]
            eprintln!(
                "Failed to queue requirement {} for indexing: {}",
                requirement_id, _e
            );
        }
    }

    fn log_created(&self, actor: &User, id: i32, entity: &NewRequirement) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log requirement creation {id}: {_err}");
            }
        }
    }

    fn log_updated<T: serde::Serialize + Loggable>(&self, actor: &User, before: &T, after: &T) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::updated(conn.as_mut(), &ctx, before, after) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log requirement update: {_err}");
            }
        }
    }

    fn log_deleted(&self, actor: &User, entity: &Requirement) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log requirement deletion {}: {_err}", entity.id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MatrixLink;
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

    fn actor() -> User {
        DieselRepoMock::make_user(1, "actor", "")
    }

    fn requirement(id: i32, project_id: i32, reference: &str) -> Requirement {
        Requirement {
            id,
            title: format!("Requirement {id}"),
            description: "Existing description".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: reference.into(),
            category_id: 1,
            parent_id: Some(1),
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            applicability_id: 1,
            justification: Some("because".into()),
            project_id,
        }
    }

    fn new_payload() -> NewRequirement {
        NewRequirement {
            id: None,
            title: "  Title  ".into(),
            description: "  Description  ".into(),
            author_id: 1,
            category_id: 1,
            status_id: 1,
            parent_id: None,
            reference_code: "  REQ-123  ".into(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: Some("   ".into()),
            project_id: 7,
        }
    }

    #[test]
    fn create_sanitizes_payload() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let payload = new_payload();
        let id = service.create(&actor(), payload, &[1]).unwrap();

        let stored = service.get_by_id(id).unwrap();
        assert_eq!(stored.title, "Title");
        assert_eq!(stored.description, "Description");
        assert_eq!(stored.reference_code, "REQ-123");
        assert!(stored.justification.is_none());
    }

    #[test]
    fn create_rejects_invalid_reference() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let mut payload = new_payload();
        payload.reference_code = "invalid".into();

        let err = service.create(&actor(), payload, &[1]).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }

    #[test]
    fn update_modifies_existing_requirement() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(1, requirement(1, 7, "REQ-001"));
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let mut payload = new_payload();
        payload.title = "  Updated  ".into();
        payload.description = "  New Description  ".into();
        payload.reference_code = "  REQ-999  ".into();

        let updated = service.update(&actor(), 1, payload, &[1]).unwrap();
        assert_eq!(updated.title, "Updated");
        assert_eq!(updated.description, "New Description");
        assert_eq!(updated.reference_code, "REQ-999");
    }

    #[test]
    fn delete_removes_requirement() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(2, requirement(2, 7, "REQ-002"));
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let removed = service.delete(&actor(), 2).unwrap();
        assert_eq!(removed.id, 2);
        assert!(matches!(service.get_by_id(2), Err(RepoError::NotFound)));
    }

    #[test]
    fn list_by_project_filters_requirements() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(1, requirement(1, 7, "REQ-001"));
        repo.requirements.insert(2, requirement(2, 99, "REQ-002"));
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let items = service.list_by_project(7).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].project_id, 7);
    }

    #[test]
    fn list_by_project_filtered_with_status_filter() {
        let mut repo = DieselRepoMock::default();
        let mut req1 = requirement(1, 7, "REQ-001");
        req1.status_id = 1;
        let mut req2 = requirement(2, 7, "REQ-002");
        req2.status_id = 2;
        repo.requirements.insert(1, req1);
        repo.requirements.insert(2, req2);
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let items = service
            .list_by_project_filtered(7, Some(1), None, None, None)
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].status_id, 1);
    }

    #[test]
    fn list_by_project_filtered_with_verification_filter() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(1, requirement(1, 7, "REQ-001"));
        repo.requirements.insert(2, requirement(2, 7, "REQ-002"));
        repo.requirement_verification_methods.push((1, 10));
        repo.requirement_verification_methods.push((2, 20));
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let items = service
            .list_by_project_filtered(7, None, Some(10), None, None)
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, 1);
    }

    #[test]
    fn list_by_project_filtered_with_category_filter() {
        let mut repo = DieselRepoMock::default();
        let mut req1 = requirement(1, 7, "REQ-001");
        req1.category_id = 100;
        let mut req2 = requirement(2, 7, "REQ-002");
        req2.category_id = 200;
        repo.requirements.insert(1, req1);
        repo.requirements.insert(2, req2);
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let items = service
            .list_by_project_filtered(7, None, None, Some(100), None)
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].category_id, 100);
    }

    #[test]
    fn list_by_project_filtered_with_applicability_filter() {
        let mut repo = DieselRepoMock::default();
        let mut req1 = requirement(1, 7, "REQ-001");
        req1.applicability_id = 5;
        let mut req2 = requirement(2, 7, "REQ-002");
        req2.applicability_id = 6;
        repo.requirements.insert(1, req1);
        repo.requirements.insert(2, req2);
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let items = service
            .list_by_project_filtered(7, None, None, None, Some(5))
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].applicability_id, 5);
    }

    #[test]
    fn list_by_project_filtered_with_multiple_filters() {
        let mut repo = DieselRepoMock::default();
        let mut req1 = requirement(1, 7, "REQ-001");
        req1.status_id = 1;
        req1.category_id = 100;
        let mut req2 = requirement(2, 7, "REQ-002");
        req2.status_id = 1;
        req2.category_id = 100;
        let mut req3 = requirement(3, 7, "REQ-003");
        req3.status_id = 2; // Different status
        req3.category_id = 100;
        repo.requirements.insert(1, req1);
        repo.requirements.insert(2, req2);
        repo.requirements.insert(3, req3);
        repo.requirement_verification_methods.push((1, 10));
        repo.requirement_verification_methods.push((2, 20));
        repo.requirement_verification_methods.push((3, 10));
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let items = service
            .list_by_project_filtered(7, Some(1), Some(10), Some(100), None)
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, 1);
    }

    #[test]
    fn list_by_project_filtered_returns_empty_when_no_matches() {
        let mut repo = DieselRepoMock::default();
        let mut req1 = requirement(1, 7, "REQ-001");
        req1.status_id = 1;
        repo.requirements.insert(1, req1);
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let items = service
            .list_by_project_filtered(7, Some(999), None, None, None)
            .unwrap();
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn list_by_project_filtered_sorts_by_reference_code() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(1, requirement(1, 7, "REQ-003"));
        repo.requirements.insert(2, requirement(2, 7, "REQ-001"));
        repo.requirements.insert(3, requirement(3, 7, "REQ-002"));
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let items = service
            .list_by_project_filtered(7, None, None, None, None)
            .unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].reference_code, "REQ-001");
        assert_eq!(items[1].reference_code, "REQ-002");
        assert_eq!(items[2].reference_code, "REQ-003");
    }

    #[test]
    fn list_by_project_filtered_sorts_empty_reference_codes_last() {
        let mut repo = DieselRepoMock::default();
        let req1 = requirement(1, 7, "");
        let req2 = requirement(2, 7, "REQ-001");
        let req3 = requirement(3, 7, "");
        repo.requirements.insert(1, req1);
        repo.requirements.insert(2, req2);
        repo.requirements.insert(3, req3);
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let items = service
            .list_by_project_filtered(7, None, None, None, None)
            .unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].reference_code, "REQ-001");
        // Empty reference codes should come after, sorted by ID
        assert!(items[1].reference_code.is_empty() || items[2].reference_code.is_empty());
    }

    #[test]
    fn get_by_parent_id_returns_children() {
        let mut repo = DieselRepoMock::default();
        let mut req1 = requirement(1, 7, "REQ-001");
        req1.parent_id = None;
        let mut req2 = requirement(2, 7, "REQ-002");
        req2.parent_id = Some(1);
        let mut req3 = requirement(3, 7, "REQ-003");
        req3.parent_id = Some(1);
        let mut req4 = requirement(4, 7, "REQ-004");
        req4.parent_id = Some(2);
        repo.requirements.insert(1, req1);
        repo.requirements.insert(2, req2);
        repo.requirements.insert(3, req3);
        repo.requirements.insert(4, req4);
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let children = service.get_by_parent_id(1).unwrap();
        assert_eq!(children.len(), 2);
        assert!(children.iter().any(|r| r.id == 2));
        assert!(children.iter().any(|r| r.id == 3));
    }

    #[test]
    fn get_by_parent_id_returns_empty_when_no_children() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(1, requirement(1, 7, "REQ-001"));
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let children = service.get_by_parent_id(999).unwrap();
        assert_eq!(children.len(), 0);
    }

    #[test]
    fn get_linked_tests_returns_tests_for_requirement() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(1, requirement(1, 7, "REQ-001"));
        let test1 = TestCase {
            id: 10,
            name: "Test 1".into(),
            description: "Desc".into(),
            source: "manual".into(),
            status_id: 1,
            reference_code: "TEST-1".into(),
            parent_id: None,
            project_id: 7,
        };
        repo.tests.insert(10, test1);
        repo.matrices.push(MatrixLink {
            req_id: 1,
            test_id: 10,
            creation_date: timestamp(),
            project_id: 7,
        });
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let tests = service.get_linked_tests(1).unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].id, 10);
    }

    #[test]
    fn create_propagates_validation_errors() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        // Create payload with invalid data that will fail validation
        let mut payload = new_payload();
        payload.title = "".to_string(); // Empty title should fail validation

        let result = service.create(&actor(), payload, &[1]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::BadInput(_)));
    }

    #[test]
    fn update_returns_not_found_when_requirement_does_not_exist() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let payload = new_payload();
        let result = service.update(&actor(), 999, payload, &[1]);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn delete_returns_not_found_when_requirement_does_not_exist() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let result = service.delete(&actor(), 999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn list_by_project_returns_empty_for_nonexistent_project() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let items = service.list_by_project(999).unwrap();
        assert_eq!(items.len(), 0);
    }

    #[test]
    fn get_by_id_returns_not_found_for_nonexistent_id() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = RequirementService::new(&state);

        let result = service.get_by_id(999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }
}

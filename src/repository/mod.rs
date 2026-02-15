pub mod cache;
pub mod cache_middleware;
pub mod diesel_repo;
// Make mock available for both unit tests and integration tests
// This module is only used in test code and does not affect production builds
pub mod diesel_repo_mock;
pub mod errors;

pub use cache::*;
pub use cache_middleware::CacheRepository;
pub use diesel_repo::*;

use crate::models::*;
use errors::RepoError;
use rocket::async_trait;
use rocket::tokio::task;
use std::sync::{Arc, RwLock};

pub trait UserRepository {
    fn get_users_all(&self) -> Result<Vec<User>, RepoError>;
    fn get_user_by_id(&self, user_id: i32) -> Result<User, RepoError>;
    fn get_user_by_username(&self, uname: &str) -> Result<Option<User>, RepoError>;

    fn insert_user(&mut self, new: &NewUser) -> Result<i32, RepoError>;
    fn update_user_password(&mut self, user_id: i32, new_hash: &str) -> Result<(), RepoError>;
    fn update_user(&mut self, user_data: &NewUser) -> Result<bool, RepoError>;
    fn update_user_without_password(&mut self, user_data: &UpdateUser) -> Result<bool, RepoError>;
    fn delete_user(&mut self, user_id: i32) -> Result<User, RepoError>;
}

/// API token lookup for headless auth (e.g. MCP). Returns user and optional project scope.
pub trait ApiTokensRepository {
    fn get_user_by_token_hash(&self, token_hash: &str) -> Result<(User, Option<i32>), RepoError>;
    fn update_api_token_last_used_at(&mut self, token_hash: &str) -> Result<(), RepoError>;
}

pub trait RequirementsRepository {
    fn get_requirement_by_id(&self, requirement_id: i32) -> Result<Requirement, RepoError>;
    fn get_requirements_all(&self) -> Result<Vec<Requirement>, RepoError>;
    fn get_requirements_by_project(&self, project_id: i32) -> Result<Vec<Requirement>, RepoError>;

    /// Filtered list with same semantics as metrics; ordered by reference_code (empty last), then id.
    /// `custom_field_filters`: optional list of (field_id, value) to AND-filter by custom field values.
    #[allow(clippy::too_many_arguments)]
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
    ) -> Result<Vec<Requirement>, RepoError>;

    fn get_verification_method_ids_for_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<i32>, RepoError>;
    /// Verification method IDs for a specific requirement version (for diff).
    fn get_verification_method_ids_for_version(
        &self,
        version_id: i32,
    ) -> Result<Vec<i32>, RepoError>;
    fn get_requirement_ids_by_verification_method(
        &self,
        verification_method_id: i32,
    ) -> Result<Vec<i32>, RepoError>;
    fn set_requirement_verification_methods(
        &mut self,
        requirement_id: i32,
        verification_method_ids: &[i32],
    ) -> Result<(), RepoError>;

    fn insert_new_requirement(&mut self, new: &NewRequirement) -> Result<i32, RepoError>;
    fn edit_requirement(&mut self, new: &NewRequirement) -> Result<bool, RepoError>;
    fn delete_requirement(&mut self, requirement_id: i32) -> Result<Requirement, RepoError>;
    fn update_requirement(&mut self, requirement_id: i32) -> Result<(), RepoError>;

    /// List all versions for a requirement (newest first).
    fn list_requirement_versions(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<RequirementVersion>, RepoError>;
    /// Fetch a single version by id (any requirement).
    fn get_requirement_version_by_id(
        &self,
        version_id: i32,
    ) -> Result<RequirementVersion, RepoError>;

    /// Set approval state of a requirement version (draft→reviewed, reviewed→approved).
    /// Returns the updated version. When transitioning to approved, sets approved_by and approved_at.
    fn set_requirement_version_approval(
        &mut self,
        version_id: i32,
        new_state: &str,
        approved_by_user_id: i32,
    ) -> Result<RequirementVersion, RepoError>;
}

pub trait TestsCaseRepository {
    fn get_test_by_id(&self, test_id: i32) -> Result<TestCase, RepoError>;
    fn get_tests_all(&self) -> Result<Vec<TestCase>, RepoError>;
    fn get_tests_by_project(&self, project_id: i32) -> Result<Vec<TestCase>, RepoError>;
    fn get_requirements_for_test(&self, test_id: i32) -> Result<Vec<Requirement>, RepoError>;
    fn get_tests_for_requirement(&self, requirement_id: i32) -> Result<Vec<TestCase>, RepoError>;
    /// Tests linked to the requirement that are currently marked suspect (impacted by requirement changes).
    fn get_impacted_tests_for_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<TestCase>, RepoError>;

    fn insert_test(&mut self, new: &NewTestCase) -> Result<i32, RepoError>;
    fn edit_test(&mut self, new: &NewTestCase) -> Result<bool, RepoError>;
    fn delete_test(&mut self, test_id: i32) -> Result<TestCase, RepoError>;
    fn update_test_requirement_links(
        &mut self,
        test_id: i32,
        requirement_ids: &[i32],
    ) -> Result<(), RepoError>;
}

pub trait LookupRepository {
    fn get_requirement_status_all(&self) -> Result<Vec<RequirementStatus>, RepoError>;
    fn get_requirement_status_by_id(&self, status_id: i32) -> Result<RequirementStatus, RepoError>;

    fn get_test_status_all(&self) -> Result<Vec<TestStatus>, RepoError>;
    fn get_test_status_by_id(&self, status_id: i32) -> Result<TestStatus, RepoError>;

    fn get_categories_all(&self) -> Result<Vec<Category>, RepoError>;
    fn get_categories_by_project(&self, project_id: i32) -> Result<Vec<Category>, RepoError>;
    fn get_category_by_id(&self, category_id: i32) -> Result<Category, RepoError>;

    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError>;
    fn get_applicability_by_id(&self, applicability_id: i32) -> Result<Applicability, RepoError>;
    fn get_applicability_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<Applicability>, RepoError>;

    fn get_verification_all(&self) -> Result<Vec<VerificationMethod>, RepoError>;
    fn get_verification_by_id(&self, verification_id: i32)
        -> Result<VerificationMethod, RepoError>;
    fn get_verification_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<VerificationMethod>, RepoError>;

    fn create_requirement_status(&mut self, new: &NewRequirementStatus) -> Result<i32, RepoError>;
    fn create_test_status(&mut self, new: &NewTestStatus) -> Result<i32, RepoError>;

    fn insert_new_verification(&mut self, new: &NewVerificationMethod) -> Result<i32, RepoError>;
    fn edit_verification(&mut self, new: &NewVerificationMethod) -> Result<bool, RepoError>;
    fn delete_verification(
        &mut self,
        verification_id: i32,
    ) -> Result<VerificationMethod, RepoError>;
    fn insert_new_category(&mut self, new: &NewCategory) -> Result<i32, RepoError>;
    fn edit_category(&mut self, new: &NewCategory) -> Result<bool, RepoError>;
    fn delete_category(&mut self, category_id: i32) -> Result<Category, RepoError>;

    fn insert_new_applicability(&mut self, new: &NewApplicability) -> Result<i32, RepoError>;
    fn edit_applicability(&mut self, new: &NewApplicability) -> Result<bool, RepoError>;
    fn delete_applicability(&mut self, applicability_id: i32) -> Result<Applicability, RepoError>;
}

pub trait ProjectsRepository {
    fn get_projects_all(&self) -> Result<Vec<Project>, RepoError>;
    fn get_project_by_id(&self, project_id: i32) -> Result<Project, RepoError>;

    fn insert_new_project(&mut self, new: &NewProject) -> Result<i32, RepoError>;
    fn edit_project(&mut self, project_id: i32, update: &UpdateProject) -> Result<bool, RepoError>;
    fn delete_project(&mut self, project_id: i32) -> Result<Project, RepoError>;
}

pub trait ProjectMembersRepository {
    fn get_members_by_project(&self, project_id: i32) -> Result<Vec<ProjectMember>, RepoError>;
    fn get_projects_for_user(&self, user_id: i32) -> Result<Vec<ProjectMember>, RepoError>;

    fn add_project_member(&mut self, new: &NewProjectMember) -> Result<(), RepoError>;
    fn update_project_member_role(
        &mut self,
        project_id: i32,
        user_id: i32,
        role: i32,
    ) -> Result<(), RepoError>;
    fn remove_project_member(&mut self, project_id: i32, user_id: i32) -> Result<(), RepoError>;
}

pub trait MatrixRepository {
    fn get_matrix_by_project(&self, project_id: i32) -> Result<Vec<MatrixLink>, RepoError>;
    fn insert_new_matrix_item(&mut self, new: &NewMatrixLink) -> Result<(), RepoError>;
    /// Mark all traceability links for a requirement as suspect (e.g. after requirement update or approval).
    /// Returns project IDs of affected links so callers can invalidate caches.
    fn mark_links_suspect_for_requirement(
        &mut self,
        requirement_id: i32,
        reason: &str,
        triggering_version_id: Option<i32>,
        triggering_user_id: Option<i32>,
    ) -> Result<Vec<i32>, RepoError>;
    /// Clear suspect flag for one link; records user and timestamp.
    /// Returns (link existed and was updated, project_id of the link if updated) for cache invalidation.
    fn clear_suspect(
        &mut self,
        req_id: i32,
        test_id: i32,
        cleared_by_user_id: i32,
    ) -> Result<(bool, Option<i32>), RepoError>;
}

pub trait CustomFieldRepository {
    fn list_custom_field_definitions_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<CustomFieldDefinition>, RepoError>;
    fn get_custom_field_definition_by_id(
        &self,
        id: i32,
    ) -> Result<CustomFieldDefinition, RepoError>;
    fn create_custom_field_definition(
        &mut self,
        project_id: i32,
        payload: &CustomFieldDefinitionPayload,
    ) -> Result<i32, RepoError>;
    fn update_custom_field_definition(
        &mut self,
        id: i32,
        payload: &CustomFieldDefinitionPayload,
    ) -> Result<(), RepoError>;
    /// Returns number of requirement versions that have a value for this field (for "in use" warning).
    fn count_requirement_versions_using_field(&self, field_id: i32) -> Result<i64, RepoError>;
    fn delete_custom_field_definition(&mut self, id: i32) -> Result<(), RepoError>;

    fn get_custom_field_values_for_version(
        &self,
        version_id: i32,
    ) -> Result<Vec<CustomFieldValueDisplay>, RepoError>;
    fn set_custom_field_values_for_version(
        &mut self,
        version_id: i32,
        values: &[(i32, Option<String>)],
    ) -> Result<(), RepoError>;
}

pub trait BaselineRepository {
    /// Create an immutable baseline: snapshot current requirement_versions and matrix for the project.
    fn create_baseline(
        &mut self,
        project_id: i32,
        created_by: i32,
        payload: &crate::models::NewBaseline,
    ) -> Result<crate::models::Baseline, RepoError>;

    fn list_baselines_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<crate::models::Baseline>, RepoError>;

    fn get_baseline_by_id(&self, baseline_id: i32) -> Result<crate::models::Baseline, RepoError>;

    /// Requirements as at baseline time (built from baseline_requirements + requirement_versions + requirements).
    fn get_requirements_for_baseline(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<Requirement>, RepoError>;

    /// Version id of a requirement as stored in the baseline, if present.
    fn get_baseline_requirement_version_id(
        &self,
        baseline_id: i32,
        requirement_id: i32,
    ) -> Result<Option<i32>, RepoError>;

    fn get_baseline_traceability(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<crate::models::BaselineTraceability>, RepoError>;
}

pub trait LogRepository {
    fn insert_log(&mut self, new: &NewLog) -> Result<(), RepoError>;
    fn get_logs_recent(&self, limit: i64) -> Result<Vec<Log>, RepoError>;
    fn get_logs_by_entity(&self, entity_type: &str, entity_id: i32) -> Result<Vec<Log>, RepoError>;
    fn cleanup_logs(&mut self, days: i64) -> Result<usize, RepoError>;
}

pub trait RequirementCommentsRepository {
    fn insert_requirement_comment(
        &mut self,
        new: &NewRequirementComment,
    ) -> Result<RequirementComment, RepoError>;
    /// List comments for a requirement. If `version_id` is Some, return only comments
    /// for that version or requirement-level (version_id NULL). Order: created_at ASC.
    fn list_comments_by_requirement(
        &self,
        requirement_id: i32,
        version_id: Option<i32>,
    ) -> Result<Vec<RequirementComment>, RepoError>;
}

pub trait Repository:
    ApiTokensRepository
    + UserRepository
    + LookupRepository
    + RequirementsRepository
    + TestsCaseRepository
    + ProjectsRepository
    + ProjectMembersRepository
    + MatrixRepository
    + CustomFieldRepository
    + BaselineRepository
    + LogRepository
    + RequirementCommentsRepository
{
}

impl<T> Repository for T where
    T: ApiTokensRepository
        + UserRepository
        + LookupRepository
        + RequirementsRepository
        + TestsCaseRepository
        + ProjectsRepository
        + ProjectMembersRepository
        + MatrixRepository
        + CustomFieldRepository
        + BaselineRepository
        + LogRepository
        + RequirementCommentsRepository
{
}

#[async_trait]
pub trait RepoLockExt<R>: Send + Sync {
    async fn async_read<F, T>(&self, f: F) -> Result<T, RepoError>
    where
        F: FnOnce(&R) -> Result<T, RepoError> + Send + 'static,
        T: Send + 'static;

    async fn async_write<F, T>(&self, f: F) -> Result<T, RepoError>
    where
        F: FnOnce(&mut R) -> Result<T, RepoError> + Send + 'static,
        T: Send + 'static;
}

#[async_trait]
impl<R> RepoLockExt<R> for Arc<RwLock<R>>
where
    R: Send + Sync + 'static,
{
    async fn async_read<F, T>(&self, f: F) -> Result<T, RepoError>
    where
        F: FnOnce(&R) -> Result<T, RepoError> + Send + 'static,
        T: Send + 'static,
    {
        let repo = Arc::clone(self);
        task::spawn_blocking(move || {
            let guard = repo
                .read()
                .map_err(|_| RepoError::Pool("repo lock poisoned".into()))?;
            f(&*guard)
        })
        .await
        .map_err(|_| RepoError::Pool("async task join error".into()))?
    }

    async fn async_write<F, T>(&self, f: F) -> Result<T, RepoError>
    where
        F: FnOnce(&mut R) -> Result<T, RepoError> + Send + 'static,
        T: Send + 'static,
    {
        let repo = Arc::clone(self);
        task::spawn_blocking(move || {
            let mut guard = repo
                .write()
                .map_err(|_| RepoError::Pool("repo lock poisoned".into()))?;
            f(&mut *guard)
        })
        .await
        .map_err(|_| RepoError::Pool("async task join error".into()))?
    }
}

#[cfg(test)]
mod tests {
    use super::{MatrixRepository, RepoError, RepoLockExt, RequirementsRepository};
    use crate::models::MatrixLink;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::sync::{Arc, RwLock};

    fn test_datetime() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    fn create_test_matrix() -> MatrixLink {
        MatrixLink {
            req_id: 1,
            test_id: 1,
            creation_date: test_datetime(),
            project_id: 1,
            suspect: false,
            suspect_at: None,
            suspect_reason: None,
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: None,
            triggering_user_id: None,
        }
    }

    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        rocket::tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }

    #[test]
    fn repo_lock_ext_async_read_join_error_on_panic() {
        let data: Arc<RwLock<i32>> = Arc::new(RwLock::new(42));
        let result: Result<i32, RepoError> =
            block_on(data.async_read(|_| -> Result<i32, RepoError> { panic!("test panic") }));
        assert!(result.is_err());
        let err = result.unwrap_err();
        match &err {
            RepoError::Pool(msg) => assert!(msg.contains("async task join error")),
            _ => panic!("expected Pool error, got {:?}", err),
        }
    }

    #[test]
    fn repo_lock_ext_async_write_join_error_on_panic() {
        let data: Arc<RwLock<i32>> = Arc::new(RwLock::new(42));
        let result: Result<(), RepoError> =
            block_on(data.async_write(|_| -> Result<(), RepoError> { panic!("test panic") }));
        assert!(result.is_err());
        let err = result.unwrap_err();
        match &err {
            RepoError::Pool(msg) => assert!(msg.contains("async task join error")),
            _ => panic!("expected Pool error, got {:?}", err),
        }
    }

    #[test]
    fn repo_lock_ext_async_read_poisoned_lock() {
        let data: Arc<RwLock<i32>> = Arc::new(RwLock::new(42));
        let data_clone = Arc::clone(&data);
        let _ = std::thread::spawn(move || {
            let _guard = data_clone.write().unwrap();
            panic!("poison");
        })
        .join();
        let result = block_on(data.async_read(|_| Ok::<i32, RepoError>(0)));
        assert!(result.is_err());
        let err = result.unwrap_err();
        match &err {
            RepoError::Pool(msg) => assert!(msg.contains("poisoned")),
            _ => panic!("expected Pool error, got {:?}", err),
        }
    }

    #[test]
    fn repo_lock_ext_async_write_poisoned_lock() {
        let data: Arc<RwLock<i32>> = Arc::new(RwLock::new(42));
        let data_clone = Arc::clone(&data);
        let _ = std::thread::spawn(move || {
            let _guard = data_clone.write().unwrap();
            panic!("poison");
        })
        .join();
        let result = block_on(data.async_write(|_| Ok::<(), RepoError>(())));
        assert!(result.is_err());
        let err = result.unwrap_err();
        match &err {
            RepoError::Pool(msg) => assert!(msg.contains("poisoned")),
            _ => panic!("expected Pool error, got {:?}", err),
        }
    }

    #[test]
    fn matrix_repository_mark_links_suspect_for_requirement() {
        let mut repo = DieselRepoMock::default();
        let mut link1 = create_test_matrix();
        link1.project_id = 7;
        repo.matrices.push(link1);
        let mut link2 = create_test_matrix();
        link2.req_id = 1;
        link2.test_id = 2;
        link2.project_id = 7;
        repo.matrices.push(link2);
        let mut link3 = create_test_matrix();
        link3.req_id = 2;
        link3.test_id = 1;
        link3.project_id = 8;
        repo.matrices.push(link3);

        let result =
            repo.mark_links_suspect_for_requirement(1, "Requirement updated", Some(1), Some(42));
        assert!(result.is_ok());
        let project_ids = result.unwrap();
        assert_eq!(project_ids.len(), 1);
        assert!(project_ids.contains(&7));

        let matrices = repo.get_matrix_by_project(7).unwrap();
        assert_eq!(matrices.len(), 2);
        for m in &matrices {
            if m.req_id == 1 {
                assert!(m.suspect, "link for req 1 should be suspect");
                assert_eq!(m.suspect_reason.as_deref(), Some("Requirement updated"));
            } else {
                assert!(!m.suspect);
            }
        }
    }

    #[test]
    fn matrix_repository_clear_suspect() {
        let mut repo = DieselRepoMock::default();
        let mut link = create_test_matrix();
        link.suspect = true;
        link.suspect_at = Some(test_datetime());
        link.suspect_reason = Some("Requirement updated".into());
        repo.matrices.push(link);

        let (ok, project_id) = repo.clear_suspect(1, 1, 42).unwrap();
        assert!(ok);
        assert_eq!(project_id, Some(1));

        let matrices = repo.get_matrix_by_project(1).unwrap();
        assert_eq!(matrices.len(), 1);
        assert!(!matrices[0].suspect);
        assert_eq!(matrices[0].cleared_by, Some(42));
        assert!(matrices[0].cleared_at.is_some());
    }

    #[test]
    fn matrix_repository_clear_suspect_returns_false_when_link_missing() {
        let mut repo = DieselRepoMock::default();
        let (ok, project_id) = repo.clear_suspect(99, 99, 1).unwrap();
        assert!(!ok);
        assert_eq!(project_id, None);
    }

    #[test]
    fn set_requirement_version_approval_marks_links_suspect() {
        use crate::models::{Requirement, RequirementVersion};
        let mut repo = DieselRepoMock::default();
        let version_id = 10;
        let req_id = 1;
        let project_id = 7;
        repo.requirement_versions.insert(
            version_id,
            RequirementVersion {
                id: version_id,
                requirement_id: req_id,
                title: "Req".into(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                category_id: 1,
                parent_id: None,
                applicability_id: 1,
                justification: None,
                deadline_date: Some(test_datetime()),
                created_at: test_datetime(),
                approval_state: "draft".into(),
                approved_by: None,
                approved_at: None,
            },
        );
        repo.requirements.insert(
            req_id,
            Requirement {
                id: req_id,
                current_version_id: Some(version_id),
                same_as_current: None,
                title: "Req".into(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "R1".into(),
                category_id: 1,
                parent_id: None,
                creation_date: test_datetime(),
                update_date: test_datetime(),
                deadline_date: Some(test_datetime()),
                applicability_id: 1,
                justification: None,
                project_id,
                approval_state: "draft".into(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            },
        );
        let mut link = create_test_matrix();
        link.req_id = req_id;
        link.test_id = 1;
        link.project_id = project_id;
        link.suspect = false;
        repo.matrices.push(link);

        let _ = repo
            .set_requirement_version_approval(version_id, "reviewed", 42)
            .unwrap();

        let matrices = repo.get_matrix_by_project(project_id).unwrap();
        let req_link = matrices.iter().find(|m| m.req_id == req_id).unwrap();
        assert!(
            req_link.suspect,
            "approval transition should mark requirement's links suspect"
        );
        assert_eq!(
            req_link.suspect_reason.as_deref(),
            Some("Approval state changed")
        );
    }
}

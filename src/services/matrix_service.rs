//! Service providing traceability matrix operations.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{ActionType, EntityType, Matrix, NewMatrix, Requirement, TestCase, User};
use crate::repository::errors::RepoError;
use crate::repository::{
    MatrixRepository, PooledConnectionWrapper, RequirementsRepository, TestsRepository,
};
use diesel::prelude::*;
use std::collections::HashSet;

/// High level matrix operations backed by the shared [`AppState`].
pub struct MatrixService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> MatrixService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve every matrix entry.
    pub fn list_all(&self) -> Result<Vec<Matrix>, RepoError> {
        use crate::schema::matrix::dsl::matrix;

        let mut conn = self.db_connection()?;
        matrix
            .load::<Matrix>(conn.as_mut())
            .map_err(RepoError::from)
    }

    /// Retrieve matrix entries scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<Matrix>, RepoError> {
        self.state.repo_read().get_matrix_by_project(project_id)
    }

    /// Generate CSV export data for the traceability matrix.
    /// Returns CSV string with headers and all matrix data filtered by project and optional test status.
    pub fn export_matrix_csv(
        &self,
        project_id: i32,
        test_status_filter: Option<i32>,
    ) -> Result<String, RepoError> {
        let repo = self.state.repo_read();

        // Load all requirements for the project
        let mut reqs = repo.get_requirements_by_project(project_id)?;
        reqs.sort_by_key(|r| r.req_id);

        // Load all tests for the project
        let mut all_tests = repo.get_tests_by_project(project_id)?;
        all_tests.sort_by_key(|t| t.test_id);

        // Filter tests by status if specified
        if let Some(status) = test_status_filter {
            all_tests.retain(|t| t.test_status == status);
        }

        // Load all matrix links for the project
        let all_links = repo.get_matrix_by_project(project_id)?;
        let links: HashSet<(i32, i32)> = all_links
            .into_iter()
            .map(|m| (m.matrix_req_id, m.matrix_test_id))
            .collect();

        // Build CSV
        let mut csv = String::from("Title,Reference");
        for test in &all_tests {
            csv.push_str(&format!(",Test #{}", test.test_id));
        }
        csv.push('\n');

        for req in &reqs {
            csv.push_str(&format!(
                "{},{}",
                Self::csv_escape(&req.req_title),
                Self::csv_escape(&req.req_reference)
            ));

            for test in &all_tests {
                let linked = links.contains(&(req.req_id, test.test_id));
                csv.push_str(if linked { ",✓" } else { ",-" });
            }
            csv.push('\n');
        }

        Ok(csv)
    }

    /// Helper to escape CSV fields containing special characters.
    fn csv_escape(s: &str) -> String {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }

    /// Create a new traceability link between a requirement and a test.
    pub fn link(
        &self,
        actor: &User,
        requirement_id: i32,
        test_id: i32,
        project_id: i32,
    ) -> Result<(), RepoError> {
        let payload = NewMatrix {
            matrix_req_id: requirement_id,
            matrix_test_id: test_id,
            project_id,
        };

        {
            let mut repo = self.state.repo_write();
            repo.insert_new_matrix_item(&payload)?;
        }

        self.log_link_created(actor, &payload);
        Ok(())
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_link_created(&self, actor: &User, entity: &NewMatrix) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.user_id);
            let description = format!(
                "Linked requirement {} with test {}",
                entity.matrix_req_id, entity.matrix_test_id
            );

            if let Err(_err) = Logger::log_custom(
                conn.as_mut(),
                &ctx,
                ActionType::Create,
                EntityType::Matrix,
                None,
                Some(entity.project_id),
                None,
                None,
                Some(description),
            ) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log matrix link {} -> {}: {_err}",
                    entity.matrix_req_id, entity.matrix_test_id
                );
            }
        }
    }

    /// Comprehensive matrix view data with all filters and pagination applied.
    pub fn get_matrix_view(
        &self,
        project_id: i32,
        filters: MatrixFilters,
        pagination: MatrixPagination,
    ) -> Result<MatrixView, RepoError> {
        let repo = self.state.repo_read();

        // Load all requirements
        let mut all_reqs = repo.get_requirements_by_project(project_id)?;

        // Apply requirement filters
        Self::apply_requirement_filters(
            &mut all_reqs,
            filters.req_status,
            filters.category,
            filters.applicability,
            filters.search.as_deref(),
        );

        // Load all tests
        let mut all_tests = repo.get_tests_by_project(project_id)?;
        all_tests.sort_by_key(|t| t.test_id);

        // Filter tests by status
        if let Some(status) = filters.test_status {
            all_tests.retain(|t| t.test_status == status);
        }

        // Load matrix links
        let matrix_links = repo.get_matrix_by_project(project_id)?;
        let links: HashSet<(i32, i32)> = matrix_links
            .into_iter()
            .map(|m| (m.matrix_req_id, m.matrix_test_id))
            .collect();

        let total_requirements = all_reqs.len() as i64;

        // Sort requirements
        Self::sort_requirements(
            &mut all_reqs,
            &pagination.sort_by,
            pagination.sort_order == SortOrder::Desc,
            &links,
        );

        // Paginate
        let total_pages = if total_requirements == 0 {
            1
        } else {
            (total_requirements as f64 / pagination.per_page as f64).ceil() as i64
        };
        let start_idx = ((pagination.page - 1) * pagination.per_page) as usize;
        let end_idx = (start_idx + pagination.per_page as usize).min(all_reqs.len());

        let paginated_reqs = if start_idx < all_reqs.len() {
            all_reqs[start_idx..end_idx].to_vec()
        } else {
            Vec::new()
        };

        // Count total links
        let total_links = paginated_reqs
            .iter()
            .map(|req| {
                all_tests
                    .iter()
                    .filter(|t| links.contains(&(req.req_id, t.test_id)))
                    .count()
            })
            .sum();

        Ok(MatrixView {
            requirements: paginated_reqs,
            tests: all_tests,
            links,
            total_requirements,
            total_links,
            total_pages,
        })
    }

    fn apply_requirement_filters(
        reqs: &mut Vec<Requirement>,
        status_filter: Option<i32>,
        category_filter: Option<i32>,
        applicability_filter: Option<i32>,
        search: Option<&str>,
    ) {
        if let Some(status) = status_filter {
            reqs.retain(|r| r.req_current_status == status);
        }

        if let Some(category) = category_filter {
            reqs.retain(|r| r.req_category == category);
        }

        if let Some(applicability) = applicability_filter {
            reqs.retain(|r| r.req_applicability == applicability);
        }

        if let Some(search_term) = search {
            let search_lower = search_term.to_lowercase();
            reqs.retain(|r| {
                r.req_title.to_lowercase().contains(&search_lower)
                    || r.req_reference.to_lowercase().contains(&search_lower)
                    || r.req_id.to_string().contains(&search_lower)
            });
        }
    }

    fn sort_requirements(
        reqs: &mut Vec<Requirement>,
        sort_by: &str,
        desc: bool,
        links: &HashSet<(i32, i32)>,
    ) {
        // Check if sorting by test column
        if let Some(test_id_str) = sort_by.strip_prefix("test_") {
            if let Ok(target_test_id) = test_id_str.parse::<i32>() {
                reqs.sort_by_key(|r| links.contains(&(r.req_id, target_test_id)));
                if desc {
                    reqs.reverse();
                }
                return;
            }
        }

        // Sort by requirement fields
        match sort_by {
            "req_title" => {
                reqs.sort_by(|a, b| a.req_title.cmp(&b.req_title));
            }
            "req_reference" => {
                reqs.sort_by(|a, b| a.req_reference.cmp(&b.req_reference));
            }
            _ => {
                reqs.sort_by_key(|r| r.req_id);
            }
        }

        if desc {
            reqs.reverse();
        }
    }
}

/// Filter parameters for matrix view
#[derive(Debug, Clone, Default)]
pub struct MatrixFilters {
    pub test_status: Option<i32>,
    pub req_status: Option<i32>,
    pub category: Option<i32>,
    pub applicability: Option<i32>,
    pub search: Option<String>,
}

/// Pagination parameters for matrix view
#[derive(Debug, Clone)]
pub struct MatrixPagination {
    pub page: i64,
    pub per_page: i64,
    pub sort_by: String,
    pub sort_order: SortOrder,
}

impl Default for MatrixPagination {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 50,
            sort_by: "req_id".to_string(),
            sort_order: SortOrder::Asc,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Complete matrix view data ready for rendering
pub struct MatrixView {
    pub requirements: Vec<Requirement>,
    pub tests: Vec<TestCase>,
    pub links: HashSet<(i32, i32)>,
    pub total_requirements: i64,
    pub total_links: usize,
    pub total_pages: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TestCase;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::sync::{Arc, RwLock};

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2023, 1, 1)
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
        DieselRepoMock::make_user(1, "logger", "")
    }

    #[test]
    fn list_all_propagates_connection_error() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let result = service.list_all();
        assert!(matches!(result, Err(RepoError::Pool(_))));
    }

    #[test]
    fn list_by_project_filters_results() {
        let mut repo = DieselRepoMock::default();
        repo.matrices.push(Matrix {
            matrix_req_id: 1,
            matrix_test_id: 10,
            matrix_creation_date: timestamp(),
            project_id: 7,
        });
        repo.matrices.push(Matrix {
            matrix_req_id: 2,
            matrix_test_id: 20,
            matrix_creation_date: timestamp(),
            project_id: 99,
        });

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let results = service.list_by_project(7).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].matrix_test_id, 10);
    }

    #[test]
    fn link_inserts_new_matrix_entry() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        service.link(&actor(), 5, 6, 42).unwrap();

        let entries = service.list_by_project(42).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].matrix_req_id, 5);
        assert_eq!(entries[0].matrix_test_id, 6);
    }

    #[test]
    fn csv_export_generates_correct_format() {
        let mut repo = DieselRepoMock::default();

        // Add a requirement
        repo.requirements.insert(
            1,
            Requirement {
                req_id: 1,
                req_title: "Test Requirement".to_string(),
                req_description: String::new(),
                req_verification_method: 1,
                req_current_status: 1,
                req_author: 1,
                req_reviewer: 1,
                req_reference: "REF-001".to_string(),
                req_category: 1,
                req_parent: 0,
                req_creation_date: timestamp(),
                req_update_date: timestamp(),
                req_deadline_date: timestamp(),
                req_applicability: 1,
                req_justification: None,
                project_id: 1,
            },
        );

        // Add tests
        repo.tests.insert(
            10,
            TestCase {
                test_id: 10,
                test_name: "Test 10".to_string(),
                test_reference: "TST-10".to_string(),
                test_description: String::new(),
                test_source: String::new(),
                test_status: 1,
                test_parent: 0,
                project_id: 1,
            },
        );
        repo.tests.insert(
            20,
            TestCase {
                test_id: 20,
                test_name: "Test 20".to_string(),
                test_reference: "TST-20".to_string(),
                test_description: String::new(),
                test_source: String::new(),
                test_status: 1,
                test_parent: 0,
                project_id: 1,
            },
        );

        // Add matrix link
        repo.matrices.push(Matrix {
            matrix_req_id: 1,
            matrix_test_id: 10,
            matrix_creation_date: timestamp(),
            project_id: 1,
        });

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let csv = service.export_matrix_csv(1, None).unwrap();

        // Check CSV structure
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2); // Header + 1 requirement row
        assert!(lines[0].starts_with("Title,Reference"));
        assert!(lines[0].contains("Test #10"));
        assert!(lines[0].contains("Test #20"));
        assert!(lines[1].starts_with("Test Requirement,REF-001"));
        assert!(lines[1].contains(",✓,")); // Linked to test 10
        assert!(lines[1].ends_with(",-")); // Not linked to test 20
    }

    #[test]
    fn csv_export_handles_special_characters() {
        let mut repo = DieselRepoMock::default();

        repo.requirements.insert(
            1,
            Requirement {
                req_id: 1,
                req_title: "Test, with \"quotes\"".to_string(),
                req_description: String::new(),
                req_verification_method: 1,
                req_current_status: 1,
                req_author: 1,
                req_reviewer: 1,
                req_reference: "REF-001".to_string(),
                req_category: 1,
                req_parent: 0,
                req_creation_date: timestamp(),
                req_update_date: timestamp(),
                req_deadline_date: timestamp(),
                req_applicability: 1,
                req_justification: None,
                project_id: 1,
            },
        );

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let csv = service.export_matrix_csv(1, None).unwrap();

        // Should escape the title properly
        assert!(csv.contains("\"Test, with \"\"quotes\"\"\""));
    }

    #[test]
    fn csv_export_filters_by_test_status() {
        let mut repo = DieselRepoMock::default();

        repo.requirements.insert(
            1,
            Requirement {
                req_id: 1,
                req_title: "Req 1".to_string(),
                req_description: String::new(),
                req_verification_method: 1,
                req_current_status: 1,
                req_author: 1,
                req_reviewer: 1,
                req_reference: "REF-1".to_string(),
                req_category: 1,
                req_parent: 0,
                req_creation_date: timestamp(),
                req_update_date: timestamp(),
                req_deadline_date: timestamp(),
                req_applicability: 1,
                req_justification: None,
                project_id: 1,
            },
        );

        // Test with status 1
        repo.tests.insert(
            10,
            TestCase {
                test_id: 10,
                test_name: "Test 10".to_string(),
                test_reference: "TST-10".to_string(),
                test_description: String::new(),
                test_source: String::new(),
                test_status: 1,
                test_parent: 0,
                project_id: 1,
            },
        );

        // Test with status 2
        repo.tests.insert(
            20,
            TestCase {
                test_id: 20,
                test_name: "Test 20".to_string(),
                test_reference: "TST-20".to_string(),
                test_description: String::new(),
                test_source: String::new(),
                test_status: 2,
                test_parent: 0,
                project_id: 1,
            },
        );

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        // Export with status filter for status 1
        let csv = service.export_matrix_csv(1, Some(1)).unwrap();

        let lines: Vec<&str> = csv.lines().collect();
        assert!(lines[0].contains("Test #10"));
        assert!(!lines[0].contains("Test #20"));
    }
}

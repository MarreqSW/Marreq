//! Service providing traceability matrix operations.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{
    ActionType, EntityType, MatrixLink, NewMatrixLink, Requirement, TestCase, User,
};
use crate::repository::errors::RepoError;
use crate::repository::{
    MatrixRepository, PooledConnectionWrapper, RequirementsRepository, TestsCaseRepository,
};
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
    /// Note: This collects from all projects since there's no get_matrix_all in the MatrixRepository trait.
    pub fn list_all(&self) -> Result<Vec<MatrixLink>, RepoError> {
        use crate::repository::ProjectsRepository;

        let repo = self.state.repo_write();
        // Collect matrix links from all projects
        let projects = repo.get_projects_all()?;
        let mut all_links = Vec::new();
        for project in projects {
            let links = repo.get_matrix_by_project(project.id)?;
            all_links.extend(links);
        }
        Ok(all_links)
    }

    /// Retrieve matrix entries scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<MatrixLink>, RepoError> {
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
        reqs.sort_by_key(|r| r.id);

        // Load all tests for the project
        let mut all_tests = repo.get_tests_by_project(project_id)?;
        all_tests.sort_by_key(|t| t.id);

        // Filter tests by status if specified
        if let Some(status) = test_status_filter {
            all_tests.retain(|t| t.status_id == status);
        }

        // Load all matrix links for the project
        let all_links = repo.get_matrix_by_project(project_id)?;
        let links: HashSet<(i32, i32)> = all_links
            .into_iter()
            .map(|m| (m.req_id, m.test_id))
            .collect();

        // Build CSV
        let mut csv = String::from("Title,Reference");
        for test in &all_tests {
            csv.push_str(&format!(",Test #{}", test.id));
        }
        csv.push('\n');

        for req in &reqs {
            csv.push_str(&format!(
                "{},{}",
                Self::csv_escape(&req.title),
                Self::csv_escape(&req.reference_code)
            ));

            for test in &all_tests {
                let linked = links.contains(&(req.id, test.id));
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
        id: i32,
        project_id: i32,
    ) -> Result<(), RepoError> {
        let payload = NewMatrixLink {
            req_id: requirement_id,
            test_id: id,
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

    fn log_link_created(&self, actor: &User, entity: &NewMatrixLink) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.id);
            let description = format!(
                "Linked requirement {} with test {}",
                entity.req_id, entity.test_id
            );

            if let Err(_err) = Logger::log_custom(
                conn.as_mut(),
                &ctx,
                ActionType::Create,
                EntityType::MatrixLink,
                None,
                Some(entity.project_id),
                None,
                None,
                Some(description),
            ) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log matrix link {} -> {}: {_err}",
                    entity.req_id, entity.test_id
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
        all_tests.sort_by_key(|t| t.id);

        // Filter tests by status
        if let Some(status) = filters.status_id {
            all_tests.retain(|t| t.status_id == status);
        }

        // Load matrix links
        let matrix_links = repo.get_matrix_by_project(project_id)?;
        let links: HashSet<(i32, i32)> = matrix_links
            .into_iter()
            .map(|m| (m.req_id, m.test_id))
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
                    .filter(|t| links.contains(&(req.id, t.id)))
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
            reqs.retain(|r| r.status_id == status);
        }

        if let Some(category) = category_filter {
            reqs.retain(|r| r.category_id == category);
        }

        if let Some(applicability) = applicability_filter {
            reqs.retain(|r| r.applicability_id == applicability);
        }

        if let Some(search_term) = search {
            let search_lower = search_term.to_lowercase();
            reqs.retain(|r| {
                r.title.to_lowercase().contains(&search_lower)
                    || r.reference_code.to_lowercase().contains(&search_lower)
                    || r.id.to_string().contains(&search_lower)
            });
        }
    }

    fn sort_requirements(
        reqs: &mut [Requirement],
        sort_by: &str,
        desc: bool,
        links: &HashSet<(i32, i32)>,
    ) {
        // Check if sorting by test column
        if let Some(test_id_str) = sort_by.strip_prefix("test_") {
            if let Ok(target_test_id) = test_id_str.parse::<i32>() {
                reqs.sort_by_key(|r| links.contains(&(r.id, target_test_id)));
                if desc {
                    reqs.reverse();
                }
                return;
            }
        }

        // Sort by requirement fields or linked tests count
        match sort_by {
            "title" => {
                reqs.sort_by(|a, b| a.title.cmp(&b.title));
            }
            "reference_code" => {
                reqs.sort_by(|a, b| a.reference_code.cmp(&b.reference_code));
            }
            "linked_tests_count" => {
                reqs.sort_by_key(|r| links.iter().filter(|(req_id, _)| *req_id == r.id).count());
            }
            _ => {
                reqs.sort_by_key(|r| r.id);
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
    pub status_id: Option<i32>,
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
            sort_by: "id".to_string(),
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
    fn list_all_returns_empty_when_no_projects() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let result = service.list_all();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn list_by_project_filters_results() {
        let mut repo = DieselRepoMock::default();
        repo.matrices.push(MatrixLink {
            req_id: 1,
            test_id: 10,
            creation_date: timestamp(),
            project_id: 7,
        });
        repo.matrices.push(MatrixLink {
            req_id: 2,
            test_id: 20,
            creation_date: timestamp(),
            project_id: 99,
        });

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let results = service.list_by_project(7).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].test_id, 10);
    }

    #[test]
    fn link_inserts_new_matrix_entry() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        service.link(&actor(), 5, 6, 42).unwrap();

        let entries = service.list_by_project(42).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].req_id, 5);
        assert_eq!(entries[0].test_id, 6);
    }

    #[test]
    fn csv_export_generates_correct_format() {
        let mut repo = DieselRepoMock::default();

        // Add a requirement
        repo.requirements.insert(
            1,
            Requirement {
                id: 1,
                title: "Test Requirement".to_string(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF-001".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );

        // Add tests
        repo.tests.insert(
            10,
            TestCase {
                id: 10,
                name: "Test 10".to_string(),
                reference_code: "TST-10".to_string(),
                description: String::new(),
                source: String::new(),
                status_id: 1,
                parent_id: None,
                project_id: 1,
            },
        );
        repo.tests.insert(
            20,
            TestCase {
                id: 20,
                name: "Test 20".to_string(),
                reference_code: "TST-20".to_string(),
                description: String::new(),
                source: String::new(),
                status_id: 1,
                parent_id: None,
                project_id: 1,
            },
        );

        // Add matrix link
        repo.matrices.push(MatrixLink {
            req_id: 1,
            test_id: 10,
            creation_date: timestamp(),
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
                id: 1,
                title: "Test, with \"quotes\"".to_string(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF-001".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: None,
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
                id: 1,
                title: "Req 1".to_string(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF-1".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );

        // Test with status 1
        repo.tests.insert(
            10,
            TestCase {
                id: 10,
                name: "Test 10".to_string(),
                reference_code: "TST-10".to_string(),
                description: String::new(),
                source: String::new(),
                status_id: 1,
                parent_id: None,
                project_id: 1,
            },
        );

        // Test with status 2
        repo.tests.insert(
            20,
            TestCase {
                id: 20,
                name: "Test 20".to_string(),
                reference_code: "TST-20".to_string(),
                description: String::new(),
                source: String::new(),
                status_id: 2,
                parent_id: None,
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

    #[test]
    fn get_matrix_view_applies_requirement_filters() {
        let mut repo = DieselRepoMock::default();
        let req1 = Requirement {
            id: 1,
            title: "Req 1".to_string(),
            description: String::new(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: "REF-1".to_string(),
            category_id: 10,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            applicability_id: 5,
            justification: None,
            project_id: 1,
        };
        let req2 = Requirement {
            id: 2,
            title: "Req 2".to_string(),
            description: String::new(),
            status_id: 2,
            author_id: 1,
            reviewer_id: 1,
            reference_code: "REF-2".to_string(),
            category_id: 20,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            applicability_id: 5,
            justification: None,
            project_id: 1,
        };
        repo.requirements.insert(1, req1);
        repo.requirements.insert(2, req2);

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let filters = MatrixFilters {
            req_status: Some(1),
            category: None,
            applicability: None,
            status_id: None,
            search: None,
        };
        let pagination = MatrixPagination::default();

        let view = service.get_matrix_view(1, filters, pagination).unwrap();
        assert_eq!(view.requirements.len(), 1);
        assert_eq!(view.requirements[0].status_id, 1);
    }

    #[test]
    fn get_matrix_view_applies_search_filter() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(
            1,
            Requirement {
                id: 1,
                title: "Alpha Requirement".to_string(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF-ALPHA".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );
        repo.requirements.insert(
            2,
            Requirement {
                id: 2,
                title: "Beta Requirement".to_string(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF-BETA".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let filters = MatrixFilters {
            req_status: None,
            category: None,
            applicability: None,
            status_id: None,
            search: Some("Alpha".to_string()),
        };
        let pagination = MatrixPagination::default();

        let view = service.get_matrix_view(1, filters, pagination).unwrap();
        assert_eq!(view.requirements.len(), 1);
        assert_eq!(view.requirements[0].title, "Alpha Requirement");
    }

    #[test]
    fn get_matrix_view_search_filter_is_case_insensitive() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(
            1,
            Requirement {
                id: 1,
                title: "Alpha Requirement".to_string(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF-ALPHA".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let filters = MatrixFilters {
            req_status: None,
            category: None,
            applicability: None,
            status_id: None,
            search: Some("alpha".to_string()),
        };
        let pagination = MatrixPagination::default();

        let view = service.get_matrix_view(1, filters, pagination).unwrap();
        assert_eq!(view.requirements.len(), 1);
    }

    #[test]
    fn get_matrix_view_search_filter_matches_reference_code() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(
            1,
            Requirement {
                id: 1,
                title: "Some Requirement".to_string(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF-123".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let filters = MatrixFilters {
            req_status: None,
            category: None,
            applicability: None,
            status_id: None,
            search: Some("123".to_string()),
        };
        let pagination = MatrixPagination::default();

        let view = service.get_matrix_view(1, filters, pagination).unwrap();
        assert_eq!(view.requirements.len(), 1);
    }

    #[test]
    fn get_matrix_view_paginates_results() {
        let mut repo = DieselRepoMock::default();
        for i in 1..=10 {
            repo.requirements.insert(
                i,
                Requirement {
                    id: i,
                    title: format!("Req {}", i),
                    description: String::new(),
                    status_id: 1,
                    author_id: 1,
                    reviewer_id: 1,
                    reference_code: format!("REF-{}", i),
                    category_id: 1,
                    parent_id: None,
                    creation_date: timestamp(),
                    update_date: timestamp(),
                    deadline_date: Some(timestamp()),
                    applicability_id: 1,
                    justification: None,
                    project_id: 1,
                },
            );
        }

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let filters = MatrixFilters::default();
        let pagination = MatrixPagination {
            page: 2,
            per_page: 3,
            sort_by: "id".to_string(),
            sort_order: SortOrder::Asc,
        };

        let view = service.get_matrix_view(1, filters, pagination).unwrap();
        assert_eq!(view.requirements.len(), 3);
        assert_eq!(view.total_requirements, 10);
        assert_eq!(view.total_pages, 4); // ceil(10/3) = 4
    }

    #[test]
    fn get_matrix_view_sorts_by_title() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(
            1,
            Requirement {
                id: 1,
                title: "Zebra".to_string(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF-1".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );
        repo.requirements.insert(
            2,
            Requirement {
                id: 2,
                title: "Alpha".to_string(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF-2".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let filters = MatrixFilters::default();
        let pagination = MatrixPagination {
            page: 1,
            per_page: 50,
            sort_by: "title".to_string(),
            sort_order: SortOrder::Asc,
        };

        let view = service.get_matrix_view(1, filters, pagination).unwrap();
        assert_eq!(view.requirements.len(), 2);
        assert_eq!(view.requirements[0].title, "Alpha");
        assert_eq!(view.requirements[1].title, "Zebra");
    }

    #[test]
    fn get_matrix_view_sorts_by_reference_code() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(
            1,
            Requirement {
                id: 1,
                title: "Req 1".to_string(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF-Z".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );
        repo.requirements.insert(
            2,
            Requirement {
                id: 2,
                title: "Req 2".to_string(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF-A".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let filters = MatrixFilters::default();
        let pagination = MatrixPagination {
            page: 1,
            per_page: 50,
            sort_by: "reference_code".to_string(),
            sort_order: SortOrder::Asc,
        };

        let view = service.get_matrix_view(1, filters, pagination).unwrap();
        assert_eq!(view.requirements.len(), 2);
        assert_eq!(view.requirements[0].reference_code, "REF-A");
        assert_eq!(view.requirements[1].reference_code, "REF-Z");
    }

    #[test]
    fn get_matrix_view_sorts_descending() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(
            1,
            Requirement {
                id: 1,
                title: "Alpha".to_string(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF-1".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );
        repo.requirements.insert(
            2,
            Requirement {
                id: 2,
                title: "Zebra".to_string(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF-2".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let filters = MatrixFilters::default();
        let pagination = MatrixPagination {
            page: 1,
            per_page: 50,
            sort_by: "title".to_string(),
            sort_order: SortOrder::Desc,
        };

        let view = service.get_matrix_view(1, filters, pagination).unwrap();
        assert_eq!(view.requirements.len(), 2);
        assert_eq!(view.requirements[0].title, "Zebra");
        assert_eq!(view.requirements[1].title, "Alpha");
    }

    #[test]
    fn get_matrix_view_counts_total_links() {
        let mut repo = DieselRepoMock::default();
        repo.requirements.insert(
            1,
            Requirement {
                id: 1,
                title: "Req 1".to_string(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REF-1".to_string(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );
        repo.tests.insert(
            10,
            TestCase {
                id: 10,
                name: "Test 10".to_string(),
                reference_code: "TST-10".to_string(),
                description: String::new(),
                source: String::new(),
                status_id: 1,
                parent_id: None,
                project_id: 1,
            },
        );
        repo.matrices.push(MatrixLink {
            req_id: 1,
            test_id: 10,
            creation_date: timestamp(),
            project_id: 1,
        });

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let filters = MatrixFilters::default();
        let pagination = MatrixPagination::default();

        let view = service.get_matrix_view(1, filters, pagination).unwrap();
        assert_eq!(view.total_links, 1);
    }

    #[test]
    fn csv_escape_handles_commas() {
        assert_eq!(MatrixService::csv_escape("Test,Value"), "\"Test,Value\"");
    }

    #[test]
    fn csv_escape_handles_quotes() {
        assert_eq!(
            MatrixService::csv_escape("Test\"Value"),
            "\"Test\"\"Value\""
        );
    }

    #[test]
    fn csv_escape_handles_newlines() {
        assert_eq!(MatrixService::csv_escape("Test\nValue"), "\"Test\nValue\"");
    }

    #[test]
    fn csv_escape_does_not_escape_normal_strings() {
        assert_eq!(MatrixService::csv_escape("Normal String"), "Normal String");
    }
}

#![cfg(feature = "test-helpers")]

//! Comprehensive integration tests for Matrix Service.
//!
//! Note: The Matrix API endpoint `/api/matrix` uses direct diesel queries which don't work
//! with the mock repository. Instead, these tests verify the MatrixService functionality
//! which contains the core business logic for traceability matrix operations.
//!
//! Tests include:
//! - Matrix link creation
//! - Listing links by project
//! - CSV export functionality
//! - Coverage analysis
//! - Matrix view generation with filters and pagination

use req_man::models::*;
use req_man::services::{MatrixFilters, MatrixPagination, MatrixService, SortOrder};
use std::collections::HashSet;

mod test_support {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use req_man::app::AppState;
    use req_man::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use std::sync::{Arc, RwLock};

    pub type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

    pub fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    pub fn managed_state(repo: DieselRepoMock) -> TestAppState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    pub fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        let mut admin = DieselRepoMock::make_user(1, "admin", "password");
        admin.is_admin = true;
        repo.users.insert(1, admin);

        repo.projects.insert(
            1,
            Project {
                id: 1,
                name: "Test Project".into(),
                description: Some("Description".into()),
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status_id: Some(1),
                owner_id: Some(1),
            },
        );

        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                id: 1,
                title: "Draft".into(),
                description: "".into(),
                tag: "D".into(),
                project_id: 1,
            },
        );

        repo.test_statuses.insert(
            1,
            TestStatus {
                id: 1,
                title: "Not Run".into(),
                description: "".into(),
                tag: "NR".into(),
                project_id: 1,
            },
        );

        repo.test_statuses.insert(
            2,
            TestStatus {
                id: 2,
                title: "Passed".into(),
                description: "".into(),
                tag: "P".into(),
                project_id: 1,
            },
        );

        repo.categories.insert(
            1,
            Category {
                id: 1,
                title: "Systems".into(),
                description: "".into(),
                tag: "SYS".into(),
                project_id: 1,
            },
        );

        repo.verifications.insert(
            1,
            VerificationMethod {
                id: 1,
                title: "Analysis".into(),
                description: "".into(),
                tag: "ANALYSIS".into(),
                project_id: 1,
            },
        );

        repo.applicability.insert(
            1,
            Applicability {
                id: 1,
                title: "All".into(),
                description: "".into(),
                tag: "ALL".into(),
                project_id: 1,
            },
        );

        repo
    }

    pub fn sample_requirement(id: i32, project_id: i32, title: &str) -> Requirement {
        Requirement {
            id: id,
            title: title.to_string(),
            description: format!("{} description", title),
            verification_method_id: 1,
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: format!("REQ-SYS-{:03}", id),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            applicability_id: 1,
            justification: Some("Test justification".into()),
            project_id,
        }
    }

    pub fn sample_test(id: i32, project_id: i32, name: &str) -> TestCase {
        TestCase {
            id: id,
            name: name.to_string(),
            reference_code: format!("TST-{:03}", id),
            description: format!("{} description", name),
            source: "automated".into(),
            status_id: 1,
            parent_id: None,
            project_id,
        }
    }

    pub fn sample_matrix_link(req_id: i32, test_id: i32, project_id: i32) -> MatrixLink {
        MatrixLink {
            req_id: req_id,
            test_id: test_id,
            creation_date: timestamp(),
            project_id,
        }
    }

    pub fn actor() -> User {
        DieselRepoMock::make_user(1, "actor", "password")
    }
}

use test_support::*;

// ============================================================================
// MatrixService::link - Create Matrix Links
// ============================================================================

#[test]
fn link_creates_new_matrix_entry() {
    let repo = base_repo();
    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let result = service.link(&actor(), 5, 10, 1);

    assert!(result.is_ok());

    // Verify the link was created
    let links = service.list_by_project(1).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].req_id, 5);
    assert_eq!(links[0].test_id, 10);
    assert_eq!(links[0].project_id, 1);
}

#[test]
fn link_can_create_multiple_links() {
    let repo = base_repo();
    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    // Create multiple links
    service.link(&actor(), 1, 1, 1).unwrap();
    service.link(&actor(), 1, 2, 1).unwrap();
    service.link(&actor(), 2, 1, 1).unwrap();

    let links = service.list_by_project(1).unwrap();
    assert_eq!(links.len(), 3);
}

#[test]
fn link_sets_correct_project_id() {
    let repo = base_repo();
    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    service.link(&actor(), 100, 200, 42).unwrap();

    let links = service.list_by_project(42).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].project_id, 42);
}

// ============================================================================
// MatrixService::list_by_project - Filter by Project
// ============================================================================

#[test]
fn list_by_project_returns_empty_for_nonexistent_project() {
    let repo = base_repo();
    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let links = service.list_by_project(999).unwrap();
    assert!(links.is_empty());
}

#[test]
fn list_by_project_filters_correctly() {
    let mut repo = base_repo();

    // Add links for different projects
    repo.matrices.push(sample_matrix_link(1, 1, 1));
    repo.matrices.push(sample_matrix_link(2, 2, 1));
    repo.matrices.push(sample_matrix_link(3, 3, 2));
    repo.matrices.push(sample_matrix_link(4, 4, 2));

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let project1_links = service.list_by_project(1).unwrap();
    assert_eq!(project1_links.len(), 2);

    let project2_links = service.list_by_project(2).unwrap();
    assert_eq!(project2_links.len(), 2);
}

// ============================================================================
// MatrixService::export_matrix_csv - CSV Export
// ============================================================================

#[test]
fn export_csv_generates_correct_header_row() {
    let mut repo = base_repo();

    repo.requirements
        .insert(1, sample_requirement(1, 1, "Req 1"));
    repo.tests.insert(1, sample_test(1, 1, "Test 1"));
    repo.tests.insert(2, sample_test(2, 1, "Test 2"));

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let csv = service.export_matrix_csv(1, None).unwrap();

    let lines: Vec<&str> = csv.lines().collect();
    assert!(lines[0].starts_with("Title,Reference"));
    assert!(lines[0].contains("Test #1"));
    assert!(lines[0].contains("Test #2"));
}

#[test]
fn export_csv_shows_linked_requirements() {
    let mut repo = base_repo();

    repo.requirements
        .insert(1, sample_requirement(1, 1, "Linked Req"));
    repo.requirements
        .insert(2, sample_requirement(2, 1, "Unlinked Req"));
    repo.tests.insert(1, sample_test(1, 1, "Test 1"));

    // Link only req 1
    repo.matrices.push(sample_matrix_link(1, 1, 1));

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let csv = service.export_matrix_csv(1, None).unwrap();

    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 3); // Header + 2 requirements

    // First requirement should show checkmark
    assert!(lines[1].contains("Linked Req"));
    assert!(lines[1].contains(",✓"));

    // Second requirement should show dash
    assert!(lines[2].contains("Unlinked Req"));
    assert!(lines[2].contains(",-"));
}

#[test]
fn export_csv_escapes_special_characters() {
    let mut repo = base_repo();

    let mut req = sample_requirement(1, 1, "Quote Test");
    req.title = "Test, with \"quotes\"".to_string();
    repo.requirements.insert(1, req);

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let csv = service.export_matrix_csv(1, None).unwrap();

    // Should properly escape the title
    assert!(csv.contains("\"Test, with \"\"quotes\"\"\""));
}

#[test]
fn export_csv_filters_tests_by_status() {
    let mut repo = base_repo();

    repo.requirements
        .insert(1, sample_requirement(1, 1, "Req 1"));

    let mut test1 = sample_test(1, 1, "Test 1");
    test1.status_id = 1; // Not Run
    repo.tests.insert(1, test1);

    let mut test2 = sample_test(2, 1, "Test 2");
    test2.status_id = 2; // Passed
    repo.tests.insert(2, test2);

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    // Export with status filter for "Passed" (status 2)
    let csv = service.export_matrix_csv(1, Some(2)).unwrap();

    let lines: Vec<&str> = csv.lines().collect();
    // Should only include test 2
    assert!(lines[0].contains("Test #2"));
    assert!(!lines[0].contains("Test #1"));
}

// ============================================================================
// MatrixService::get_matrix_view - Matrix View with Filters
// ============================================================================

#[test]
fn matrix_view_returns_all_requirements_and_tests() {
    let mut repo = base_repo();

    for i in 1..=3 {
        repo.requirements
            .insert(i, sample_requirement(i, 1, &format!("Req {}", i)));
        repo.tests
            .insert(i, sample_test(i, 1, &format!("Test {}", i)));
    }

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let filters = MatrixFilters::default();
    let pagination = MatrixPagination::default();

    let view = service.get_matrix_view(1, filters, pagination).unwrap();

    assert_eq!(view.requirements.len(), 3);
    assert_eq!(view.tests.len(), 3);
    assert_eq!(view.total_requirements, 3);
}

#[test]
fn matrix_view_includes_links() {
    let mut repo = base_repo();

    repo.requirements
        .insert(1, sample_requirement(1, 1, "Req 1"));
    repo.requirements
        .insert(2, sample_requirement(2, 1, "Req 2"));
    repo.tests.insert(1, sample_test(1, 1, "Test 1"));

    repo.matrices.push(sample_matrix_link(1, 1, 1));
    repo.matrices.push(sample_matrix_link(2, 1, 1));

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let filters = MatrixFilters::default();
    let pagination = MatrixPagination::default();

    let view = service.get_matrix_view(1, filters, pagination).unwrap();

    assert_eq!(view.total_links, 2);
    assert!(view.links.contains(&(1, 1)));
    assert!(view.links.contains(&(2, 1)));
}

#[test]
fn matrix_view_filters_by_requirement_status() {
    let mut repo = base_repo();

    repo.requirement_statuses.insert(
        2,
        RequirementStatus {
            id: 2,
            project_id: 1,
            title: "Accepted".into(),
            description: "".into(),
            tag: "A".into(),
        },
    );

    let mut req1 = sample_requirement(1, 1, "Draft Req");
    req1.status_id = 1; // Draft
    repo.requirements.insert(1, req1);

    let mut req2 = sample_requirement(2, 1, "Accepted Req");
    req2.status_id = 2; // Accepted
    repo.requirements.insert(2, req2);

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let mut filters = MatrixFilters::default();
    filters.req_status = Some(2); // Filter for Accepted
    let pagination = MatrixPagination::default();

    let view = service.get_matrix_view(1, filters, pagination).unwrap();

    assert_eq!(view.requirements.len(), 1);
    assert_eq!(view.requirements[0].title, "Accepted Req");
}

#[test]
fn matrix_view_filters_by_test_status() {
    let mut repo = base_repo();

    repo.requirements
        .insert(1, sample_requirement(1, 1, "Req 1"));

    let mut test1 = sample_test(1, 1, "Not Run Test");
    test1.status_id = 1;
    repo.tests.insert(1, test1);

    let mut test2 = sample_test(2, 1, "Passed Test");
    test2.status_id = 2;
    repo.tests.insert(2, test2);

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let mut filters = MatrixFilters::default();
    filters.status_id = Some(2); // Filter for Passed
    let pagination = MatrixPagination::default();

    let view = service.get_matrix_view(1, filters, pagination).unwrap();

    assert_eq!(view.tests.len(), 1);
    assert_eq!(view.tests[0].name, "Passed Test");
}

#[test]
fn matrix_view_searches_requirements() {
    let mut repo = base_repo();

    repo.requirements
        .insert(1, sample_requirement(1, 1, "Authentication Feature"));
    repo.requirements
        .insert(2, sample_requirement(2, 1, "Database Connection"));
    repo.requirements
        .insert(3, sample_requirement(3, 1, "User Authentication"));

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let mut filters = MatrixFilters::default();
    filters.search = Some("auth".to_string());
    let pagination = MatrixPagination::default();

    let view = service.get_matrix_view(1, filters, pagination).unwrap();

    // Should match both requirements containing "auth"
    assert_eq!(view.requirements.len(), 2);
    assert_eq!(view.total_requirements, 2);
}

#[test]
fn matrix_view_paginates_results() {
    let mut repo = base_repo();

    // Create 25 requirements
    for i in 1..=25 {
        repo.requirements
            .insert(i, sample_requirement(i, 1, &format!("Req {}", i)));
    }

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let filters = MatrixFilters::default();
    let mut pagination = MatrixPagination::default();
    pagination.per_page = 10;
    pagination.page = 2; // Get second page

    let view = service.get_matrix_view(1, filters, pagination).unwrap();

    assert_eq!(view.requirements.len(), 10); // Second page of 10
    assert_eq!(view.total_requirements, 25);
    assert_eq!(view.total_pages, 3); // 25 items / 10 per page = 3 pages
}

#[test]
fn matrix_view_sorts_by_title() {
    let mut repo = base_repo();

    repo.requirements
        .insert(1, sample_requirement(1, 1, "Zebra"));
    repo.requirements
        .insert(2, sample_requirement(2, 1, "Alpha"));
    repo.requirements
        .insert(3, sample_requirement(3, 1, "Beta"));

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let filters = MatrixFilters::default();
    let mut pagination = MatrixPagination::default();
    pagination.sort_by = "title".to_string();
    pagination.sort_order = SortOrder::Asc;

    let view = service.get_matrix_view(1, filters, pagination).unwrap();

    assert_eq!(view.requirements[0].title, "Alpha");
    assert_eq!(view.requirements[1].title, "Beta");
    assert_eq!(view.requirements[2].title, "Zebra");
}

#[test]
fn matrix_view_sorts_descending() {
    let mut repo = base_repo();

    for i in 1..=5 {
        repo.requirements
            .insert(i, sample_requirement(i, 1, &format!("Req {}", i)));
    }

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let filters = MatrixFilters::default();
    let mut pagination = MatrixPagination::default();
    pagination.sort_by = "req_id".to_string();
    pagination.sort_order = SortOrder::Desc;

    let view = service.get_matrix_view(1, filters, pagination).unwrap();

    assert_eq!(view.requirements[0].id, 5);
    assert_eq!(view.requirements[4].id, 1);
}

// ============================================================================
// Coverage Analysis Tests
// ============================================================================

#[test]
fn can_calculate_coverage_percentage() {
    let mut repo = base_repo();

    // 4 requirements, 2 tests
    for i in 1..=4 {
        repo.requirements
            .insert(i, sample_requirement(i, 1, &format!("Req {}", i)));
    }
    repo.tests.insert(1, sample_test(1, 1, "Test 1"));
    repo.tests.insert(2, sample_test(2, 1, "Test 2"));

    // Link 3 out of 4 requirements
    repo.matrices.push(sample_matrix_link(1, 1, 1));
    repo.matrices.push(sample_matrix_link(2, 1, 1));
    repo.matrices.push(sample_matrix_link(3, 2, 1));
    // Requirement 4 is not linked (uncovered)

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let links = service.list_by_project(1).unwrap();

    // Calculate coverage
    let covered_reqs: HashSet<i32> = links.iter().map(|l| l.req_id).collect();
    let coverage_percentage = (covered_reqs.len() as f32 / 4.0) * 100.0;

    assert_eq!(coverage_percentage, 75.0); // 3 out of 4 = 75%
}

#[test]
fn identifies_requirements_without_tests() {
    let mut repo = base_repo();

    for i in 1..=5 {
        repo.requirements
            .insert(i, sample_requirement(i, 1, &format!("Req {}", i)));
    }
    repo.tests.insert(1, sample_test(1, 1, "Test 1"));

    // Link only requirements 1, 2, and 3
    repo.matrices.push(sample_matrix_link(1, 1, 1));
    repo.matrices.push(sample_matrix_link(2, 1, 1));
    repo.matrices.push(sample_matrix_link(3, 1, 1));

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let links = service.list_by_project(1).unwrap();
    let covered_reqs: HashSet<i32> = links.iter().map(|l| l.req_id).collect();

    // Requirements 4 and 5 are not covered
    assert!(!covered_reqs.contains(&4));
    assert!(!covered_reqs.contains(&5));
}

#[test]
fn identifies_tests_without_requirements() {
    let mut repo = base_repo();

    repo.requirements
        .insert(1, sample_requirement(1, 1, "Req 1"));
    for i in 1..=5 {
        repo.tests
            .insert(i, sample_test(i, 1, &format!("Test {}", i)));
    }

    // Link only tests 1, 2, and 3 to the requirement
    repo.matrices.push(sample_matrix_link(1, 1, 1));
    repo.matrices.push(sample_matrix_link(1, 2, 1));
    repo.matrices.push(sample_matrix_link(1, 3, 1));

    let state = managed_state(repo);
    let service = MatrixService::new(&state);

    let links = service.list_by_project(1).unwrap();
    let linked_tests: HashSet<i32> = links.iter().map(|l| l.test_id).collect();

    // Tests 4 and 5 are orphaned
    assert!(!linked_tests.contains(&4));
    assert!(!linked_tests.contains(&5));
}

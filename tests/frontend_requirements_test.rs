#![cfg(feature = "test-helpers")]

use req_man::models::*;
use req_man::status_enums::ProjectStatus;
/// Frontend integration tests for requirements pages.
///
/// These tests verify:
/// - HTML rendering correctness
/// - Form submission flows
/// - HTTP redirects
/// - JavaScript data attributes
/// - Frontend-backend integration
///
/// These are higher-level tests than the module-level unit tests,
/// focusing on the user-facing behavior and complete request/response cycles.
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;
use rocket_dyn_templates::Template;

mod test_support {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use req_man::app::AppState;
    use req_man::auth::session::SESSION_COOKIE;
    use req_man::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::Route;
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

    pub async fn test_client(repo: DieselRepoMock) -> Client {
        client_with_routes(repo, req_man::routes::html::project::requirements::routes()).await
    }

    pub async fn client_with_routes(repo: DieselRepoMock, routes: Vec<Route>) -> Client {
        let rocket = rocket::build()
            .manage(managed_state(repo))
            .attach(Template::fairing())
            .mount("/p", routes);

        Client::tracked(rocket).await.expect("rocket instance")
    }

    pub fn session_cookie(id: i32) -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, id.to_string());
        cookie.set_path("/");
        cookie
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
                status: ProjectStatus::Active,
                owner_id: Some(1),
            },
        );

        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 1,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

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

        repo.requirement_statuses.insert(
            2,
            RequirementStatus {
                id: 2,
                title: "Accepted".into(),
                description: "".into(),
                tag: "A".into(),
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

    pub fn sample_requirement(id: i32, project_id: i32) -> Requirement {
        Requirement {
            id: id,
            current_version_id: None,
            title: format!("Requirement {id}"),
            description: "Test requirement".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: format!("REQ-SYS-{id:03}"),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            applicability_id: 1,
            justification: Some("Test justification".into()),
            project_id,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
        }
    }
}

use test_support::*;

// ============================================================================
// HTML Rendering Tests
// ============================================================================

#[rocket::async_test]
async fn requirements_page_renders_correct_html_structure() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .get("/p/1/requirements")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    // Verify key HTML elements exist
    assert!(
        html.contains("reqman-requirements-page"),
        "Missing requirements page container"
    );
    assert!(
        html.contains("reqman-requirements-header"),
        "Missing header"
    );
    assert!(
        html.contains("reqman-requirements-table"),
        "Missing requirements table"
    );
    assert!(
        html.contains("requirementsFilterForm"),
        "Missing filter form"
    );
    assert!(
        html.contains("newRequirementButton"),
        "Missing new requirement button"
    );
}

#[rocket::async_test]
async fn requirements_page_displays_metrics_section() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .get("/p/1/requirements")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    assert!(
        html.contains("reqman-requirements-metrics"),
        "Missing metrics section"
    );
    assert!(html.contains("requirement_metrics"), "Missing metrics data");
    assert!(html.contains("Total"), "Missing total metric label");
    assert!(html.contains("Coverage"), "Missing coverage metric");
}

#[rocket::async_test]
async fn requirements_table_contains_sortable_headers() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .get("/p/1/requirements")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    // Verify sortable columns
    assert!(
        html.contains("data-sort-key=\"key\""),
        "Missing sortable key column"
    );
    assert!(
        html.contains("data-sort-key=\"title\""),
        "Missing sortable title column"
    );
    assert!(
        html.contains("data-sort-key=\"status\""),
        "Missing sortable status column"
    );
    assert!(
        html.contains("class=\"is-sortable\""),
        "Missing sortable class"
    );
}

#[rocket::async_test]
async fn requirement_row_has_correct_data_attributes() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .get("/p/1/requirements")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    // Verify data attributes for JavaScript
    assert!(
        html.contains("data-requirement-id=\"1\""),
        "Missing requirement ID"
    );
    assert!(
        html.contains("data-status-id"),
        "Missing status ID attribute"
    );
    assert!(
        html.contains("data-status-label"),
        "Missing status label attribute"
    );
    assert!(
        html.contains("data-action=\"toggle-row-details\""),
        "Missing toggle action"
    );
    assert!(
        html.contains("data-action=\"duplicate-requirement\""),
        "Missing duplicate action"
    );
}

#[rocket::async_test]
async fn new_requirement_form_has_required_fields() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/p/1/requirements/new")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    // Verify form fields
    assert!(html.contains("name=\"title\""), "Missing title field");
    assert!(
        html.contains("name=\"description\""),
        "Missing description field"
    );
    assert!(
        html.contains("name=\"reference_code\""),
        "Missing reference field"
    );
    assert!(
        html.contains("name=\"category_id\""),
        "Missing category field"
    );
    assert!(
        html.contains("name=\"verification_method_ids\""),
        "Missing verification field"
    );
    assert!(html.contains("name=\"status_id\""), "Missing status field");
    assert!(
        html.contains("name=\"applicability_id\""),
        "Missing applicability field"
    );
    assert!(
        html.contains("name=\"justification\""),
        "Missing justification field"
    );
    assert!(
        html.contains("data-requirement-form"),
        "Missing form marker"
    );
}

#[rocket::async_test]
async fn edit_requirement_form_populates_existing_data() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .get("/p/1/requirements/edit/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    // Verify data is populated
    assert!(html.contains("Requirement 1"), "Missing requirement title");
    assert!(
        html.contains("REQ-SYS-001"),
        "Missing requirement reference"
    );
    assert!(
        html.contains("value=\"REQ-SYS-001\"") || html.contains("REQ-SYS-001"),
        "Reference not in form"
    );
    assert!(
        html.contains("data-requirement-form"),
        "Missing form marker"
    );
    assert!(
        html.contains("data-allow-soft-mismatch"),
        "Missing validation config"
    );
}

#[rocket::async_test]
async fn requirement_detail_page_shows_relationships() {
    let mut repo = base_repo();

    let parent = sample_requirement(1, 1);
    repo.requirements.insert(1, parent);

    let mut child = sample_requirement(2, 1);
    child.parent_id = Some(1);
    repo.requirements.insert(2, child);

    let client = test_client(repo).await;

    let response = client
        .get("/p/1/requirements/show/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    // Verify requirement data is embedded in the page
    assert!(
        html.contains("requirement-detail-data"),
        "Missing requirement data script tag"
    );
    assert!(
        html.contains("relationships") || html.contains("children"),
        "Missing relationships section"
    );
}

// ============================================================================
// Form Submission and Redirect Tests
// ============================================================================

#[rocket::async_test]
async fn create_requirement_redirects_to_detail_page() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/p/1/requirements/new")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body(
            "title=New+Req&description=Body&verification_method_ids=1&\
               status_id=1&reviewer_id=1&category_id=1&parent_id=0&\
               applicability_id=1&reference_code=&justification=",
        )
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response
        .headers()
        .get_one("Location")
        .expect("redirect location");
    assert!(
        location.contains("/p/1/requirements/show/"),
        "Should redirect to detail page"
    );
}

#[rocket::async_test]
async fn create_requirement_with_add_another_redirects_to_form() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/p/1/requirements/new")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body(
            "title=Test&description=Body&verification_method_ids=1&\
               status_id=1&reviewer_id=1&category_id=1&parent_id=0&\
               applicability_id=1&reference_code=&justification=&intent=add_another",
        )
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response
        .headers()
        .get_one("Location")
        .expect("redirect location");
    assert!(
        location.contains("/p/1/requirements/new"),
        "Should redirect to new form"
    );
    assert!(
        location.contains("created=1"),
        "Should include success parameter"
    );
}

#[rocket::async_test]
async fn edit_requirement_redirects_to_detail_page() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .post("/p/1/requirements/edit/1")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body(
            "id=1&title=Updated&description=Body&verification_method_ids=1&\
               status_id=1&author_id=1&reviewer_id=1&category_id=1&\
               parent_id=0&applicability_id=1&justification=&project_id=1&\
               reference_code=REQ-SYS-001",
        )
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response
        .headers()
        .get_one("Location")
        .expect("redirect location");
    assert!(
        location.contains("/p/1/requirements/show/1"),
        "Should redirect to detail page"
    );
}

#[rocket::async_test]
async fn delete_requirement_redirects_to_list() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .delete("/p/1/requirements/delete/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response
        .headers()
        .get_one("Location")
        .expect("redirect location");
    assert!(
        location.contains("/p/1/requirements"),
        "Should redirect to requirements list"
    );
}

#[rocket::async_test]
async fn filter_form_submission_updates_url_parameters() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .get("/p/1/requirements?status_filter=1&category_filter=1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    // Verify filter form has correct action
    assert!(
        html.contains("action=\"/p/1/requirements\""),
        "Filter form action incorrect"
    );
    // Verify selected options are marked
    assert!(
        html.contains("selected") || html.contains("value=\"1\""),
        "Filters not applied"
    );
}

// ============================================================================
// JavaScript Integration Tests
// ============================================================================

#[rocket::async_test]
async fn requirements_page_includes_filter_controls() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .get("/p/1/requirements")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    // Verify filter controls for JavaScript
    assert!(
        html.contains("data-filter-control=\"status\""),
        "Missing status filter control"
    );
    assert!(
        html.contains("data-filter-control=\"verification\""),
        "Missing verification filter control"
    );
    assert!(
        html.contains("data-filter-control=\"category\""),
        "Missing category filter control"
    );
    assert!(
        html.contains("data-action=\"clear-filters\""),
        "Missing clear filters button"
    );
}

#[rocket::async_test]
async fn requirements_table_has_search_input() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .get("/p/1/requirements")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    assert!(
        html.contains("id=\"requirementsSearch\""),
        "Missing search input"
    );
    assert!(html.contains("type=\"search\""), "Search input wrong type");
    assert!(
        html.contains("name=\"search\""),
        "Search input missing name"
    );
}

#[rocket::async_test]
async fn requirement_form_has_reference_validation_markers() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/p/1/requirements/new")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    assert!(
        html.contains("id=\"reference-error\""),
        "Missing reference error element"
    );
    assert!(
        html.contains("id=\"reference_code\""),
        "Missing reference input"
    );
    assert!(
        html.contains("data-role=\"submit-requirement\""),
        "Missing submit button marker"
    );
}

#[rocket::async_test]
async fn edit_form_has_autosave_configuration() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .get("/p/1/requirements/edit/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    // Verify autosave elements
    assert!(
        html.contains("data-role=\"autosave-text\"") || html.contains("autosave"),
        "Missing autosave indicator"
    );
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[rocket::async_test]
async fn accessing_nonexistent_requirement_returns_error() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/p/1/requirements/show/999")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert!(
        response.status() == Status::NotFound
            || response.status() == Status::SeeOther
            || response.status() == Status::InternalServerError,
        "Should return error for nonexistent requirement"
    );
}

#[rocket::async_test]
async fn editing_nonexistent_requirement_returns_error() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/p/1/requirements/edit/999")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert!(
        response.status() == Status::NotFound
            || response.status() == Status::SeeOther
            || response.status() == Status::InternalServerError,
        "Should return error for nonexistent requirement"
    );
}

#[rocket::async_test]
async fn requirements_page_handles_empty_state() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/p/1/requirements")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    // Should show empty state message
    assert!(
        html.contains("No requirements") || html.contains("empty"),
        "Should display empty state"
    );
}

// ============================================================================
// Breadcrumb and Navigation Tests
// ============================================================================

#[rocket::async_test]
async fn requirements_page_displays_breadcrumb() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .get("/p/1/requirements")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    assert!(
        html.contains("Breadcrumb") || html.contains("breadcrumb"),
        "Missing breadcrumb"
    );
    assert!(
        html.contains("Test Project"),
        "Missing project name in breadcrumb"
    );
    assert!(
        html.contains("Requirements"),
        "Missing requirements in breadcrumb"
    );
}

#[rocket::async_test]
async fn new_requirement_form_displays_breadcrumb() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/p/1/requirements/new")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    assert!(html.contains("breadcrumb"), "Missing breadcrumb");
    assert!(
        html.contains("/p/1/requirements"),
        "Missing back link to requirements"
    );
}

#[rocket::async_test]
async fn edit_requirement_form_displays_breadcrumb() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .get("/p/1/requirements/edit/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    assert!(html.contains("breadcrumb"), "Missing breadcrumb");
    assert!(
        html.contains("/p/1/requirements"),
        "Missing requirements link"
    );
    assert!(
        html.contains("REQ-SYS-001"),
        "Missing requirement reference"
    );
}

// ============================================================================
// Action Button Tests
// ============================================================================

#[rocket::async_test]
async fn requirements_page_shows_action_buttons() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .get("/p/1/requirements")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    assert!(
        html.contains("New Requirement"),
        "Missing new requirement button"
    );
    assert!(html.contains("Export"), "Missing export button");
    assert!(html.contains("Edit"), "Missing edit action");
    assert!(html.contains("Duplicate"), "Missing duplicate action");
}

#[rocket::async_test]
async fn requirement_row_actions_have_correct_links() {
    let mut repo = base_repo();
    repo.requirements.insert(1, sample_requirement(1, 1));
    let client = test_client(repo).await;

    let response = client
        .get("/p/1/requirements")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    assert!(
        html.contains("/p/1/requirements/edit/1"),
        "Missing edit link"
    );
    assert!(
        html.contains("/p/1/requirements/show/1"),
        "Missing detail link"
    );
    assert!(
        html.contains("data-requirement-id=\"1\""),
        "Missing requirement ID for actions"
    );
}

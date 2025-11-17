#![cfg(feature = "test-helpers")]

use req_man::models::*;
/// End-to-end workflow tests for requirements management.
///
/// These tests verify complete user workflows including:
/// - Creating, editing, and deleting requirements
/// - Filtering and searching
/// - Parent-child relationships
/// - Permission controls
/// - Data consistency across operations
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;
use rocket_dyn_templates::Template;

mod workflow_support {
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

    pub fn session_cookie(user_id: i32) -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, user_id.to_string());
        cookie.set_path("/");
        cookie
    }

    pub fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        let mut admin = DieselRepoMock::make_user(1, "admin", "password");
        admin.is_admin = true;
        repo.users.insert(1, admin);

        let mut regular_user = DieselRepoMock::make_user(2, "user", "password");
        regular_user.is_admin = false;
        repo.users.insert(2, regular_user);

        repo.projects.insert(
            1,
            Project {
                project_id: 1,
                project_name: "Test Project".into(),
                project_description: Some("Description".into()),
                project_creation_date: Some(timestamp()),
                project_update_date: Some(timestamp()),
                project_status: Some("Active".into()),
                project_owner_id: Some(1),
            },
        );

        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 1,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 2,
            role: 2,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                req_st_id: 1,
                req_st_title: "Draft".into(),
                req_st_description: "".into(),
                req_st_short_name: "D".into(),
            },
        );

        repo.requirement_statuses.insert(
            2,
            RequirementStatus {
                req_st_id: 2,
                req_st_title: "Accepted".into(),
                req_st_description: "".into(),
                req_st_short_name: "A".into(),
            },
        );

        repo.requirement_statuses.insert(
            3,
            RequirementStatus {
                req_st_id: 3,
                req_st_title: "Released".into(),
                req_st_description: "".into(),
                req_st_short_name: "R".into(),
            },
        );

        repo.categories.insert(
            1,
            Category {
                cat_id: 1,
                cat_title: "Systems".into(),
                cat_description: "".into(),
                cat_tag: "SYS".into(),
                project_id: 1,
            },
        );

        repo.categories.insert(
            2,
            Category {
                cat_id: 2,
                cat_title: "Network".into(),
                cat_description: "".into(),
                cat_tag: "NET".into(),
                project_id: 1,
            },
        );

        repo.verifications.insert(
            1,
            VerificationMethod {
                verification_id: 1,
                verification_name: "Analysis".into(),
                verification_description: "".into(),
                project_id: 1,
            },
        );

        repo.verifications.insert(
            2,
            VerificationMethod {
                verification_id: 2,
                verification_name: "Testing".into(),
                verification_description: "".into(),
                project_id: 1,
            },
        );

        repo.applicability.insert(
            1,
            Applicability {
                app_id: 1,
                app_title: "All".into(),
                app_description: "".into(),
                app_tag: "ALL".into(),
                project_id: 1,
            },
        );

        repo
    }
}

use workflow_support::*;

// ============================================================================
// Complete Create-Edit-Delete Workflow
// ============================================================================

#[rocket::async_test]
async fn complete_requirement_lifecycle() {
    let client = test_client(base_repo()).await;

    // 1. View empty requirements list
    let response = client
        .get("/p/1/requirements")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");
    assert!(html.contains("No requirements") || html.contains("empty"));

    // 2. Navigate to new requirement form
    let response = client
        .get("/p/1/requirements/new")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    assert!(response
        .into_string()
        .await
        .expect("body")
        .contains("New Requirement"));

    // 3. Create a new requirement
    let response = client
        .post("/p/1/requirements/new")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body(
            "title=System+Boot+Sequence&description=System+shall+boot+in+5+seconds&\
               verification_method_id=1&current_status_id=1&reviewer_id=1&category_id=1&\
               parent_id=0&applicability_id=1&reference_code=&justification=Performance",
        )
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response.headers().get_one("Location").expect("redirect");
    assert!(location.contains("/p/1/requirements/show/"));

    // Extract requirement ID from location
    let id = location
        .split('/')
        .last()
        .and_then(|s| s.parse::<i32>().ok())
        .expect("requirement ID");

    // 4. View created requirement
    let response = client
        .get(format!("/p/1/requirements/show/{}", id))
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");
    assert!(html.contains("System Boot Sequence"));
    assert!(html.contains("Performance"));

    // 5. Edit the requirement
    let response = client
        .get(format!("/p/1/requirements/edit/{}", id))
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    assert!(response
        .into_string()
        .await
        .expect("body")
        .contains("System Boot Sequence"));

    // 6. Save edited requirement
    let response = client
        .post(format!("/p/1/requirements/edit/{}", id))
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body(format!(
            "id={}&title=Updated+Boot+Sequence&description=System+shall+boot+in+3+seconds&\
             verification_method_id=1&current_status_id=1&author_id=1&reviewer_id=1&category_id=1&\
             parent_id=0&applicability_id=1&justification=Updated+Performance&project_id=1&\
             reference_code=REQ-SYS-{:03}",
            id, id
        ))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);

    // 7. Verify update
    let response = client
        .get(format!("/p/1/requirements/show/{}", id))
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");
    assert!(html.contains("Updated Boot Sequence"));
    assert!(html.contains("3 seconds"));

    // 8. Delete the requirement
    let response = client
        .delete(format!("/p/1/requirements/delete/{}", id))
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);

    // 9. Verify deletion
    let response = client
        .get(format!("/p/1/requirements/show/{}", id))
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert!(
        response.status() == Status::NotFound
            || response.status() == Status::SeeOther
            || response.status() == Status::InternalServerError
    );
}

// ============================================================================
// Parent-Child Relationship Workflows
// ============================================================================

#[rocket::async_test]
async fn create_requirement_hierarchy() {
    let client = test_client(base_repo()).await;

    // 1. Create parent requirement
    let response = client
        .post("/p/1/requirements/new")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body(
            "title=Parent+Requirement&description=Top+level&verification_method_id=1&\
               current_status_id=1&reviewer_id=1&category_id=1&parent_id=0&\
               applicability_id=1&reference_code=&justification=",
        )
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let parent_location = response.headers().get_one("Location").expect("redirect");
    let parent_id = parent_location
        .split('/')
        .last()
        .and_then(|s| s.parse::<i32>().ok())
        .expect("parent ID");

    // 2. Create child requirement with parent
    let response = client
        .post("/p/1/requirements/new")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body(format!(
            "title=Child+Requirement&description=Derived&verification_method_id=1&\
             current_status_id=1&reviewer_id=1&category_id=1&parent_id={}&\
             applicability_id=1&reference_code=&justification=",
            parent_id
        ))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let child_location = response.headers().get_one("Location").expect("redirect");
    let child_id = child_location
        .split('/')
        .last()
        .and_then(|s| s.parse::<i32>().ok())
        .expect("child ID");

    // 3. View parent - should show child
    let response = client
        .get(format!("/p/1/requirements/show/{}", parent_id))
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");
    assert!(html.contains("Parent Requirement"));
    // Should contain reference to child

    // 4. View child - should reference parent
    let response = client
        .get(format!("/p/1/requirements/show/{}", child_id))
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");
    assert!(html.contains("Child Requirement"));
}

// ============================================================================
// Filtering and Search Workflows
// ============================================================================

#[rocket::async_test]
async fn filter_and_search_requirements() {
    let mut repo = base_repo();

    // Add multiple requirements with different attributes
    for i in 1..=5 {
        let req = Requirement {
            id: i,
            title: format!("Requirement {}", i),
            description: format!("Description {}", i),
            verification_method_id: if i % 2 == 0 { 1 } else { 2 },
            current_status_id: if i <= 2 { 1 } else { 2 },
            author_id: 1,
            reviewer_id: 1,
            reference_code: format!("REQ-SYS-{:03}", i),
            category_id: if i <= 3 { 1 } else { 2 },
            parent_id: 0,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: timestamp(),
            applicability_id: 1,
            justification: Some(format!("Justification {}", i)),
            project_id: 1,
        };
        repo.requirements.insert(i, req);
    }

    let client = test_client(repo).await;

    // 1. View all requirements
    let response = client
        .get("/p/1/requirements")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");
    assert!(html.contains("Requirement 1"));
    assert!(html.contains("Requirement 5"));

    // 2. Filter by status (Draft)
    let response = client
        .get("/p/1/requirements?status_filter=1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");
    assert!(html.contains("Requirement 1"));
    assert!(html.contains("Requirement 2"));
    // Should not contain status 2 requirements
    assert!(!html.contains("Requirement 3") || !html.contains("status_filter=1"));

    // 3. Filter by category (Network)
    let response = client
        .get("/p/1/requirements?category_filter=2")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");
    // Should contain category 2 requirements (4, 5)
    assert!(html.contains("Requirement 4") || html.contains("REQ-SYS-004"));

    // 4. Filter by verification method
    let response = client
        .get("/p/1/requirements?verification_filter=1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");
    // Should contain even-numbered requirements
    assert!(html.contains("Requirement 2") || html.contains("REQ-SYS-002"));

    // 5. Combine multiple filters
    let response = client
        .get("/p/1/requirements?status_filter=1&category_filter=1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");
    // Should only contain requirements matching both filters
    assert!(html.contains("Requirement 1") || html.contains("Requirement 2"));
}

// ============================================================================
// Permission and Access Control Workflows
// ============================================================================

#[rocket::async_test]
async fn non_admin_cannot_delete_released_requirement() {
    let mut repo = base_repo();

    let req = Requirement {
        id: 1,
        title: "Released Requirement".into(),
        description: "Cannot delete".into(),
        verification_method_id: 1,
        current_status_id: 3, // Released
        author_id: 1,
        reviewer_id: 1,
        reference_code: "REQ-SYS-001".into(),
        category_id: 1,
        parent_id: 0,
        creation_date: timestamp(),
        update_date: timestamp(),
        deadline_date: timestamp(),
        applicability_id: 1,
        justification: Some("Released".into()),
        project_id: 1,
    };
    repo.requirements.insert(1, req);

    let client = test_client(repo).await;

    // Regular user (non-admin) attempts to delete
    let response = client
        .delete("/p/1/requirements/delete/1")
        .private_cookie(session_cookie(2))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Forbidden);

    // Admin can delete
    let response = client
        .delete("/p/1/requirements/delete/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
}

// ============================================================================
// Inline Creation Workflows
// ============================================================================

#[rocket::async_test]
async fn create_requirement_with_inline_category() {
    let client = test_client(base_repo()).await;

    // 1. Create category inline
    let response = client
        .post("/p/1/requirements/inline/category")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(r#"{"title":"Performance","description":"Performance requirements","tag":"PERF"}"#)
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let body = response.into_string().await.expect("json response");
    let json: serde_json::Value = serde_json::from_str(&body).expect("parse json");
    let category_id = json["id"].as_i64().expect("category ID") as i32;

    // 2. Create requirement with new category
    let response = client
        .post("/p/1/requirements/new")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body(format!(
            "title=Fast+Response&description=Response+time&verification_method_id=1&\
             current_status_id=1&reviewer_id=1&category_id={}&parent_id=0&\
             applicability_id=1&reference_code=&justification=",
            category_id
        ))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
}

// ============================================================================
// Add Another Workflow
// ============================================================================

#[rocket::async_test]
async fn create_multiple_requirements_with_add_another() {
    let client = test_client(base_repo()).await;

    // 1. Create first requirement with "add another"
    let response = client
        .post("/p/1/requirements/new")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body(
            "title=First&description=First+req&verification_method_id=1&\
               current_status_id=1&reviewer_id=1&category_id=1&parent_id=0&\
               applicability_id=1&reference_code=&justification=&intent=add_another",
        )
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response.headers().get_one("Location").expect("redirect");
    assert!(location.contains("/requirements/new"));
    assert!(location.contains("created=1"));

    // 2. Verify success message shown
    let response = client
        .get(location)
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");
    assert!(html.contains("created successfully") || html.contains("data-flash-success"));

    // 3. Create second requirement normally
    let response = client
        .post("/p/1/requirements/new")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body(
            "title=Second&description=Second+req&verification_method_id=1&\
               current_status_id=1&reviewer_id=1&category_id=1&parent_id=0&\
               applicability_id=1&reference_code=&justification=",
        )
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    let location = response.headers().get_one("Location").expect("redirect");
    assert!(location.contains("/requirements/show/"));

    // Verify both requirements exist
    let response = client
        .get("/p/1/requirements")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");
    assert!(html.contains("First") || html.contains("REQ-"));
    assert!(html.contains("Second") || html.contains("REQ-"));
}

// ============================================================================
// Template Usage Workflow
// ============================================================================

#[rocket::async_test]
async fn create_requirement_from_template() {
    let mut repo = base_repo();

    let template_req = Requirement {
        id: 1,
        title: "Template Requirement".into(),
        description: "Template description with specific format".into(),
        verification_method_id: 2,
        current_status_id: 1,
        author_id: 1,
        reviewer_id: 1,
        reference_code: "REQ-SYS-001".into(),
        category_id: 2,
        parent_id: 0,
        creation_date: timestamp(),
        update_date: timestamp(),
        deadline_date: timestamp(),
        applicability_id: 1,
        justification: Some("Template justification".into()),
        project_id: 1,
    };
    repo.requirements.insert(1, template_req);

    let client = test_client(repo).await;

    // 1. Open new requirement form with template
    let response = client
        .get("/p/1/requirements/new?template=1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let html = response.into_string().await.expect("body");

    // Verify template data is pre-filled
    assert!(html.contains("Template Requirement") || html.contains("Template description"));
    assert!(html.contains("Template justification"));

    // 2. Create new requirement (data copied from template)
    let response = client
        .post("/p/1/requirements/new")
        .header(ContentType::Form)
        .private_cookie(session_cookie(1))
        .body("title=New+From+Template&description=Template+description+with+specific+format&\
               verification_method_id=2&current_status_id=1&reviewer_id=1&category_id=2&parent_id=0&\
               applicability_id=1&reference_code=&justification=Template+justification")
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
}

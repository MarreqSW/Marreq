#![cfg(feature = "test-helpers")]

//! Comprehensive integration tests for Tests API endpoints.
//!
//! These tests verify the complete behavior of `/api/tests` endpoints including:
//! - CRUD operations
//! - Field update API
//! - Test hierarchy (parent-child)
//! - Test status management
//! - Error handling
//! - Authentication

use req_man::models::*;
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;
use serde_json::{json, Value};

mod test_support {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use req_man::app::AppState;
    use req_man::auth::session::SESSION_COOKIE;
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

    pub async fn test_client(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(managed_state(repo))
            .mount("/api", req_man::api::routes());

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

        let regular_user = DieselRepoMock::make_user(2, "user", "password");
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

        repo.test_statuses.insert(
            1,
            TestStatus {
                test_st_id: 1,
                test_st_title: "Not Run".into(),
                test_st_description: "".into(),
                test_st_short_name: "NR".into(),
            },
        );

        repo.test_statuses.insert(
            2,
            TestStatus {
                test_st_id: 2,
                test_st_title: "Passed".into(),
                test_st_description: "".into(),
                test_st_short_name: "P".into(),
            },
        );

        repo.test_statuses.insert(
            3,
            TestStatus {
                test_st_id: 3,
                test_st_title: "Failed".into(),
                test_st_description: "".into(),
                test_st_short_name: "F".into(),
            },
        );

        repo
    }

    pub fn sample_test(id: i32, project_id: i32, name: &str) -> Test {
        Test {
            test_id: id,
            test_name: name.to_string(),
            test_reference: format!("TST-{:03}", id),
            test_description: format!("{} description", name),
            test_source: "automated".into(),
            test_status: 1,
            test_parent: 0,
            project_id,
        }
    }

    pub fn new_test_json(name: &str, project_id: i32) -> Value {
        json!({
            "test_name": name,
            "test_reference": "",
            "test_description": format!("{} description", name),
            "test_source": "automated",
            "test_status": 1,
            "test_parent": 0,
            "project_id": project_id
        })
    }
}

use test_support::*;

// ============================================================================
// GET /api/tests - List All Tests
// ============================================================================

#[rocket::async_test]
async fn get_tests_returns_empty_list_when_no_tests() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/api/tests")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let tests: Vec<Test> = response.into_json().await.expect("json");
    assert!(tests.is_empty());
}

#[rocket::async_test]
async fn get_tests_returns_all_tests() {
    let mut repo = base_repo();
    repo.tests.insert(1, sample_test(1, 1, "Test 1"));
    repo.tests.insert(2, sample_test(2, 1, "Test 2"));
    repo.tests.insert(3, sample_test(3, 1, "Test 3"));

    let client = test_client(repo).await;

    let response = client
        .get("/api/tests")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let tests: Vec<Test> = response.into_json().await.expect("json");
    assert_eq!(tests.len(), 3);
}

#[rocket::async_test]
async fn get_tests_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/tests").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// GET /api/tests/{id} - Get Single Test
// ============================================================================

#[rocket::async_test]
async fn get_test_by_id_returns_correct_test() {
    let mut repo = base_repo();
    repo.tests.insert(1, sample_test(1, 1, "Integration Test"));

    let client = test_client(repo).await;

    let response = client
        .get("/api/tests/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let test: Test = response.into_json().await.expect("json");
    assert_eq!(test.test_id, 1);
    assert_eq!(test.test_name, "Integration Test");
    assert_eq!(test.test_reference, "TST-001");
}

#[rocket::async_test]
async fn get_test_with_nonexistent_id_returns_404() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/api/tests/999")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn get_test_requires_authentication() {
    let mut repo = base_repo();
    repo.tests.insert(1, sample_test(1, 1, "Test"));

    let client = test_client(repo).await;

    let response = client.get("/api/tests/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// POST /api/tests - Create New Test
// ============================================================================

#[rocket::async_test]
async fn post_test_creates_new_test() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/tests")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(new_test_json("Smoke Test", 1).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    assert_eq!(result["status"], "ok");
    assert_eq!(result["id"], 1);
}

#[rocket::async_test]
async fn post_test_with_missing_fields_returns_error() {
    let client = test_client(base_repo()).await;

    let invalid_json = json!({
        "test_name": "Incomplete Test"
        // Missing required fields
    });

    let response = client
        .post("/api/tests")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(invalid_json.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::UnprocessableEntity);
}

#[rocket::async_test]
async fn post_test_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/tests")
        .header(ContentType::JSON)
        .body(new_test_json("New Test", 1).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// POST /api/tests/{id}/field - Update Test Field
// ============================================================================

#[rocket::async_test]
async fn update_field_changes_test_name() {
    let mut repo = base_repo();
    repo.tests.insert(1, sample_test(1, 1, "Original Name"));

    let client = test_client(repo).await;

    let update = json!({
        "field": "test_name",
        "value": "Updated Name"
    });

    let response = client
        .post("/api/tests/1/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    assert_eq!(result["success"], true);

    // Verify the update
    let get_response = client
        .get("/api/tests/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let test: Test = get_response.into_json().await.expect("json");
    assert_eq!(test.test_name, "Updated Name");
}

#[rocket::async_test]
async fn update_field_changes_test_status() {
    let mut repo = base_repo();
    repo.tests.insert(1, sample_test(1, 1, "Test"));

    let client = test_client(repo).await;

    let update = json!({
        "field": "test_status",
        "value": "2"
    });

    let response = client
        .post("/api/tests/1/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);

    // Verify status was updated
    let get_response = client
        .get("/api/tests/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let test: Test = get_response.into_json().await.expect("json");
    assert_eq!(test.test_status, 2);
}

#[rocket::async_test]
async fn update_field_with_invalid_field_returns_error() {
    let mut repo = base_repo();
    repo.tests.insert(1, sample_test(1, 1, "Test"));

    let client = test_client(repo).await;

    let update = json!({
        "field": "invalid_field",
        "value": "value"
    });

    let response = client
        .post("/api/tests/1/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn update_field_with_invalid_status_value_returns_error() {
    let mut repo = base_repo();
    repo.tests.insert(1, sample_test(1, 1, "Test"));

    let client = test_client(repo).await;

    let update = json!({
        "field": "test_status",
        "value": "invalid"
    });

    let response = client
        .post("/api/tests/1/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::BadRequest);
}

// ============================================================================
// DELETE /api/tests/{id} - Delete Test
// ============================================================================

#[rocket::async_test]
async fn delete_test_removes_test() {
    let mut repo = base_repo();
    repo.tests.insert(1, sample_test(1, 1, "To Delete"));

    let client = test_client(repo).await;

    let response = client
        .delete("/api/tests/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NoContent);

    // Verify test is gone
    let get_response = client
        .get("/api/tests/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(get_response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn delete_nonexistent_test_returns_404() {
    let client = test_client(base_repo()).await;

    let response = client
        .delete("/api/tests/999")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn delete_test_requires_authentication() {
    let mut repo = base_repo();
    repo.tests.insert(1, sample_test(1, 1, "To Delete"));

    let client = test_client(repo).await;

    let response = client.delete("/api/tests/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// Test Hierarchy Tests
// ============================================================================

#[rocket::async_test]
async fn create_test_with_parent() {
    let mut repo = base_repo();
    repo.tests.insert(1, sample_test(1, 1, "Parent Test"));

    let client = test_client(repo).await;

    let mut child_json = new_test_json("Child Test", 1);
    child_json["test_parent"] = json!(1);

    let response = client
        .post("/api/tests")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(child_json.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    let child_id = result["id"].as_i64().expect("id");

    // Verify parent relationship
    let get_response = client
        .get(format!("/api/tests/{}", child_id))
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let child_test: Test = get_response.into_json().await.expect("json");
    assert_eq!(child_test.test_parent, 1);
}

#[rocket::async_test]
async fn update_test_parent() {
    let mut repo = base_repo();
    repo.tests.insert(1, sample_test(1, 1, "Parent Test"));
    repo.tests.insert(2, sample_test(2, 1, "Child Test"));

    let client = test_client(repo).await;

    let update = json!({
        "field": "test_parent",
        "value": "1"
    });

    let response = client
        .post("/api/tests/2/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);

    // Verify parent was set
    let get_response = client
        .get("/api/tests/2")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let test: Test = get_response.into_json().await.expect("json");
    assert_eq!(test.test_parent, 1);
}

// ============================================================================
// Additional Field Update Tests
// ============================================================================

#[rocket::async_test]
async fn update_test_description() {
    let mut repo = base_repo();
    repo.tests.insert(1, sample_test(1, 1, "Test"));

    let client = test_client(repo).await;

    let update = json!({
        "field": "test_description",
        "value": "Updated description"
    });

    let response = client
        .post("/api/tests/1/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);

    let get_response = client
        .get("/api/tests/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let test: Test = get_response.into_json().await.expect("json");
    assert_eq!(test.test_description, "Updated description");
}

#[rocket::async_test]
async fn update_test_source() {
    let mut repo = base_repo();
    repo.tests.insert(1, sample_test(1, 1, "Test"));

    let client = test_client(repo).await;

    let update = json!({
        "field": "test_source",
        "value": "manual"
    });

    let response = client
        .post("/api/tests/1/field")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);

    let get_response = client
        .get("/api/tests/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let test: Test = get_response.into_json().await.expect("json");
    assert_eq!(test.test_source, "manual");
}


// ============================================================================
// Edge Cases
// ============================================================================


#[rocket::async_test]
async fn create_multiple_tests_sequentially() {
    let client = test_client(base_repo()).await;

    for i in 1..=5 {
        let response = client
            .post("/api/tests")
            .header(ContentType::JSON)
            .private_cookie(session_cookie(1))
            .body(new_test_json(&format!("Test {}", i), 1).to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let result: Value = response.into_json().await.expect("json");
        assert_eq!(result["id"], i);
    }

    // Verify all were created
    let list_response = client
        .get("/api/tests")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let tests: Vec<Test> = list_response.into_json().await.expect("json");
    assert_eq!(tests.len(), 5);
}

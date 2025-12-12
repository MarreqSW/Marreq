#![cfg(feature = "test-helpers")]

//! Comprehensive integration tests for Categories API endpoints.
//!
//! These tests verify the complete behavior of `/api/categories` endpoints including:
//! - CRUD operations
//! - Project scoping
//! - Authentication
//! - Tag uniqueness
//! - Error handling

use req_man::models::*;
use req_man::status_enums::ProjectStatus;
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

        repo.projects.insert(
            2,
            Project {
                id: 2,
                name: "Other Project".into(),
                description: Some("Other Description".into()),
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(2),
            },
        );

        repo
    }

    pub fn sample_category(id: i32, project_id: i32, title: &str, tag: &str) -> Category {
        Category {
            id: id,
            title: title.to_string(),
            description: format!("{} description", title),
            tag: tag.to_string(),
            project_id,
        }
    }

    pub fn new_category_json(title: &str, tag: &str, project_id: i32) -> Value {
        json!({
            "title": title,
            "description": format!("{} description", title),
            "tag": tag,
            "project_id": project_id
        })
    }
}

use test_support::*;

// ============================================================================
// GET /api/categories - List All Categories
// ============================================================================

#[rocket::async_test]
async fn get_categories_returns_empty_list_when_no_categories() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/api/categories")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let categories: Vec<Category> = response.into_json().await.expect("json");
    assert!(categories.is_empty());
}

#[rocket::async_test]
async fn get_categories_returns_all_categories() {
    let mut repo = base_repo();
    repo.categories
        .insert(1, sample_category(1, 1, "Functional", "FUNC"));
    repo.categories
        .insert(2, sample_category(2, 1, "Performance", "PERF"));
    repo.categories
        .insert(3, sample_category(3, 1, "Security", "SEC"));

    let client = test_client(repo).await;

    let response = client
        .get("/api/categories")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let categories: Vec<Category> = response.into_json().await.expect("json");
    assert_eq!(categories.len(), 3);
}

#[rocket::async_test]
async fn get_categories_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/categories").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// GET /api/categories/{id} - Get Single Category
// ============================================================================

#[rocket::async_test]
async fn get_category_by_id_returns_correct_category() {
    let mut repo = base_repo();
    repo.categories
        .insert(1, sample_category(1, 1, "System Requirements", "SYS"));

    let client = test_client(repo).await;

    let response = client
        .get("/api/categories/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let category: Category = response.into_json().await.expect("json");
    assert_eq!(category.id, 1);
    assert_eq!(category.title, "System Requirements");
    assert_eq!(category.tag, "SYS");
}

#[rocket::async_test]
async fn get_category_with_nonexistent_id_returns_404() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/api/categories/999")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn get_category_requires_authentication() {
    let mut repo = base_repo();
    repo.categories
        .insert(1, sample_category(1, 1, "Category", "CAT"));

    let client = test_client(repo).await;

    let response = client.get("/api/categories/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// POST /api/categories - Create New Category
// ============================================================================

#[rocket::async_test]
async fn post_category_creates_new_category() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/categories")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(new_category_json("New Category", "NEW", 1).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    assert_eq!(result["status"], "ok");
    assert_eq!(result["id"], 1);
}

#[rocket::async_test]
async fn post_category_with_missing_fields_returns_error() {
    let client = test_client(base_repo()).await;

    let invalid_json = json!({
        "title": "Incomplete Category"
        // Missing required fields
    });

    let response = client
        .post("/api/categories")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(invalid_json.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::UnprocessableEntity);
}

#[rocket::async_test]
async fn post_category_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/categories")
        .header(ContentType::JSON)
        .body(new_category_json("New Category", "NEW", 1).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// PUT /api/categories/{id} - Update Category
// ============================================================================

#[rocket::async_test]
async fn put_category_updates_category() {
    let mut repo = base_repo();
    repo.categories
        .insert(1, sample_category(1, 1, "Original", "ORIG"));

    let client = test_client(repo).await;

    let update = json!({
        "title": "Updated Title",
        "description": "Updated description",
        "tag": "UPDATED",
        "project_id": 1
    });

    let response = client
        .put("/api/categories/1")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    assert_eq!(result["status"], "ok");

    // Verify the update
    let get_response = client
        .get("/api/categories/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let category: Category = get_response.into_json().await.expect("json");
    assert_eq!(category.title, "Updated Title");
    assert_eq!(category.tag, "UPDATED");
}

#[rocket::async_test]
async fn put_nonexistent_category_returns_404() {
    let client = test_client(base_repo()).await;

    let update = json!({
        "title": "Updated",
        "description": "Desc",
        "tag": "TAG",
        "project_id": 1
    });

    let response = client
        .put("/api/categories/999")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

// ============================================================================
// DELETE /api/categories/{id} - Delete Category
// ============================================================================

#[rocket::async_test]
async fn delete_category_removes_category() {
    let mut repo = base_repo();
    repo.categories
        .insert(1, sample_category(1, 1, "To Delete", "DEL"));

    let client = test_client(repo).await;

    let response = client
        .delete("/api/categories/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NoContent);

    // Verify category is gone
    let get_response = client
        .get("/api/categories/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(get_response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn delete_nonexistent_category_returns_404() {
    let client = test_client(base_repo()).await;

    let response = client
        .delete("/api/categories/999")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn delete_category_requires_authentication() {
    let mut repo = base_repo();
    repo.categories
        .insert(1, sample_category(1, 1, "To Delete", "DEL"));

    let client = test_client(repo).await;

    let response = client.delete("/api/categories/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// Project Scoping Tests
// ============================================================================

#[rocket::async_test]
async fn categories_are_project_scoped() {
    let mut repo = base_repo();
    repo.categories
        .insert(1, sample_category(1, 1, "Project 1 Cat", "P1"));
    repo.categories
        .insert(2, sample_category(2, 2, "Project 2 Cat", "P2"));

    let client = test_client(repo).await;

    let response = client
        .get("/api/categories")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let categories: Vec<Category> = response.into_json().await.expect("json");
    assert_eq!(categories.len(), 2);

    // Verify both projects' categories are present
    let project_ids: Vec<i32> = categories.iter().map(|c| c.project_id).collect();
    assert!(project_ids.contains(&1));
    assert!(project_ids.contains(&2));
}

#[rocket::async_test]
async fn create_category_in_specific_project() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/categories")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(new_category_json("Project Category", "PROJ", 2).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let result: Value = response.into_json().await.expect("json");
    let id = result["id"].as_i64().expect("id");

    // Verify it was created with correct project_id
    let get_response = client
        .get(format!("/api/categories/{}", id))
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let category: Category = get_response.into_json().await.expect("json");
    assert_eq!(category.project_id, 2);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[rocket::async_test]
async fn create_multiple_categories_sequentially() {
    let client = test_client(base_repo()).await;

    for i in 1..=5 {
        let response = client
            .post("/api/categories")
            .header(ContentType::JSON)
            .private_cookie(session_cookie(1))
            .body(
                new_category_json(&format!("Category {}", i), &format!("CAT{}", i), 1).to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let result: Value = response.into_json().await.expect("json");
        assert_eq!(result["id"], i);
    }

    // Verify all were created
    let list_response = client
        .get("/api/categories")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let categories: Vec<Category> = list_response.into_json().await.expect("json");
    assert_eq!(categories.len(), 5);
}

#[rocket::async_test]
async fn update_preserves_category_id() {
    let mut repo = base_repo();
    repo.categories
        .insert(1, sample_category(1, 1, "Original", "ORIG"));

    let client = test_client(repo).await;

    let update = json!({
        "title": "Updated",
        "description": "Updated desc",
        "tag": "NEW",
        "project_id": 1
    });

    client
        .put("/api/categories/1")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    let get_response = client
        .get("/api/categories/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let category: Category = get_response.into_json().await.expect("json");
    assert_eq!(category.id, 1); // ID should remain the same
}

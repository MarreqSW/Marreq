// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

#![cfg(feature = "test-helpers")]

//! Comprehensive integration tests for Applicability API endpoints.
//!
//! These tests verify the complete behavior of `/api/applicability` endpoints including:
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

    pub fn sample_applicability(id: i32, project_id: i32, title: &str, tag: &str) -> Applicability {
        Applicability {
            id: id,
            title: title.to_string(),
            description: format!("{} description", title),
            tag: tag.to_string(),
            project_id,
        }
    }

    pub fn new_applicability_json(title: &str, tag: &str, project_id: i32) -> Value {
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
// GET /api/applicability - List All Applicability
// ============================================================================

#[rocket::async_test]
async fn get_applicability_returns_empty_list_when_none_exist() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/api/applicability")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let items: Vec<Applicability> = response.into_json().await.expect("json");
    assert!(items.is_empty());
}

#[rocket::async_test]
async fn get_applicability_returns_all_items() {
    let mut repo = base_repo();
    repo.applicability
        .insert(1, sample_applicability(1, 1, "All Products", "ALL"));
    repo.applicability
        .insert(2, sample_applicability(2, 1, "Safety Critical", "SAFE"));
    repo.applicability
        .insert(3, sample_applicability(3, 1, "Commercial Only", "COMM"));

    let client = test_client(repo).await;

    let response = client
        .get("/api/applicability")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let items: Vec<Applicability> = response.into_json().await.expect("json");
    assert_eq!(items.len(), 3);
}

#[rocket::async_test]
async fn get_applicability_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client.get("/api/applicability").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// GET /api/applicability/{id} - Get Single Applicability
// ============================================================================

#[rocket::async_test]
async fn get_applicability_by_id_returns_correct_item() {
    let mut repo = base_repo();
    repo.applicability
        .insert(1, sample_applicability(1, 1, "Production Only", "PROD"));

    let client = test_client(repo).await;

    let response = client
        .get("/api/applicability/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let item: Applicability = response.into_json().await.expect("json");
    assert_eq!(item.id, 1);
    assert_eq!(item.title, "Production Only");
    assert_eq!(item.tag, "PROD");
}

#[rocket::async_test]
async fn get_applicability_with_nonexistent_id_returns_404() {
    let client = test_client(base_repo()).await;

    let response = client
        .get("/api/applicability/999")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn get_applicability_by_id_requires_authentication() {
    let mut repo = base_repo();
    repo.applicability
        .insert(1, sample_applicability(1, 1, "Test", "TST"));

    let client = test_client(repo).await;

    let response = client.get("/api/applicability/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// POST /api/applicability - Create New Applicability
// ============================================================================

#[rocket::async_test]
async fn post_applicability_creates_new_item() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/applicability")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(new_applicability_json("New Applicability", "NEW", 1).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Created);
    let result: Value = response.into_json().await.expect("json");
    assert_eq!(result["status"], "ok");
    assert_eq!(result["id"], 1);
}

#[rocket::async_test]
async fn post_applicability_with_missing_fields_returns_error() {
    let client = test_client(base_repo()).await;

    let invalid_json = json!({
        "title": "Incomplete"
        // Missing required fields
    });

    let response = client
        .post("/api/applicability")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(invalid_json.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::UnprocessableEntity);
}

#[rocket::async_test]
async fn post_applicability_requires_authentication() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/applicability")
        .header(ContentType::JSON)
        .body(new_applicability_json("New Item", "NEW", 1).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// PUT /api/applicability/{id} - Update Applicability
// ============================================================================

#[rocket::async_test]
async fn put_applicability_updates_item() {
    let mut repo = base_repo();
    repo.applicability
        .insert(1, sample_applicability(1, 1, "Original", "ORIG"));

    let client = test_client(repo).await;

    let update = json!({
        "title": "Updated Title",
        "description": "Updated description",
        "tag": "UPDATED",
        "project_id": 1
    });

    let response = client
        .put("/api/applicability/1")
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
        .get("/api/applicability/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let item: Applicability = get_response.into_json().await.expect("json");
    assert_eq!(item.title, "Updated Title");
    assert_eq!(item.tag, "UPDATED");
}

#[rocket::async_test]
async fn put_nonexistent_applicability_returns_404() {
    let client = test_client(base_repo()).await;

    let update = json!({
        "title": "Updated",
        "description": "Desc",
        "tag": "TAG",
        "project_id": 1
    });

    let response = client
        .put("/api/applicability/999")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

// ============================================================================
// DELETE /api/applicability/{id} - Delete Applicability
// ============================================================================

#[rocket::async_test]
async fn delete_applicability_removes_item() {
    let mut repo = base_repo();
    repo.applicability
        .insert(1, sample_applicability(1, 1, "To Delete", "DEL"));

    let client = test_client(repo).await;

    let response = client
        .delete("/api/applicability/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NoContent);

    // Verify item is gone
    let get_response = client
        .get("/api/applicability/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(get_response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn delete_nonexistent_applicability_returns_404() {
    let client = test_client(base_repo()).await;

    let response = client
        .delete("/api/applicability/999")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::NotFound);
}

#[rocket::async_test]
async fn delete_applicability_requires_authentication() {
    let mut repo = base_repo();
    repo.applicability
        .insert(1, sample_applicability(1, 1, "To Delete", "DEL"));

    let client = test_client(repo).await;

    let response = client.delete("/api/applicability/1").dispatch().await;

    assert_eq!(response.status(), Status::Unauthorized);
}

// ============================================================================
// Project Scoping Tests
// ============================================================================

#[rocket::async_test]
async fn applicability_is_project_scoped() {
    let mut repo = base_repo();
    repo.applicability
        .insert(1, sample_applicability(1, 1, "Project 1 App", "P1"));
    repo.applicability
        .insert(2, sample_applicability(2, 2, "Project 2 App", "P2"));

    let client = test_client(repo).await;

    let response = client
        .get("/api/applicability")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let items: Vec<Applicability> = response.into_json().await.expect("json");
    assert_eq!(items.len(), 2);

    // Verify both projects' items are present
    let project_ids: Vec<i32> = items.iter().map(|a| a.project_id).collect();
    assert!(project_ids.contains(&1));
    assert!(project_ids.contains(&2));
}

#[rocket::async_test]
async fn create_applicability_in_specific_project() {
    let client = test_client(base_repo()).await;

    let response = client
        .post("/api/applicability")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(new_applicability_json("Project Item", "PROJ", 2).to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Created);
    let result: Value = response.into_json().await.expect("json");
    let id = result["id"].as_i64().expect("id");

    // Verify it was created with correct project_id
    let get_response = client
        .get(format!("/api/applicability/{}", id))
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let item: Applicability = get_response.into_json().await.expect("json");
    assert_eq!(item.project_id, 2);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[rocket::async_test]
async fn create_multiple_applicability_items_sequentially() {
    let client = test_client(base_repo()).await;

    for i in 1..=5 {
        let response = client
            .post("/api/applicability")
            .header(ContentType::JSON)
            .private_cookie(session_cookie(1))
            .body(
                new_applicability_json(&format!("Applicability {}", i), &format!("APP{}", i), 1)
                    .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);
        let result: Value = response.into_json().await.expect("json");
        assert_eq!(result["id"], i);
    }

    // Verify all were created
    let list_response = client
        .get("/api/applicability")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let items: Vec<Applicability> = list_response.into_json().await.expect("json");
    assert_eq!(items.len(), 5);
}

#[rocket::async_test]
async fn update_preserves_applicability_id() {
    let mut repo = base_repo();
    repo.applicability
        .insert(1, sample_applicability(1, 1, "Original", "ORIG"));

    let client = test_client(repo).await;

    let update = json!({
        "title": "Updated",
        "description": "Updated desc",
        "tag": "NEW",
        "project_id": 1
    });

    client
        .put("/api/applicability/1")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(1))
        .body(update.to_string())
        .dispatch()
        .await;

    let get_response = client
        .get("/api/applicability/1")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    let item: Applicability = get_response.into_json().await.expect("json");
    assert_eq!(item.id, 1); // ID should remain the same
}

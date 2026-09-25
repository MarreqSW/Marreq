// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

//! Comprehensive project scoping and filtering tests for API endpoints.
//!
//! These tests verify:
//! - Requirements are filtered by project
//! - Tests are filtered by project
//! - Categories are filtered by project
//! - Cross-project access is prevented
//! - Project membership requirements

use marreq_core::models::*;
use marreq_core::status_enums::ProjectStatus;
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

mod test_support {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use marreq_core::app::AppState;
    use marreq_core::auth::session::test_session_cookie_for;
    use marreq_core::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
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
        marreq_core::deployment::install_test_server_mode();
        let rocket = rocket::build()
            .manage(managed_state(repo))
            .manage(marreq_core::auth::AuthConfig::default())
            .manage(marreq_core::auth::rate_limiter::LoginRateLimiter::new())
            .mount("/api", marreq_core::api::routes());

        Client::tracked(rocket).await.expect("rocket instance")
    }

    pub fn session_cookie(client: &Client, user_id: i32) -> Cookie<'static> {
        let state = client
            .rocket()
            .state::<TestAppState>()
            .expect("managed app state");
        test_session_cookie_for(state, user_id)
    }

    pub fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        let mut admin = DieselRepoMock::make_user(1, "admin", "password");
        admin.is_admin = true;
        repo.users.insert(1, admin);

        let user1 = DieselRepoMock::make_user(2, "user1", "password");
        repo.users.insert(2, user1);

        let user2 = DieselRepoMock::make_user(3, "user2", "password");
        repo.users.insert(3, user2);

        // Project 1 - owned by user 1
        repo.projects.insert(
            1,
            Project {
                id: 1,
                name: "Project 1".into(),
                description: Some("Description 1".into()),
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(1),
                slug: "project-1".into(),
                group_id: None,
            },
        );

        // Project 2 - owned by user 2
        repo.projects.insert(
            2,
            Project {
                id: 2,
                name: "Project 2".into(),
                description: Some("Description 2".into()),
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(2),
                slug: "project-2".into(),
                group_id: None,
            },
        );

        // Add project memberships
        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 1,
            role: 1, // Owner
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 2,
            role: 2, // Member
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo.project_members.push(ProjectMember {
            project_id: 2,
            user_id: 2,
            role: 1, // Owner
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
                is_system: false,
                tag_color: None,
            },
        );

        repo.categories.insert(
            1,
            Category {
                id: 1,
                title: "Category 1".into(),
                description: "".into(),
                tag: "CAT1".into(),
                project_id: 1,
            },
        );

        repo.categories.insert(
            2,
            Category {
                id: 2,
                title: "Category 2".into(),
                description: "".into(),
                tag: "CAT2".into(),
                project_id: 2,
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

        repo.applicability.insert(
            2,
            Applicability {
                id: 2,
                title: "All".into(),
                description: "".into(),
                tag: "ALL".into(),
                project_id: 2,
            },
        );

        repo.verification_methods.insert(
            1,
            VerificationMethod {
                id: 1,
                title: "Analysis".into(),
                description: "".into(),
                tag: "ANALYSIS".into(),
                project_id: 1,
            },
        );

        repo.verification_methods.insert(
            2,
            VerificationMethod {
                id: 2,
                title: "Analysis".into(),
                description: "".into(),
                tag: "ANALYSIS".into(),
                project_id: 2,
            },
        );

        // Project 3 — no memberships for user 2 / user 3 (inaccessible fixture).
        repo.projects.insert(
            3,
            Project {
                id: 3,
                name: "Project 3".into(),
                description: Some("Forbidden project".into()),
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(1),
                slug: "project-3".into(),
                group_id: None,
            },
        );
        repo.project_members.push(ProjectMember {
            project_id: 3,
            user_id: 1,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo.categories.insert(
            3,
            Category {
                id: 3,
                title: "Category 3".into(),
                description: "".into(),
                tag: "CAT3".into(),
                project_id: 3,
            },
        );
        repo.applicability.insert(
            3,
            Applicability {
                id: 3,
                title: "All".into(),
                description: "".into(),
                tag: "ALL".into(),
                project_id: 3,
            },
        );
        repo.requirement_statuses.insert(
            3,
            RequirementStatus {
                id: 3,
                title: "Draft".into(),
                description: "".into(),
                tag: "D".into(),
                project_id: 3,
                is_system: false,
                tag_color: None,
            },
        );
        repo.verification_statuses.insert(
            3,
            VerificationStatus {
                id: 3,
                title: "Not run".into(),
                description: "".into(),
                tag: "NR".into(),
                project_id: 3,
                is_system: false,
                tag_color: None,
            },
        );

        repo
    }

    pub fn requirement(id: i32, project_id: i32) -> Requirement {
        Requirement {
            id,
            current_version_id: None,
            same_as_current: None,
            title: format!("Requirement {id}"),
            description: "Description".into(),
            reference_code: format!("REQ-{id:03}"),
            category_id: 1,
            applicability_id: 1,
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            justification: None,
            project_id,
            approval_state: "draft".into(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        }
    }

    pub fn verification(id: i32, project_id: i32) -> Verification {
        Verification {
            id,
            name: format!("Verification {id}"),
            description: "Description".into(),
            reference_code: format!("VER-{id:03}"),
            source: "manual".into(),
            status_id: 1,
            parent_id: None,
            project_id,
            verification_method_id: None,
            author_id: 1,
            reviewer_id: 1,
            status_set_by: None,
            status_set_at: None,
        }
    }
}

use test_support::*;

#[rocket::async_test]
async fn scoped_api_token_is_denied_after_membership_removal() {
    use marreq_core::permissions::ROLE_AUTHOR;
    use marreq_core::repository::ProjectMembersRepository;

    let token = "revoked-membership-token";
    let token_hash = format!("{:x}", Sha256::digest(token.as_bytes()));
    let mut repo = base_repo();
    // User 3 starts as a member of project 1 so the scoped token is initially valid.
    repo.project_members.push(ProjectMember {
        project_id: 1,
        user_id: 3,
        role: ROLE_AUTHOR,
        created_at: timestamp(),
        updated_at: timestamp(),
    });
    let repo = repo.with_api_token(&token_hash, 3, Some(1));
    let client = test_client(repo).await;
    let bearer = || rocket::http::Header::new("Authorization", format!("Bearer {token}"));

    let allowed = client
        .get("/api/projects/1/requirements")
        .header(bearer())
        .dispatch()
        .await;
    assert_eq!(allowed.status(), Status::Ok);

    // Warm the membership cache, then remove via the repository path so
    // CacheRepository invalidation is required for the subsequent denial.
    {
        let state = client
            .rocket()
            .state::<TestAppState>()
            .expect("managed app state");
        let mut write = state.repo.write().expect("repo lock");
        let _ = write.get_projects_for_user(3).expect("membership");
        write
            .remove_project_member(1, 3)
            .expect("remove membership");
    }

    let denied = client
        .get("/api/projects/1/requirements")
        .header(bearer())
        .dispatch()
        .await;
    assert_eq!(denied.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn legacy_requirement_subresources_use_the_requirement_project() {
    let mut repo = base_repo();
    repo.requirements.insert(1, requirement(1, 1));
    let client = test_client(repo).await;
    let cookie = || session_cookie(&client, 3);

    for path in [
        "/api/requirements/1/versions",
        "/api/requirements/1/impacted_tests",
        "/api/requirements/1/comments",
    ] {
        let response = client.get(path).private_cookie(cookie()).dispatch().await;
        assert_eq!(response.status(), Status::Forbidden, "path: {path}");
    }
}

#[rocket::async_test]
async fn legacy_verification_delete_uses_the_verification_project() {
    let mut repo = base_repo();
    repo.verifications.insert(1, verification(1, 1));
    let client = test_client(repo).await;

    let response = client
        .delete("/api/verifications/1")
        .private_cookie(session_cookie(&client, 3))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Forbidden);
}

// ============================================================================
// Requirements API - Project Scoping Tests
// ============================================================================

#[rocket::async_test]
async fn list_requirements_returns_only_user_projects() {
    let mut repo = base_repo();

    // Requirement in project 1 (user 2 is member)
    repo.requirements.insert(
        1,
        Requirement {
            id: 1,
            current_version_id: None,
            same_as_current: None,
            title: "Req 1".into(),
            description: "Description".into(),
            reference_code: "REQ-001".into(),
            category_id: 1,
            applicability_id: 1,
            status_id: 1,
            author_id: 2,
            reviewer_id: 2,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            justification: None,
            project_id: 1,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );

    // Requirement in project 2 (user 2 is owner)
    repo.requirements.insert(
        2,
        Requirement {
            id: 2,
            current_version_id: None,
            same_as_current: None,
            title: "Req 2".into(),
            description: "Description".into(),
            reference_code: "REQ-002".into(),
            category_id: 2,
            applicability_id: 2,
            status_id: 1,
            author_id: 2,
            reviewer_id: 2,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            justification: None,
            project_id: 2,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );

    // Requirement in project 1 but user 3 is not a member
    repo.requirements.insert(
        3,
        Requirement {
            id: 3,
            current_version_id: None,
            same_as_current: None,
            title: "Req 3".into(),
            description: "Description".into(),
            reference_code: "REQ-003".into(),
            category_id: 1,
            applicability_id: 1,
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            justification: None,
            project_id: 1,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );

    // Requirement in project 3 — user 2 has no access
    repo.requirements.insert(
        4,
        Requirement {
            id: 4,
            current_version_id: None,
            same_as_current: None,
            title: "Req Forbidden".into(),
            description: "Description".into(),
            reference_code: "REQ-004".into(),
            category_id: 3,
            applicability_id: 3,
            status_id: 3,
            author_id: 1,
            reviewer_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            justification: None,
            project_id: 3,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );

    let client = test_client(repo).await;

    // User 2 should see requirements from projects 1 and 2
    let response = client
        .get("/api/requirements")
        .private_cookie(session_cookie(&client, 2))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let requirements: Vec<Value> = response.into_json().await.expect("json");

    let req_ids: Vec<i32> = requirements
        .iter()
        .map(|r| r["id"].as_i64().unwrap() as i32)
        .collect();

    assert!(req_ids.contains(&1));
    assert!(req_ids.contains(&2));
    assert!(req_ids.contains(&3));
    assert!(!req_ids.contains(&4));
}

#[rocket::async_test]
async fn get_requirement_from_unauthorized_project_returns_forbidden() {
    let mut repo = base_repo();

    // Requirement in project 1
    repo.requirements.insert(
        1,
        Requirement {
            id: 1,
            current_version_id: None,
            same_as_current: None,
            title: "Req 1".into(),
            description: "Description".into(),
            reference_code: "REQ-001".into(),
            category_id: 1,
            applicability_id: 1,
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            justification: None,
            project_id: 1,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );

    let client = test_client(repo).await;

    // User 3 is not a member of project 1
    let response = client
        .get("/api/requirements/1")
        .private_cookie(session_cookie(&client, 3))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn create_requirement_forbidden_when_user_not_project_member() {
    let client = test_client(base_repo()).await;

    // User 3 is not a member of project 1 — create requires EditRequirements in that project.
    let payload = json!({
        "title": "New Requirement",
        "description": "Description",
        "reference_code": "REQ-999",
        "category_id": 1,
        "applicability_id": 1,
        "status_id": 1,
        "verification_method_ids": [1],
        "author_id": 3,
        "reviewer_id": 3,
        "parent_id": null,
        "project_id": 1
    });

    let response = client
        .post("/api/requirements")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 3))
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn delete_requirement_from_unauthorized_project_is_forbidden() {
    let mut repo = base_repo();

    repo.requirements.insert(
        1,
        Requirement {
            id: 1,
            current_version_id: None,
            same_as_current: None,
            title: "Req 1".into(),
            description: "Description".into(),
            reference_code: "REQ-001".into(),
            category_id: 1,
            applicability_id: 1,
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            justification: None,
            project_id: 1,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );

    let client = test_client(repo).await;

    let response = client
        .delete("/api/requirements/1")
        .private_cookie(session_cookie(&client, 3))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Forbidden);
}

// ============================================================================
// Tests API - Project Scoping Tests
// ============================================================================

#[rocket::async_test]
async fn list_tests_returns_only_user_projects() {
    let mut repo = base_repo();

    repo.verifications.insert(
        1,
        Verification {
            id: 1,
            name: "Test 1".into(),
            description: "Description".into(),
            reference_code: "TEST-001".into(),
            source: "manual".into(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
            verification_method_id: None,
            author_id: 1,
            reviewer_id: 1,
            status_set_by: None,
            status_set_at: None,
        },
    );

    repo.verifications.insert(
        2,
        Verification {
            id: 2,
            name: "Test 2".into(),
            description: "Description".into(),
            reference_code: "TEST-002".into(),
            source: "manual".into(),
            status_id: 1,
            parent_id: None,
            project_id: 2,
            verification_method_id: None,
            author_id: 1,
            reviewer_id: 1,
            status_set_by: None,
            status_set_at: None,
        },
    );

    repo.verifications.insert(
        3,
        Verification {
            id: 3,
            name: "Test 3".into(),
            description: "Description".into(),
            reference_code: "TEST-003".into(),
            source: "manual".into(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
            verification_method_id: None,
            author_id: 1,
            reviewer_id: 1,
            status_set_by: None,
            status_set_at: None,
        },
    );

    repo.verifications.insert(
        4,
        Verification {
            id: 4,
            name: "Test Forbidden".into(),
            description: "Description".into(),
            reference_code: "TEST-004".into(),
            source: "manual".into(),
            status_id: 3,
            parent_id: None,
            project_id: 3,
            verification_method_id: None,
            author_id: 1,
            reviewer_id: 1,
            status_set_by: None,
            status_set_at: None,
        },
    );

    let client = test_client(repo).await;

    // User 2 should see tests from projects 1 and 2
    let response = client
        .get("/api/verifications")
        .private_cookie(session_cookie(&client, 2))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let tests: Vec<Value> = response.into_json().await.expect("json");

    let test_ids: Vec<i32> = tests
        .iter()
        .map(|t| t["id"].as_i64().unwrap() as i32)
        .collect();

    assert!(test_ids.contains(&1));
    assert!(test_ids.contains(&2));
    assert!(test_ids.contains(&3));
    assert!(!test_ids.contains(&4));
}

#[rocket::async_test]
async fn get_test_from_unauthorized_project_is_forbidden() {
    let mut repo = base_repo();

    repo.verifications.insert(
        1,
        Verification {
            id: 1,
            name: "Test 1".into(),
            description: "Description".into(),
            reference_code: "TEST-001".into(),
            source: "manual".into(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
            verification_method_id: None,
            author_id: 1,
            reviewer_id: 1,
            status_set_by: None,
            status_set_at: None,
        },
    );

    let client = test_client(repo).await;

    let response = client
        .get("/api/verifications/1")
        .private_cookie(session_cookie(&client, 3))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Forbidden);
}

// ============================================================================
// Categories API - Project Scoping Tests
// ============================================================================

#[rocket::async_test]
async fn list_categories_returns_only_user_projects() {
    let repo = base_repo(); // Already has categories for projects 1 and 2
    let client = test_client(repo).await;

    // User 2 should see categories from projects 1 and 2
    let response = client
        .get("/api/categories")
        .private_cookie(session_cookie(&client, 2))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let categories: Vec<Value> = response.into_json().await.expect("json");

    let cat_ids: Vec<i32> = categories
        .iter()
        .map(|c| c["id"].as_i64().unwrap() as i32)
        .collect();

    assert!(cat_ids.contains(&1));
    assert!(cat_ids.contains(&2));
    assert!(!cat_ids.contains(&3));
}

#[rocket::async_test]
async fn get_category_from_unauthorized_project_is_forbidden() {
    let repo = base_repo();
    let client = test_client(repo).await;

    let response = client
        .get("/api/categories/1")
        .private_cookie(session_cookie(&client, 3))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn create_category_in_unauthorized_project_is_forbidden() {
    let client = test_client(base_repo()).await;

    let payload = json!({
        "title": "New Category",
        "description": "Description",
        "tag": "NEW",
        "project_id": 1
    });

    let response = client
        .post("/api/categories")
        .header(ContentType::JSON)
        .private_cookie(session_cookie(&client, 3))
        .body(payload.to_string())
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Forbidden);
}

// ============================================================================
// Applicability API - Project Scoping Tests
// ============================================================================

#[rocket::async_test]
async fn list_applicability_returns_only_user_projects() {
    let repo = base_repo(); // Already has applicability for projects 1 and 2
    let client = test_client(repo).await;

    // User 2 should see applicability from projects 1 and 2
    let response = client
        .get("/api/applicability")
        .private_cookie(session_cookie(&client, 2))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let items: Vec<Value> = response.into_json().await.expect("json");

    let app_ids: Vec<i32> = items
        .iter()
        .map(|a| a["id"].as_i64().unwrap() as i32)
        .collect();

    assert!(app_ids.contains(&1));
    assert!(app_ids.contains(&2));
    assert!(!app_ids.contains(&3));
}

#[rocket::async_test]
async fn get_applicability_from_unauthorized_project_is_forbidden() {
    let repo = base_repo();
    let client = test_client(repo).await;

    let response = client
        .get("/api/applicability/1")
        .private_cookie(session_cookie(&client, 3))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Forbidden);
}

// ============================================================================
// Status / Matrix global collection filtering
// ============================================================================

#[rocket::async_test]
async fn list_requirement_statuses_returns_only_user_projects() {
    let mut repo = base_repo();
    repo.requirement_statuses.insert(
        2,
        RequirementStatus {
            id: 2,
            title: "Draft".into(),
            description: "".into(),
            tag: "D".into(),
            project_id: 2,
            is_system: false,
            tag_color: None,
        },
    );
    let client = test_client(repo).await;

    let response = client
        .get("/api/status")
        .private_cookie(session_cookie(&client, 2))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let statuses: Vec<Value> = response.into_json().await.expect("json");
    let ids: Vec<i32> = statuses
        .iter()
        .map(|s| s["id"].as_i64().unwrap() as i32)
        .collect();

    assert!(ids.contains(&1));
    assert!(ids.contains(&2));
    assert!(!ids.contains(&3));
}

#[rocket::async_test]
async fn list_verification_statuses_returns_only_user_projects() {
    let mut repo = base_repo();
    repo.verification_statuses.insert(
        1,
        VerificationStatus {
            id: 1,
            title: "Not run".into(),
            description: "".into(),
            tag: "NR".into(),
            project_id: 1,
            is_system: false,
            tag_color: None,
        },
    );
    repo.verification_statuses.insert(
        2,
        VerificationStatus {
            id: 2,
            title: "Not run".into(),
            description: "".into(),
            tag: "NR".into(),
            project_id: 2,
            is_system: false,
            tag_color: None,
        },
    );
    let client = test_client(repo).await;

    let response = client
        .get("/api/verification-status")
        .private_cookie(session_cookie(&client, 2))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let statuses: Vec<Value> = response.into_json().await.expect("json");
    let ids: Vec<i32> = statuses
        .iter()
        .map(|s| s["id"].as_i64().unwrap() as i32)
        .collect();

    assert!(ids.contains(&1));
    assert!(ids.contains(&2));
    assert!(!ids.contains(&3));
}

#[rocket::async_test]
async fn list_matrix_returns_only_user_projects() {
    let mut repo = base_repo();
    repo.requirements.insert(1, requirement(1, 1));
    repo.requirements.insert(2, requirement(2, 3));
    repo.verifications.insert(1, verification(1, 1));
    repo.verifications.insert(2, verification(2, 3));
    repo.matrices.push(MatrixLink {
        req_id: 1,
        verification_id: 1,
        creation_date: timestamp(),
        project_id: 1,
        suspect: false,
        suspect_at: None,
        suspect_reason: None,
        cleared_by: None,
        cleared_at: None,
        triggering_version_id: None,
        triggering_user_id: None,
    });
    repo.matrices.push(MatrixLink {
        req_id: 2,
        verification_id: 2,
        creation_date: timestamp(),
        project_id: 3,
        suspect: false,
        suspect_at: None,
        suspect_reason: None,
        cleared_by: None,
        cleared_at: None,
        triggering_version_id: None,
        triggering_user_id: None,
    });
    let client = test_client(repo).await;

    let response = client
        .get("/api/matrix")
        .private_cookie(session_cookie(&client, 2))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let entries: Vec<Value> = response.into_json().await.expect("json");
    let project_ids: Vec<i32> = entries
        .iter()
        .map(|e| e["project_id"].as_i64().unwrap() as i32)
        .collect();

    assert!(project_ids.contains(&1));
    assert!(!project_ids.contains(&3));
}

// ============================================================================
// Admin Override Tests
// ============================================================================

#[rocket::async_test]
async fn admin_can_access_all_projects() {
    let mut repo = base_repo();

    repo.requirements.insert(
        1,
        Requirement {
            id: 1,
            current_version_id: None,
            same_as_current: None,
            title: "Req 1".into(),
            description: "Description".into(),
            reference_code: "REQ-001".into(),
            category_id: 1,
            applicability_id: 1,
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            justification: None,
            project_id: 1,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        },
    );

    let client = test_client(repo).await;

    // Admin (user 1) should be able to access requirement from project 1
    let response = client
        .get("/api/requirements/1")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

use marreq_core::models::*;
use marreq_core::permissions::ROLE_VIEWER;
use marreq_core::status_enums::ProjectStatus;
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;
use serde_json::Value;

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

    pub fn bundle_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        let mut admin = DieselRepoMock::make_user(1, "admin", "password");
        admin.is_admin = true;
        repo.users.insert(1, admin);
        repo.users
            .insert(2, DieselRepoMock::make_user(2, "viewer", "password"));
        repo.users
            .insert(3, DieselRepoMock::make_user(3, "outsider", "password"));

        repo.projects.insert(
            1,
            Project {
                id: 1,
                name: "Satellite Demo".into(),
                description: Some("Demo project".into()),
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(1),
                slug: "satellite-demo".into(),
                group_id: None,
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
            role: ROLE_VIEWER,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                id: 1,
                title: "Draft".into(),
                description: "Draft".into(),
                tag: "Drf".into(),
                project_id: 1,
                is_system: true,
                tag_color: None,
            },
        );
        repo.categories.insert(
            1,
            Category {
                id: 1,
                title: "Power".into(),
                description: "Power".into(),
                tag: "PWR".into(),
                project_id: 1,
            },
        );
        repo.applicability.insert(
            1,
            Applicability {
                id: 1,
                title: "All".into(),
                description: "All".into(),
                tag: "ALL".into(),
                project_id: 1,
            },
        );
        repo.verification_methods.insert(
            1,
            VerificationMethod {
                id: 1,
                title: "Test".into(),
                description: "Test".into(),
                tag: "TEST".into(),
                project_id: 1,
            },
        );
        repo.verification_statuses.insert(
            1,
            VerificationStatus {
                id: 1,
                title: "Pending".into(),
                description: "Pending".into(),
                tag: "Pend".into(),
                project_id: 1,
                is_system: true,
                tag_color: None,
            },
        );

        repo.requirements.insert(
            1,
            Requirement {
                id: 1,
                current_version_id: Some(10),
                same_as_current: None,
                title: "Bus voltage".into(),
                description: "The bus shall supply 28V.".into(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "SAT-PWR-001".into(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: None,
                applicability_id: 1,
                justification: Some("Power budget".into()),
                project_id: 1,
                approval_state: "draft".into(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            },
        );
        repo.requirement_versions.insert(
            10,
            RequirementVersion {
                id: 10,
                requirement_id: 1,
                title: "Bus voltage".into(),
                description: "The bus shall supply 28V.".into(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                category_id: 1,
                applicability_id: 1,
                justification: Some("Power budget".into()),
                deadline_date: None,
                created_at: timestamp(),
                approval_state: "draft".into(),
                approved_by: None,
                approved_at: None,
                reviewed_by: None,
                reviewed_at: None,
            },
        );

        repo.verifications.insert(
            1,
            Verification {
                id: 1,
                name: "Measure bus voltage".into(),
                reference_code: "VER-PWR-001".into(),
                description: "Measure 28V on the bus.".into(),
                source: "lab".into(),
                status_id: 1,
                parent_id: None,
                project_id: 1,
                verification_method_id: Some(1),
                author_id: 1,
                reviewer_id: 1,
                status_set_by: None,
                status_set_at: None,
            },
        );

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

        repo.requirement_comments.push(RequirementComment {
            id: 1,
            requirement_id: 1,
            requirement_version_id: Some(10),
            author_id: 1,
            body: "Need a clarification.".into(),
            created_at: timestamp(),
        });

        repo.next_comment_id = 2;
        repo.next_version_id = 11;

        repo
    }

    pub fn json_file_body(filename: &str, json: &str) -> (ContentType, String) {
        let boundary = "----MarreqBundleBoundary";
        let ct = ContentType::new("multipart", "form-data").with_params(("boundary", boundary));
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/json\r\n\r\n{json}\r\n--{boundary}--\r\n"
        );
        (ct, body)
    }
}

use test_support::*;

#[rocket::async_test]
async fn export_bundle_requires_auth() {
    let client = test_client(bundle_repo()).await;
    let response = client
        .get("/api/projects/1/exports/bundle.json")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn viewer_can_export_bundle() {
    let client = test_client(bundle_repo()).await;
    let response = client
        .get("/api/projects/1/exports/bundle.json")
        .private_cookie(session_cookie(&client, 2))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Content-Type"),
        Some("application/json")
    );
    let body: Value = response.into_json().await.expect("json");
    assert_eq!(body["format"], "marreq.project-bundle.v1");
    assert_eq!(body["project"]["name"], "Satellite Demo");
    assert_eq!(body["requirements"][0]["reference_code"], "SAT-PWR-001");
    assert_eq!(body["verifications"][0]["reference_code"], "VER-PWR-001");
    assert_eq!(
        body["matrix"][0]["requirement_reference_code"],
        "SAT-PWR-001"
    );
    assert_eq!(body["comments"][0]["body"], "Need a clarification.");
}

#[rocket::async_test]
async fn import_bundle_rejects_unknown_format() {
    let client = test_client(bundle_repo()).await;
    let (ct, body) = json_file_body("bad.json", r#"{"format":"nope","project":{"name":"X"}}"#);
    let response = client
        .post("/api/projects/imports/bundle")
        .header(ct)
        .private_cookie(session_cookie(&client, 1))
        .body(body)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn import_bundle_round_trip_creates_new_project() {
    let client = test_client(bundle_repo()).await;
    let exported = client
        .get("/api/projects/1/exports/bundle.json")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;
    assert_eq!(exported.status(), Status::Ok);
    let json = exported.into_string().await.expect("bundle body");
    let (ct, body) = json_file_body("project-bundle.json", &json);
    let imported = client
        .post("/api/projects/imports/bundle")
        .header(ct)
        .private_cookie(session_cookie(&client, 1))
        .body(body)
        .dispatch()
        .await;
    assert_eq!(imported.status(), Status::Ok);
    let result: Value = imported.into_json().await.expect("import json");
    let new_id = result["project_id"].as_i64().expect("project_id") as i32;
    assert_ne!(new_id, 1);
    assert!(result["imported_counts"]["requirements"].as_u64().unwrap() >= 1);
    assert!(result["imported_counts"]["verifications"].as_u64().unwrap() >= 1);
    assert!(result["imported_counts"]["matrix_links"].as_u64().unwrap() >= 1);
    assert!(result["imported_counts"]["comments"].as_u64().unwrap() >= 1);

    let listed = client
        .get(format!("/api/projects/{new_id}/requirements"))
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;
    assert_eq!(listed.status(), Status::Ok);
    let reqs: Value = listed.into_json().await.expect("reqs");
    let codes: Vec<&str> = reqs
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["reference_code"].as_str().unwrap_or(""))
        .collect();
    assert!(codes.contains(&"SAT-PWR-001"));
}

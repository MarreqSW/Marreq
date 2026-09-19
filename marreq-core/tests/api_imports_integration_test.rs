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

    pub fn catalog_repo() -> DieselRepoMock {
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
                name: "Test Project".into(),
                description: Some("Description".into()),
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(1),
                slug: "test-project".into(),
                group_id: None,
            },
        );

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
                tag: "draft".into(),
                project_id: 1,
                is_system: true,
                tag_color: None,
            },
        );
        repo.categories.insert(
            1,
            Category {
                id: 1,
                title: "General".into(),
                description: "General".into(),
                tag: "gen".into(),
                project_id: 1,
            },
        );
        repo.applicability.insert(
            1,
            Applicability {
                id: 1,
                title: "All".into(),
                description: "All".into(),
                tag: "all".into(),
                project_id: 1,
            },
        );
        repo.verification_methods.insert(
            1,
            VerificationMethod {
                id: 1,
                title: "Test".into(),
                description: "Test".into(),
                tag: "test".into(),
                project_id: 1,
            },
        );
        repo
    }

    pub fn csv_preview_body(csv: &str) -> (ContentType, String) {
        let boundary = "----MarreqImportBoundary";
        let ct = ContentType::new("multipart", "form-data").with_params(("boundary", boundary));
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"reqs.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n--{boundary}--\r\n"
        );
        (ct, body)
    }

    pub fn csv_commit_body(
        csv: &str,
        import_type: &str,
        mappings: &str,
        value_mappings: &str,
    ) -> (ContentType, String) {
        let boundary = "----MarreqImportBoundary";
        let ct = ContentType::new("multipart", "form-data").with_params(("boundary", boundary));
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"reqs.csv\"\r\nContent-Type: text/csv\r\n\r\n{csv}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"import_type\"\r\n\r\n{import_type}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"column_mappings\"\r\n\r\n{mappings}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"value_mappings\"\r\n\r\n{value_mappings}\r\n--{boundary}--\r\n"
        );
        (ct, body)
    }
}

use test_support::*;

const SAMPLE_CSV: &str = "Title,Description,Req ID\nAlpha requirement,Imported row,REQ-001\n";

#[rocket::async_test]
async fn preview_excel_requires_auth() {
    let client = test_client(catalog_repo()).await;
    let (ct, body) = csv_preview_body(SAMPLE_CSV);
    let response = client
        .post("/api/projects/1/imports/excel/preview")
        .header(ct)
        .body(body)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn preview_excel_forbids_viewer() {
    let client = test_client(catalog_repo()).await;
    let (ct, body) = csv_preview_body(SAMPLE_CSV);
    let response = client
        .post("/api/projects/1/imports/excel/preview")
        .private_cookie(session_cookie(&client, 2))
        .header(ct)
        .body(body)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn preview_excel_returns_columns() {
    let client = test_client(catalog_repo()).await;
    let (ct, body) = csv_preview_body(SAMPLE_CSV);
    let response = client
        .post("/api/projects/1/imports/excel/preview")
        .private_cookie(session_cookie(&client, 1))
        .header(ct)
        .body(body)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let json: Value = response.into_json().await.expect("json");
    assert_eq!(json["import_type"], "requirements");
    assert_eq!(json["row_count"], 1);
    assert_eq!(json["columns"][0]["name"], "Title");
    assert_eq!(json["unique_values"]["Title"][0], "Alpha requirement");
    assert!(json["available_fields"]["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "title"));
}

#[rocket::async_test]
async fn commit_excel_creates_requirement() {
    let client = test_client(catalog_repo()).await;
    let mappings = r#"[{"excel_column":"Title","target_field":"title"},{"excel_column":"Description","target_field":"description"},{"excel_column":"Req ID","target_field":"reference_code"}]"#;
    let (ct, body) = csv_commit_body(SAMPLE_CSV, "requirements", mappings, "[]");
    let response = client
        .post("/api/projects/1/imports/excel")
        .private_cookie(session_cookie(&client, 1))
        .header(ct)
        .body(body)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let json: Value = response.into_json().await.expect("json");
    assert_eq!(json["imported_count"], 1);
    assert_eq!(json["success"], true);

    let listed = client
        .get("/api/projects/1/requirements")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;
    assert_eq!(listed.status(), Status::Ok);
    let reqs: Vec<Requirement> = listed.into_json().await.expect("reqs");
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].title, "Alpha requirement");
}

#[rocket::async_test]
async fn commit_excel_rejects_bad_import_type() {
    let client = test_client(catalog_repo()).await;
    let mappings = r#"[{"excel_column":"Title","target_field":"title"}]"#;
    let (ct, body) = csv_commit_body(SAMPLE_CSV, "widgets", mappings, "[]");
    let response = client
        .post("/api/projects/1/imports/excel")
        .private_cookie(session_cookie(&client, 1))
        .header(ct)
        .body(body)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn commit_excel_requires_title_mapping() {
    let client = test_client(catalog_repo()).await;
    let mappings = r#"[{"excel_column":"Title","target_field":"description"}]"#;
    let (ct, body) = csv_commit_body(SAMPLE_CSV, "requirements", mappings, "[]");
    let response = client
        .post("/api/projects/1/imports/excel")
        .private_cookie(session_cookie(&client, 1))
        .header(ct)
        .body(body)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn commit_excel_defaults_unknown_catalog_value() {
    let client = test_client(catalog_repo()).await;
    let csv = "Title,Description,Category\nAlpha requirement,Imported row,Unknown category\n";
    let mappings = r#"[{"excel_column":"Title","target_field":"title"},{"excel_column":"Description","target_field":"description"},{"excel_column":"Category","target_field":"category_id"}]"#;
    let (ct, body) = csv_commit_body(csv, "requirements", mappings, "[]");
    let response = client
        .post("/api/projects/1/imports/excel")
        .private_cookie(session_cookie(&client, 1))
        .header(ct)
        .body(body)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let json: Value = response.into_json().await.expect("json");
    assert_eq!(json["success"], true);
    assert_eq!(json["imported_count"], 1);
}

#[rocket::async_test]
async fn commit_excel_rejects_cross_project_value_mapping() {
    let client = test_client(catalog_repo()).await;
    let mappings = r#"[{"excel_column":"Title","target_field":"title"}]"#;
    let value_mappings =
        r#"[{"target_field":"category_id","source_value":"Unknown","target_id":999}]"#;
    let (ct, body) = csv_commit_body(SAMPLE_CSV, "requirements", mappings, value_mappings);
    let response = client
        .post("/api/projects/1/imports/excel")
        .private_cookie(session_cookie(&client, 1))
        .header(ct)
        .body(body)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::BadRequest);
}

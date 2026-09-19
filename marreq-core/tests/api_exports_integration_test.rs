// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![cfg(feature = "test-helpers")]

use marreq_core::models::*;
use marreq_core::permissions::ROLE_VIEWER;
use marreq_core::status_enums::ProjectStatus;
use rocket::http::{Cookie, Status};
use rocket::local::asynchronous::Client;

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

    pub fn export_repo() -> DieselRepoMock {
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
        repo.verification_statuses.insert(
            1,
            VerificationStatus {
                id: 1,
                title: "Planned".into(),
                description: "Planned".into(),
                tag: "plan".into(),
                project_id: 1,
                is_system: true,
                tag_color: None,
            },
        );

        repo.requirements.insert(
            1,
            Requirement {
                id: 1,
                current_version_id: None,
                same_as_current: None,
                title: "Alpha requirement".into(),
                description: "Alpha description".into(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 1,
                reference_code: "REQ-001".into(),
                category_id: 1,
                parent_id: None,
                creation_date: timestamp(),
                update_date: timestamp(),
                deadline_date: Some(timestamp()),
                applicability_id: 1,
                justification: Some("Because".into()),
                project_id: 1,
                approval_state: "draft".into(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            },
        );

        repo.verifications.insert(
            1,
            Verification {
                id: 1,
                name: "Alpha verification".into(),
                reference_code: "TST-001".into(),
                description: "Alpha verification description".into(),
                source: "automated".into(),
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

        repo.baselines.push(Baseline {
            id: 10,
            project_id: 1,
            name: "BL-1".into(),
            description: Some("First snapshot".into()),
            created_at: timestamp(),
            created_by: 1,
            source_saved_view_id: None,
            source_view_definition: None,
        });

        repo
    }
}

use test_support::*;

const XLSX_CONTENT_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

/// xlsx files are zip archives, so a valid workbook starts with the local file header magic.
const ZIP_MAGIC: &[u8] = b"PK\x03\x04";

const PDF_MAGIC: &[u8] = b"%PDF-";

#[rocket::async_test]
async fn export_requirements_requires_auth() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/requirements.xlsx")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn export_requirements_forbids_non_member() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/requirements.xlsx")
        .private_cookie(session_cookie(&client, 3))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn export_requirements_returns_workbook() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/requirements.xlsx")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Content-Type"),
        Some(XLSX_CONTENT_TYPE)
    );
    assert_eq!(
        response.headers().get_one("Content-Disposition"),
        Some("attachment; filename=\"requirements-project-1.xlsx\"")
    );

    let bytes = response.into_bytes().await.expect("body");
    assert!(bytes.starts_with(ZIP_MAGIC), "expected an xlsx archive");
}

#[rocket::async_test]
async fn export_requirements_allows_viewer() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/requirements.xlsx")
        .private_cookie(session_cookie(&client, 2))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn export_verifications_returns_workbook() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/verifications.xlsx")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Content-Disposition"),
        Some("attachment; filename=\"verifications-project-1.xlsx\"")
    );

    let bytes = response.into_bytes().await.expect("body");
    assert!(bytes.starts_with(ZIP_MAGIC), "expected an xlsx archive");
}

#[rocket::async_test]
async fn export_verifications_forbids_non_member() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/verifications.xlsx")
        .private_cookie(session_cookie(&client, 3))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn export_matrix_returns_workbook() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/matrix.xlsx")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Content-Type"),
        Some(XLSX_CONTENT_TYPE)
    );
    assert_eq!(
        response.headers().get_one("Content-Disposition"),
        Some("attachment; filename=\"matrix-project-1.xlsx\"")
    );

    let bytes = response.into_bytes().await.expect("body");
    assert!(bytes.starts_with(ZIP_MAGIC), "expected an xlsx archive");
}

#[rocket::async_test]
async fn export_matrix_forbids_non_member() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/matrix.xlsx")
        .private_cookie(session_cookie(&client, 3))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn export_requirements_pdf_returns_document() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/requirements.pdf")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Content-Type"),
        Some("application/pdf")
    );
    assert_eq!(
        response.headers().get_one("Content-Disposition"),
        Some("attachment; filename=\"requirements-project-1.pdf\"")
    );

    let bytes = response.into_bytes().await.expect("body");
    assert!(bytes.starts_with(PDF_MAGIC), "expected a pdf document");
}

#[rocket::async_test]
async fn export_report_pdf_returns_document() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/report.pdf")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Content-Disposition"),
        Some("attachment; filename=\"report-project-1.pdf\"")
    );

    let bytes = response.into_bytes().await.expect("body");
    assert!(bytes.starts_with(PDF_MAGIC), "expected a pdf document");
}

#[rocket::async_test]
async fn export_report_pdf_allows_viewer() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/report.pdf")
        .private_cookie(session_cookie(&client, 2))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn export_requirements_pdf_requires_auth() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/requirements.pdf")
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn export_requirements_reqif_returns_xml() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/requirements.reqif")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Content-Type"),
        Some("application/xml")
    );
    assert_eq!(
        response.headers().get_one("Content-Disposition"),
        Some("attachment; filename=\"requirements-project-1.reqif\"")
    );
    let body = response.into_string().await.expect("xml");
    assert!(body.contains("<REQ-IF"));
    assert!(body.contains("SPEC-OBJECT"));
}

#[rocket::async_test]
async fn export_requirements_reqif_allows_viewer() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/requirements.reqif")
        .private_cookie(session_cookie(&client, 2))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
}

#[rocket::async_test]
async fn export_requirements_reqif_forbids_non_member() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/requirements.reqif")
        .private_cookie(session_cookie(&client, 3))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn export_baseline_reqif_returns_xml() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/baselines/10.reqif")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    assert_eq!(
        response.headers().get_one("Content-Disposition"),
        Some("attachment; filename=\"baseline-10-project-1.reqif\"")
    );
    let body = response.into_string().await.expect("xml");
    assert!(body.contains("<REQ-IF"));
}

#[rocket::async_test]
async fn export_baseline_reqif_not_found_for_unknown_baseline() {
    let client = test_client(export_repo()).await;
    let response = client
        .get("/api/projects/1/exports/baselines/999.reqif")
        .private_cookie(session_cookie(&client, 1))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::NotFound);
}

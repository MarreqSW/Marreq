// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::serde::Deserialize;

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::models::MatrixLink;
use crate::repository::MatrixRepository;
use crate::repository::RequirementsRepository;
use crate::repository::VerificationsRepository;
use crate::services::MatrixService;

/// Request body for [`put_verification_matrix`]: replace all matrix rows for one verification.
#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct VerificationMatrixPutBody {
    /// Requirement ids that this verification will cover (empty = unlink all).
    pub requirement_ids: Vec<i32>,
}

#[get("/matrix")]
pub async fn list(state: &State<AppState>) -> ApiResult<Json<Vec<MatrixLink>>> {
    let service = MatrixService::new(state.inner());
    let entries = service.list_all()?;
    Ok(Json(entries))
}

/// Project-scoped matrix (traceability links). Accepts session or Bearer token.
#[get("/projects/<project_id>/matrix")]
pub async fn list_by_project(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<MatrixLink>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = MatrixService::new(state.inner());
    let entries = service.list_by_project(project_id)?;
    Ok(Json(entries))
}

/// Requirement ids currently linked to this verification (project-scoped). Session or Bearer.
#[get("/projects/<project_id>/verifications/<verification_id>/matrix")]
pub async fn get_verification_matrix(
    access: ProjectAccessOrBearer,
    project_id: i32,
    verification_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Value>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let repo = state.repo_read();
    let verification = repo.get_verification_by_id(verification_id)?;
    if verification.project_id != project_id {
        return Err(ApiError::NotFound("verification not in project".into()));
    }
    let links = repo.get_matrix_by_project(project_id)?;
    let mut requirement_ids: Vec<i32> = links
        .into_iter()
        .filter(|m| m.verification_id == verification_id)
        .map(|m| m.req_id)
        .collect();
    requirement_ids.sort_unstable();
    requirement_ids.dedup();
    Ok(Json(json!({
        "verification_id": verification_id,
        "requirement_ids": requirement_ids
    })))
}

/// Replace traceability links for one verification with the given requirements (same semantics as
/// legacy “edit test” form: delete existing matrix rows for this verification, then insert one per id).
/// Requires `EditRequirements`. Session or Bearer.
#[put(
    "/projects/<project_id>/verifications/<verification_id>/matrix",
    data = "<body>"
)]
pub async fn put_verification_matrix(
    access: ProjectAccessOrBearer,
    project_id: i32,
    verification_id: i32,
    body: Json<VerificationMatrixPutBody>,
    state: &State<AppState>,
) -> ApiResult<Json<Value>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;

    let body = body.into_inner();
    let mut requirement_ids = body.requirement_ids;
    requirement_ids.sort_unstable();
    requirement_ids.dedup();

    let repo = state.repo_read();
    let verification = repo.get_verification_by_id(verification_id)?;
    if verification.project_id != project_id {
        return Err(ApiError::NotFound("verification not in project".into()));
    }
    for &rid in &requirement_ids {
        let req = repo.get_requirement_by_id(rid)?;
        if req.project_id != project_id {
            return Err(ApiError::BadRequest(format!(
                "requirement {rid} is not in project {project_id}"
            )));
        }
    }
    drop(repo);

    state
        .repo_write()
        .update_verification_requirement_links(verification_id, &requirement_ids)?;

    Ok(Json(json!({
        "status": "ok",
        "verification_id": verification_id,
        "requirement_ids": requirement_ids
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::models::{MatrixLink, Project, ProjectMember};
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use crate::status_enums::ProjectStatus;
    use chrono::NaiveDate;
    use rocket::http::{Cookie, SameSite};
    use rocket::local::asynchronous::Client;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;
    const PROJECT_ID: i32 = 1;

    fn epoch() -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn test_state(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_routes(repo: DieselRepoMock, mount_list_by_project: bool) -> Client {
        let state = test_state(repo);
        let rocket = rocket::build().manage(state).mount(
            "/api",
            if mount_list_by_project {
                routes![list, list_by_project]
            } else {
                routes![list]
            },
        );
        Client::tracked(rocket).await.unwrap()
    }

    #[rocket::async_test]
    async fn list_returns_empty_without_data() {
        let client = client_with_routes(DieselRepoMock::default(), false).await;
        let response = client.get("/api/matrix").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.unwrap();
        assert_eq!(body, "[]");
    }

    #[rocket::async_test]
    async fn list_returns_all_links() {
        let mut repo = DieselRepoMock::default();
        repo.projects.insert(
            7,
            Project {
                id: 7,
                name: "P7".into(),
                description: None,
                creation_date: None,
                update_date: None,
                status: ProjectStatus::Active,
                owner_id: None,
                slug: "p7".into(),
                group_id: None,
            },
        );
        repo.matrices.push(MatrixLink {
            req_id: 1,
            verification_id: 2,
            creation_date: epoch(),
            project_id: 7,
            suspect: false,
            suspect_at: None,
            suspect_reason: None,
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: None,
            triggering_user_id: None,
        });
        let client = client_with_routes(repo, false).await;
        let response = client.get("/api/matrix").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body: Vec<MatrixLink> = response.into_json().await.unwrap();
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].req_id, 1);
        assert_eq!(body[0].verification_id, 2);
    }

    #[rocket::async_test]
    async fn list_by_project_returns_empty_when_no_links() {
        let mut repo = DieselRepoMock::default().with_admin_user();
        repo.projects.insert(
            PROJECT_ID,
            Project {
                id: PROJECT_ID,
                name: "P".into(),
                description: None,
                creation_date: None,
                update_date: None,
                status: ProjectStatus::Active,
                owner_id: Some(ADMIN_ID),
                slug: "p".into(),
                group_id: None,
            },
        );
        repo.project_members.push(ProjectMember {
            project_id: PROJECT_ID,
            user_id: ADMIN_ID,
            role: 1,
            created_at: epoch(),
            updated_at: epoch(),
        });
        let client = client_with_routes(repo, true).await;
        let response = client
            .get(format!("/api/projects/{}/matrix", PROJECT_ID))
            .private_cookie({
                let mut c = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
                c.set_path("/");
                c.set_http_only(true);
                c.set_secure(true);
                c.set_same_site(SameSite::Strict);
                c
            })
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: Vec<MatrixLink> = response.into_json().await.unwrap();
        assert!(body.is_empty());
    }

    #[rocket::async_test]
    async fn list_by_project_returns_links_for_project_only() {
        let mut repo = DieselRepoMock::default().with_admin_user();
        repo.projects.insert(
            PROJECT_ID,
            Project {
                id: PROJECT_ID,
                name: "P".into(),
                description: None,
                creation_date: None,
                update_date: None,
                status: ProjectStatus::Active,
                owner_id: Some(ADMIN_ID),
                slug: "p".into(),
                group_id: None,
            },
        );
        repo.project_members.push(ProjectMember {
            project_id: PROJECT_ID,
            user_id: ADMIN_ID,
            role: 1,
            created_at: epoch(),
            updated_at: epoch(),
        });
        repo.matrices.push(MatrixLink {
            req_id: 1,
            verification_id: 2,
            creation_date: epoch(),
            project_id: PROJECT_ID,
            suspect: false,
            suspect_at: None,
            suspect_reason: None,
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: None,
            triggering_user_id: None,
        });
        repo.matrices.push(MatrixLink {
            req_id: 3,
            verification_id: 4,
            creation_date: epoch(),
            project_id: 999,
            suspect: false,
            suspect_at: None,
            suspect_reason: None,
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: None,
            triggering_user_id: None,
        });
        let client = client_with_routes(repo, true).await;
        let response = client
            .get(format!("/api/projects/{}/matrix", PROJECT_ID))
            .private_cookie({
                let mut c = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
                c.set_path("/");
                c.set_http_only(true);
                c.set_secure(true);
                c.set_same_site(SameSite::Strict);
                c
            })
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: Vec<MatrixLink> = response.into_json().await.unwrap();
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].project_id, PROJECT_ID);
    }
}

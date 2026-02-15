//! API routes for immutable project baselines.

use rocket::serde::Deserialize;

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::models::{Baseline, BaselineTraceability, NewBaseline, Requirement};
use crate::services::baseline_service::BaselineDiff;
use crate::services::BaselineService;

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct CreateBaselineRequest {
    pub name: String,
    pub description: Option<String>,
}

/// List baselines (session or Bearer). Project-scoped.
#[get("/projects/<project_id>/baselines")]
pub async fn list(
    _access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<Baseline>>> {
    let service = BaselineService::new(state.inner());
    let baselines = service.list_by_project(project_id)?;
    Ok(Json(baselines))
}

/// Get baseline by id (session or Bearer). Project-scoped.
#[get("/projects/<project_id>/baselines/<baseline_id>")]
pub async fn get(
    _access: ProjectAccessOrBearer,
    project_id: i32,
    baseline_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Baseline>> {
    let service = BaselineService::new(state.inner());
    let baseline = service.get_by_id(baseline_id)?;
    if baseline.project_id != project_id {
        return Err(ApiError::NotFound(
            "baseline not found in this project".into(),
        ));
    }
    Ok(Json(baseline))
}

/// Create baseline (session or Bearer). Project-scoped; supports MCP Phase 2 draft_write.
#[post("/projects/<project_id>/baselines", data = "<payload>")]
pub async fn create(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
    payload: Json<CreateBaselineRequest>,
) -> ApiResult<Json<Baseline>> {
    let payload = payload.into_inner();
    let new_baseline = NewBaseline {
        name: payload.name,
        description: payload.description,
    };
    let service = BaselineService::new(state.inner());
    let baseline = service.create_baseline(project_id, access.user().id, &new_baseline)?;
    Ok(Json(baseline))
}

/// Retrieve baseline contents: requirements as at baseline time (from snapshot). Session or Bearer.
#[get("/projects/<project_id>/baselines/<baseline_id>/requirements")]
pub async fn get_requirements(
    _access: ProjectAccessOrBearer,
    project_id: i32,
    baseline_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<Requirement>>> {
    let service = BaselineService::new(state.inner());
    let baseline = service.get_by_id(baseline_id)?;
    if baseline.project_id != project_id {
        return Err(ApiError::NotFound(
            "baseline not found in this project".into(),
        ));
    }
    let requirements = service.get_requirements(baseline_id)?;
    Ok(Json(requirements))
}

/// Retrieve baseline traceability snapshot (requirement–test links). Session or Bearer.
#[get("/projects/<project_id>/baselines/<baseline_id>/traceability")]
pub async fn get_traceability(
    _access: ProjectAccessOrBearer,
    project_id: i32,
    baseline_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<BaselineTraceability>>> {
    let service = BaselineService::new(state.inner());
    let baseline = service.get_by_id(baseline_id)?;
    if baseline.project_id != project_id {
        return Err(ApiError::NotFound(
            "baseline not found in this project".into(),
        ));
    }
    let traceability = service.get_traceability(baseline_id)?;
    Ok(Json(traceability))
}

/// Compare two baselines. Query: baseline_a, baseline_b. Accepts session or Bearer token.
#[get("/projects/<project_id>/baselines/diff?<baseline_a>&<baseline_b>")]
pub async fn diff_baselines(
    _access: ProjectAccessOrBearer,
    project_id: i32,
    baseline_a: i32,
    baseline_b: i32,
    state: &State<AppState>,
) -> ApiResult<Json<BaselineDiff>> {
    let service = BaselineService::new(state.inner());
    let diff = service.diff_baselines(project_id, baseline_a, baseline_b)?;
    Ok(Json(diff))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::models::{Baseline, BaselineTraceability, Project, Requirement, RequirementVersion};
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use crate::status_enums::ProjectStatus;
    use chrono::NaiveDate;
    use rocket::http::{ContentType, Cookie, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::json;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;
    const PROJECT_ID: i32 = 1;

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(state_from_repo(repo.with_admin_user()))
            .mount(
                "/api",
                routes![list, get, create, get_requirements, get_traceability],
            );
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
        cookie.set_path("/");
        cookie
    }

    fn repo_with_project() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default().with_admin_user();
        repo.projects.insert(
            PROJECT_ID,
            Project {
                id: PROJECT_ID,
                name: "Test Project".into(),
                description: Some("Description".into()),
                creation_date: None,
                update_date: None,
                status: ProjectStatus::Active,
                owner_id: Some(ADMIN_ID),
            },
        );
        repo
    }

    /// Repo with one requirement and its snapshot version so baseline creation snapshots it.
    fn repo_with_project_and_requirement() -> DieselRepoMock {
        let created = NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        const SNAPSHOT_VERSION_ID: i32 = 100;
        let mut repo = repo_with_project();
        repo.requirement_versions.insert(
            SNAPSHOT_VERSION_ID,
            RequirementVersion {
                id: SNAPSHOT_VERSION_ID,
                requirement_id: 1,
                title: "Baseline snapshot title".into(),
                description: "Snapshot description".into(),
                status_id: 1,
                author_id: ADMIN_ID,
                reviewer_id: ADMIN_ID,
                category_id: 1,
                parent_id: None,
                applicability_id: 1,
                justification: None,
                deadline_date: None,
                created_at: created,
                approval_state: "approved".to_string(),
                approved_by: Some(ADMIN_ID),
                approved_at: Some(created),
            },
        );
        repo.requirements.insert(
            1,
            Requirement {
                id: 1,
                current_version_id: Some(SNAPSHOT_VERSION_ID),
                same_as_current: None,
                title: "Live title".into(),
                description: "Live description".into(),
                status_id: 1,
                author_id: ADMIN_ID,
                reviewer_id: ADMIN_ID,
                reference_code: "REQ-1".into(),
                category_id: 1,
                parent_id: None,
                creation_date: created,
                update_date: created,
                deadline_date: None,
                applicability_id: 1,
                justification: None,
                project_id: PROJECT_ID,
                approval_state: "approved".to_string(),
                approved_by: Some(ADMIN_ID),
                approved_at: Some(created),
                custom_fields: None,
            },
        );
        repo
    }

    #[rocket::async_test]
    async fn list_returns_empty_array() {
        let client = client_with_repo(repo_with_project()).await;
        let response = client
            .get(format!("/api/projects/{PROJECT_ID}/baselines"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let baselines: Vec<Baseline> = response.into_json().await.unwrap();
        assert!(baselines.is_empty());
    }

    #[rocket::async_test]
    async fn create_returns_baseline() {
        let client = client_with_repo(repo_with_project()).await;
        let response = client
            .post(format!("/api/projects/{PROJECT_ID}/baselines"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(
                json!({
                    "name": "Release 1.0",
                    "description": "Initial release baseline"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let baseline: Baseline = response.into_json().await.unwrap();
        assert_eq!(baseline.name, "Release 1.0");
        assert_eq!(
            baseline.description.as_deref(),
            Some("Initial release baseline")
        );
        assert_eq!(baseline.project_id, PROJECT_ID);
        assert_eq!(baseline.created_by, ADMIN_ID);
        assert!(baseline.id >= 1);
    }

    #[rocket::async_test]
    async fn get_returns_baseline_when_project_matches() {
        let client = client_with_repo(repo_with_project()).await;
        let create_resp = client
            .post(format!("/api/projects/{PROJECT_ID}/baselines"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(json!({ "name": "Baseline A", "description": null }).to_string())
            .dispatch()
            .await;
        assert_eq!(create_resp.status(), Status::Ok);
        let created: Baseline = create_resp.into_json().await.unwrap();
        let baseline_id = created.id;

        let get_resp = client
            .get(format!(
                "/api/projects/{PROJECT_ID}/baselines/{baseline_id}"
            ))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(get_resp.status(), Status::Ok);
        let baseline: Baseline = get_resp.into_json().await.unwrap();
        assert_eq!(baseline.id, baseline_id);
        assert_eq!(baseline.name, "Baseline A");
    }

    #[rocket::async_test]
    async fn get_returns_not_found_when_baseline_in_different_project() {
        let client = client_with_repo(repo_with_project()).await;
        let create_resp = client
            .post(format!("/api/projects/{PROJECT_ID}/baselines"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(json!({ "name": "Baseline A", "description": null }).to_string())
            .dispatch()
            .await;
        assert_eq!(create_resp.status(), Status::Ok);
        let created: Baseline = create_resp.into_json().await.unwrap();
        let baseline_id = created.id;
        let other_project_id = 999;

        let get_resp = client
            .get(format!(
                "/api/projects/{other_project_id}/baselines/{baseline_id}"
            ))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(get_resp.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn get_requirements_returns_json_array() {
        let client = client_with_repo(repo_with_project()).await;
        let create_resp = client
            .post(format!("/api/projects/{PROJECT_ID}/baselines"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(json!({ "name": "Empty baseline", "description": null }).to_string())
            .dispatch()
            .await;
        assert_eq!(create_resp.status(), Status::Ok);
        let created: Baseline = create_resp.into_json().await.unwrap();

        let req_resp = client
            .get(format!(
                "/api/projects/{PROJECT_ID}/baselines/{}/requirements",
                created.id
            ))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(req_resp.status(), Status::Ok);
        let requirements: Vec<Requirement> = req_resp.into_json().await.unwrap();
        assert!(requirements.is_empty());
    }

    #[rocket::async_test]
    async fn get_requirements_returns_snapshot_version_id_and_content() {
        let client = client_with_repo(repo_with_project_and_requirement()).await;
        let create_resp = client
            .post(format!("/api/projects/{PROJECT_ID}/baselines"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(json!({ "name": "With req", "description": null }).to_string())
            .dispatch()
            .await;
        assert_eq!(create_resp.status(), Status::Ok);
        let created: Baseline = create_resp.into_json().await.unwrap();

        let req_resp = client
            .get(format!(
                "/api/projects/{PROJECT_ID}/baselines/{}/requirements",
                created.id
            ))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(req_resp.status(), Status::Ok);
        let requirements: Vec<Requirement> = req_resp.into_json().await.unwrap();
        assert_eq!(requirements.len(), 1);
        let r = &requirements[0];
        assert_eq!(r.id, 1);
        assert_eq!(
            r.current_version_id,
            Some(100),
            "baseline requirements must expose snapshot version_id for versioned links"
        );
        assert_eq!(r.title, "Baseline snapshot title");
        assert_eq!(r.description, "Snapshot description");
    }

    #[rocket::async_test]
    async fn get_traceability_returns_json_array() {
        let client = client_with_repo(repo_with_project()).await;
        let create_resp = client
            .post(format!("/api/projects/{PROJECT_ID}/baselines"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(json!({ "name": "Baseline", "description": null }).to_string())
            .dispatch()
            .await;
        assert_eq!(create_resp.status(), Status::Ok);
        let created: Baseline = create_resp.into_json().await.unwrap();

        let trace_resp = client
            .get(format!(
                "/api/projects/{PROJECT_ID}/baselines/{}/traceability",
                created.id
            ))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(trace_resp.status(), Status::Ok);
        let traceability: Vec<BaselineTraceability> = trace_resp.into_json().await.unwrap();
        assert!(traceability.is_empty());
    }

    #[rocket::async_test]
    async fn list_returns_created_baselines() {
        let client = client_with_repo(repo_with_project()).await;
        client
            .post(format!("/api/projects/{PROJECT_ID}/baselines"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(json!({ "name": "First", "description": null }).to_string())
            .dispatch()
            .await;
        client
            .post(format!("/api/projects/{PROJECT_ID}/baselines"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(json!({ "name": "Second", "description": null }).to_string())
            .dispatch()
            .await;

        let list_resp = client
            .get(format!("/api/projects/{PROJECT_ID}/baselines"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(list_resp.status(), Status::Ok);
        let baselines: Vec<Baseline> = list_resp.into_json().await.unwrap();
        assert_eq!(baselines.len(), 2);
        let names: Vec<&str> = baselines.iter().map(|b| b.name.as_str()).collect();
        assert!(names.contains(&"First"));
        assert!(names.contains(&"Second"));
    }
}

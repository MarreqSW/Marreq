//! Traceability (matrix) API: suspect link management, trace_up/trace_down, coverage report.

use rocket::serde::{Deserialize, Serialize};

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::models::{Requirement, TestCase};
use crate::repository::{MatrixRepository, RequirementsRepository, TestsCaseRepository};
use crate::services::{MatrixService, RequirementService};

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct ClearSuspectRequest {
    pub req_id: i32,
    pub test_id: i32,
}

/// Trace up: parent requirement(s) for a requirement. Project-scoped; accepts session or Bearer.
#[get("/projects/<project_id>/requirements/<id>/trace_up")]
pub async fn trace_up(
    _access: ProjectAccessOrBearer,
    project_id: i32,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<TraceUpResponse>> {
    let service = RequirementService::new(state.inner());
    let requirement = service.get_by_id(id)?;
    if requirement.project_id != project_id {
        return Err(ApiError::NotFound("requirement not in project".into()));
    }
    let parent = requirement
        .parent_id
        .and_then(|pid| service.get_by_id(pid).ok())
        .filter(|p| p.project_id == project_id);
    Ok(Json(TraceUpResponse { parent }))
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct TraceUpResponse {
    pub parent: Option<Requirement>,
}

/// Trace down: child requirements and linked tests. Project-scoped; accepts session or Bearer.
#[get("/projects/<project_id>/requirements/<id>/trace_down")]
pub async fn trace_down(
    _access: ProjectAccessOrBearer,
    project_id: i32,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<TraceDownResponse>> {
    let req_service = RequirementService::new(state.inner());
    let requirement = req_service.get_by_id(id)?;
    if requirement.project_id != project_id {
        return Err(ApiError::NotFound("requirement not in project".into()));
    }
    let child_requirements = req_service.get_children_by_parent_and_project(project_id, id)?;
    let linked_tests = req_service.get_linked_tests(id)?;
    Ok(Json(TraceDownResponse {
        child_requirements,
        linked_tests,
    }))
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct TraceDownResponse {
    pub child_requirements: Vec<Requirement>,
    pub linked_tests: Vec<TestCase>,
}

/// Coverage report: requirements without tests, tests without requirements, suspect links. Project-scoped.
#[get("/projects/<project_id>/coverage_report")]
pub async fn coverage_report(
    _access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<CoverageReport>> {
    let repo = state.repo_read();
    let requirements = repo.get_requirements_by_project(project_id)?;
    let tests = repo.get_tests_by_project(project_id)?;
    let links = repo.get_matrix_by_project(project_id)?;

    let req_ids_with_tests: std::collections::HashSet<i32> =
        links.iter().map(|m| m.req_id).collect();
    let test_ids_with_reqs: std::collections::HashSet<i32> =
        links.iter().map(|m| m.test_id).collect();

    let requirements_without_tests: Vec<i32> = requirements
        .iter()
        .filter(|r| !req_ids_with_tests.contains(&r.id))
        .map(|r| r.id)
        .collect();
    let tests_without_requirements: Vec<i32> = tests
        .iter()
        .filter(|t| !test_ids_with_reqs.contains(&t.id))
        .map(|t| t.id)
        .collect();
    let suspect_links: Vec<SuspectLink> = links
        .iter()
        .filter(|m| m.suspect)
        .map(|m| SuspectLink {
            req_id: m.req_id,
            test_id: m.test_id,
        })
        .collect();

    Ok(Json(CoverageReport {
        requirements_without_tests,
        tests_without_requirements,
        suspect_links,
    }))
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct SuspectLink {
    pub req_id: i32,
    pub test_id: i32,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct CoverageReport {
    pub requirements_without_tests: Vec<i32>,
    pub tests_without_requirements: Vec<i32>,
    pub suspect_links: Vec<SuspectLink>,
}

/// Clear the suspect flag for a traceability link. Records current user and timestamp (auditable).
#[post("/traceability/clear_suspect", data = "<body>")]
pub async fn clear_suspect(
    user: ApiUser,
    state: &State<AppState>,
    body: Json<ClearSuspectRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let payload = body.into_inner();
    let service = MatrixService::new(state.inner());
    let updated = service.clear_suspect(user.user(), payload.req_id, payload.test_id)?;
    Ok(Json(serde_json::json!({
        "status": if updated { "ok" } else { "no_change" },
        "cleared": updated
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::models::{MatrixLink, Project, ProjectMember, Requirement, TestCase};
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use crate::status_enums::ProjectStatus;
    use chrono::NaiveDate;
    use rocket::http::{ContentType, Cookie};
    use rocket::local::asynchronous::Client;
    use rocket::serde::json::Value;
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

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    fn auth_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
        cookie.set_path("/");
        cookie
    }

    fn repo_with_project_and_requirement() -> DieselRepoMock {
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
            },
        );
        repo.project_members.push(ProjectMember {
            project_id: PROJECT_ID,
            user_id: ADMIN_ID,
            role: 1,
            created_at: epoch(),
            updated_at: epoch(),
        });
        repo.requirements.insert(
            1,
            Requirement {
                id: 1,
                current_version_id: None,
                same_as_current: None,
                title: "R1".into(),
                description: "D1".into(),
                status_id: 1,
                author_id: ADMIN_ID,
                reviewer_id: ADMIN_ID,
                reference_code: "REF-1".into(),
                category_id: 1,
                parent_id: None,
                creation_date: epoch(),
                update_date: epoch(),
                deadline_date: None,
                applicability_id: 1,
                justification: None,
                project_id: PROJECT_ID,
                approval_state: "draft".into(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            },
        );
        repo
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build().manage(state_from_repo(repo)).mount(
            "/api",
            routes![trace_up, trace_down, coverage_report, clear_suspect],
        );
        Client::tracked(rocket).await.unwrap()
    }

    #[rocket::async_test]
    async fn clear_suspect_returns_ok_and_cleared_true_when_link_was_suspect() {
        let mut repo = DieselRepoMock::default().with_admin_user();
        repo.matrices.push(MatrixLink {
            req_id: 1,
            test_id: 2,
            creation_date: epoch(),
            project_id: 7,
            suspect: true,
            suspect_at: Some(epoch()),
            suspect_reason: Some("Requirement updated".into()),
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: None,
            triggering_user_id: None,
        });
        let client = client_with_repo(repo).await;
        let response = client
            .post("/api/traceability/clear_suspect")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(r#"{"req_id":1,"test_id":2}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), rocket::http::Status::Ok);
        let body: Value = response.into_json().await.unwrap();
        assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("ok"));
        assert_eq!(body.get("cleared"), Some(&Value::from(true)));
    }

    #[rocket::async_test]
    async fn clear_suspect_returns_ok_and_cleared_false_when_link_missing() {
        let client = client_with_repo(DieselRepoMock::default().with_admin_user()).await;
        let response = client
            .post("/api/traceability/clear_suspect")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(r#"{"req_id":99,"test_id":99}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), rocket::http::Status::Ok);
        let body: Value = response.into_json().await.unwrap();
        assert_eq!(
            body.get("status").and_then(|v| v.as_str()),
            Some("no_change")
        );
        assert_eq!(body.get("cleared"), Some(&Value::from(false)));
    }

    #[rocket::async_test]
    async fn clear_suspect_requires_auth() {
        let client = client_with_repo(DieselRepoMock::default().with_admin_user()).await;
        let response = client
            .post("/api/traceability/clear_suspect")
            .header(ContentType::JSON)
            .body(r#"{"req_id":1,"test_id":1}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), rocket::http::Status::Unauthorized);
    }

    #[rocket::async_test]
    async fn trace_up_returns_parent_none_when_no_parent() {
        let client = client_with_repo(repo_with_project_and_requirement()).await;
        let response = client
            .get(format!(
                "/api/projects/{}/requirements/1/trace_up",
                PROJECT_ID
            ))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), rocket::http::Status::Ok);
        let body: Value = response.into_json().await.unwrap();
        assert!(body.get("parent").is_some());
        assert!(
            body.get("parent").unwrap().is_null(),
            "expected parent to be null when requirement has no parent"
        );
    }

    #[rocket::async_test]
    async fn trace_up_returns_404_when_requirement_not_in_project() {
        let client = client_with_repo(repo_with_project_and_requirement()).await;
        let response = client
            .get(format!(
                "/api/projects/{}/requirements/999/trace_up",
                PROJECT_ID
            ))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), rocket::http::Status::NotFound);
    }

    #[rocket::async_test]
    async fn trace_down_returns_children_and_linked_tests() {
        let mut repo = repo_with_project_and_requirement();
        repo.tests.insert(
            10,
            TestCase {
                id: 10,
                name: "T1".into(),
                reference_code: "T-1".into(),
                description: "".into(),
                source: "".into(),
                status_id: 1,
                parent_id: None,
                project_id: PROJECT_ID,
            },
        );
        repo.matrices.push(MatrixLink {
            req_id: 1,
            test_id: 10,
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
        let client = client_with_repo(repo).await;
        let response = client
            .get(format!(
                "/api/projects/{}/requirements/1/trace_down",
                PROJECT_ID
            ))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), rocket::http::Status::Ok);
        let body: Value = response.into_json().await.unwrap();
        let linked = body.get("linked_tests").and_then(|v| v.as_array()).unwrap();
        assert_eq!(linked.len(), 1);
    }

    #[rocket::async_test]
    async fn coverage_report_returns_empty_arrays_when_no_data() {
        let client = client_with_repo(repo_with_project_and_requirement()).await;
        let response = client
            .get(format!("/api/projects/{}/coverage_report", PROJECT_ID))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), rocket::http::Status::Ok);
        let body: Value = response.into_json().await.unwrap();
        assert!(body.get("requirements_without_tests").is_some());
        assert!(body.get("tests_without_requirements").is_some());
        assert!(body.get("suspect_links").is_some());
        let reqs: &Vec<_> = body
            .get("requirements_without_tests")
            .unwrap()
            .as_array()
            .unwrap();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].as_i64(), Some(1));
    }
}

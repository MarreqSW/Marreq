// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! API endpoints for requirement version diffs (read-only, deterministic).

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::diff::RequirementDiff;
use crate::services::{RequirementDiffService, RequirementService};

/// Diff two versions of a requirement (v1 = old, v2 = new).
/// Both version IDs must belong to the given requirement.
#[get("/requirements/<req_id>/versions/<v1>/diff/<v2>")]
pub async fn diff_versions(
    _user: ApiUser,
    req_id: i32,
    v1: i32,
    v2: i32,
    state: &State<AppState>,
) -> ApiResult<Json<RequirementDiff>> {
    let service = RequirementDiffService::new(state.inner());
    let diff = service.diff_versions(req_id, v1, v2)?;
    Ok(Json(diff))
}

/// Project-scoped diff two versions (session or Bearer). Enforces requirement belongs to project.
#[get("/projects/<project_id>/requirements/<req_id>/versions/<v1>/diff/<v2>")]
pub async fn diff_versions_by_project(
    _access: ProjectAccessOrBearer,
    project_id: i32,
    req_id: i32,
    v1: i32,
    v2: i32,
    state: &State<AppState>,
) -> ApiResult<Json<RequirementDiff>> {
    let req_service = RequirementService::new(state.inner());
    let requirement = req_service.get_by_id(req_id)?;
    if requirement.project_id != project_id {
        return Err(ApiError::NotFound("requirement not in project".into()));
    }
    let service = RequirementDiffService::new(state.inner());
    let diff = service.diff_versions(req_id, v1, v2)?;
    Ok(Json(diff))
}

/// Diff the requirement as stored in the baseline vs the current version (session or Bearer).
#[get("/projects/<project_id>/baselines/<baseline_id>/requirements/<req_id>/diff/current")]
pub async fn diff_baseline_vs_current(
    _access: ProjectAccessOrBearer,
    project_id: i32,
    baseline_id: i32,
    req_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<RequirementDiff>> {
    let service = RequirementDiffService::new(state.inner());
    let diff = service.diff_baseline_vs_current(project_id, baseline_id, req_id)?;
    Ok(Json(diff))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{baselines, requirements};
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::diff::RequirementDiff;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::{ContentType, Cookie};
    use rocket::local::asynchronous::Client;
    use serde_json::Value;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;

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
                routes![
                    requirements::list,
                    requirements::get,
                    requirements::list_versions,
                    requirements::get_version,
                    requirements::create,
                    requirements::patch_requirement,
                    baselines::create,
                    diff_versions,
                    diff_baseline_vs_current,
                ],
            );
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
        cookie.set_path("/");
        cookie
    }

    fn sample_requirement(title: &str) -> Value {
        serde_json::json!({
            "title": title,
            "description": format!("{title} description"),
            "verification_method_ids": [1],
            "author_id": 1,
            "category_id": 1,
            "status_id": 1,
            "parent_id": null,
            "reference_code": "REF-1",
            "reviewer_id": 2,
            "applicability_id": 3,
            "justification": null,
            "project_id": 1
        })
    }

    #[rocket::async_test]
    async fn diff_versions_returns_structured_diff() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let mut req = sample_requirement("V1 Title");
        req["reference_code"] = serde_json::Value::from("REQ-001");
        let create_resp = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(req.to_string())
            .dispatch()
            .await;
        assert_eq!(create_resp.status(), Status::Ok);
        let created: Value = create_resp.into_json().await.unwrap();
        let req_id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        client
            .patch(format!("/api/requirements/{}", req_id))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(serde_json::json!({ "title": "V2 Updated" }).to_string())
            .dispatch()
            .await;

        let versions_resp = client
            .get(format!("/api/requirements/{}/versions", req_id))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(versions_resp.status(), Status::Ok);
        let versions: Vec<Value> = versions_resp.into_json().await.unwrap();
        assert!(versions.len() >= 2);
        let v2_id = versions[0].get("id").and_then(Value::as_i64).unwrap() as i32;
        let v1_id = versions[1].get("id").and_then(Value::as_i64).unwrap() as i32;

        let diff_resp = client
            .get(format!(
                "/api/requirements/{}/versions/{}/diff/{}",
                req_id, v1_id, v2_id
            ))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(diff_resp.status(), Status::Ok);
        let diff: RequirementDiff = diff_resp.into_json().await.unwrap();
        assert!(!diff.text.title.added.is_empty() || !diff.text.title.removed.is_empty());
        assert!(diff.text.description.added.is_empty() && diff.text.description.removed.is_empty());
        assert!(diff.metadata.status.unchanged.is_some());
    }

    #[rocket::async_test]
    async fn diff_versions_404_when_version_not_belong_to_requirement() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create1 = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(sample_requirement("Req1").to_string())
            .dispatch()
            .await;
        let id1 = create1
            .into_json::<Value>()
            .await
            .unwrap()
            .get("id")
            .and_then(Value::as_i64)
            .unwrap() as i32;
        let create2 = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(
                sample_requirement("Req2")
                    .to_string()
                    .replace("\"REF-1\"", "\"REF-2\""),
            )
            .dispatch()
            .await;
        let id2 = create2
            .into_json::<Value>()
            .await
            .unwrap()
            .get("id")
            .and_then(Value::as_i64)
            .unwrap() as i32;
        let versions_resp = client
            .get(format!("/api/requirements/{}/versions", id1))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        let versions: Vec<Value> = versions_resp.into_json().await.unwrap();
        let v1_id = versions[0].get("id").and_then(Value::as_i64).unwrap() as i32;
        let diff_resp = client
            .get(format!(
                "/api/requirements/{}/versions/{}/diff/{}",
                id2, v1_id, v1_id
            ))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(diff_resp.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn diff_baseline_vs_current_returns_diff() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_req = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(sample_requirement("Before Baseline").to_string())
            .dispatch()
            .await;
        let req_id = create_req
            .into_json::<Value>()
            .await
            .unwrap()
            .get("id")
            .and_then(Value::as_i64)
            .unwrap() as i32;
        let create_bl = client
            .post("/api/projects/1/baselines")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(serde_json::json!({ "name": "BL", "description": null }).to_string())
            .dispatch()
            .await;
        assert_eq!(create_bl.status(), Status::Ok);
        let baseline: Value = create_bl.into_json::<Value>().await.unwrap();
        let baseline_id = baseline.get("id").and_then(Value::as_i64).unwrap() as i32;
        client
            .patch(format!("/api/requirements/{}", req_id))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(serde_json::json!({ "title": "After Baseline" }).to_string())
            .dispatch()
            .await;
        let diff_resp = client
            .get(format!(
                "/api/projects/1/baselines/{}/requirements/{}/diff/current",
                baseline_id, req_id
            ))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(diff_resp.status(), Status::Ok);
        let diff: RequirementDiff = diff_resp.into_json().await.unwrap();
        assert!(!diff.text.title.added.is_empty() || !diff.text.title.removed.is_empty());
    }

    #[rocket::async_test]
    async fn diff_baseline_vs_current_404_when_requirement_not_in_baseline() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_bl = client
            .post("/api/projects/1/baselines")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(serde_json::json!({ "name": "Empty BL", "description": null }).to_string())
            .dispatch()
            .await;
        assert_eq!(create_bl.status(), Status::Ok);
        let baseline: Value = create_bl.into_json::<Value>().await.unwrap();
        let baseline_id = baseline.get("id").and_then(Value::as_i64).unwrap() as i32;
        let create_req = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(sample_requirement("Req After BL").to_string())
            .dispatch()
            .await;
        let req_id = create_req
            .into_json::<Value>()
            .await
            .unwrap()
            .get("id")
            .and_then(Value::as_i64)
            .unwrap() as i32;
        let diff_resp = client
            .get(format!(
                "/api/projects/1/baselines/{}/requirements/{}/diff/current",
                baseline_id, req_id
            ))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(diff_resp.status(), Status::NotFound);
    }
}

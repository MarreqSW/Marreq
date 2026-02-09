use rocket::serde::Deserialize;

use crate::api::prelude::*;
use crate::models::{NewRequirement, Requirement, RequirementVersion, TestCase};
use crate::repository::{ProjectMembersRepository, RequirementsRepository};
use crate::services::RequirementService;

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct RequirementCreateRequest {
    pub title: String,
    pub description: String,
    pub author_id: i32,
    pub category_id: i32,
    pub status_id: i32,
    pub parent_id: Option<i32>,
    pub reference_code: String,
    pub reviewer_id: i32,
    pub applicability_id: i32,
    pub justification: Option<String>,
    pub project_id: i32,
    #[serde(default)]
    pub verification_method_ids: Vec<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct RequirementPatch {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status_id: Option<i32>,
    pub verification_method_ids: Option<Vec<i32>>,
    pub author_id: Option<i32>,
    pub reviewer_id: Option<i32>,
    pub category_id: Option<i32>,
    pub applicability_id: Option<i32>,
    pub parent_id: Option<i32>,
}

#[get("/requirements")]
pub async fn list(_user: ApiUser, state: &State<AppState>) -> ApiResult<Json<Vec<Requirement>>> {
    let service = RequirementService::new(state.inner());
    let requirements = service.list_all()?;
    Ok(Json(requirements))
}

#[get("/requirements/<id>")]
pub async fn get(_user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Json<Requirement>> {
    let service = RequirementService::new(state.inner());
    let requirement = service.get_by_id(id)?;
    Ok(Json(requirement))
}

/// List all versions for a requirement (newest first).
#[get("/requirements/<id>/versions")]
pub async fn list_versions(
    _user: ApiUser,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<RequirementVersion>>> {
    let service = RequirementService::new(state.inner());
    let versions = service.list_versions(id)?;
    Ok(Json(versions))
}

/// Get a single requirement version by id (version must belong to the given requirement).
#[get("/requirements/<req_id>/versions/<version_id>")]
pub async fn get_version(
    _user: ApiUser,
    req_id: i32,
    version_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<RequirementVersion>> {
    let service = RequirementService::new(state.inner());
    let version = service.get_version_by_id(version_id)?;
    if version.requirement_id != req_id {
        return Err(ApiError::NotFound(
            "version does not belong to requirement".into(),
        ));
    }
    Ok(Json(version))
}

/// List tests linked to the requirement that are currently marked suspect (impacted by requirement changes).
#[get("/requirements/<id>/impacted_tests")]
pub async fn get_impacted_tests(
    _user: ApiUser,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<TestCase>>> {
    let service = RequirementService::new(state.inner());
    let _requirement = service.get_by_id(id)?;
    let tests = service.get_impacted_tests(id)?;
    Ok(Json(tests))
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct SetApprovalRequest {
    /// Target state: "reviewed" or "approved"
    pub state: String,
}

/// Transition a requirement version's approval state (draft→reviewed, reviewed→approved).
/// Restricted to project owners (role 1), managers (role 2), or admins.
#[put(
    "/requirements/<req_id>/versions/<version_id>/approval",
    data = "<payload>"
)]
pub async fn set_version_approval(
    user: ApiUser,
    req_id: i32,
    version_id: i32,
    state: &State<AppState>,
    payload: Json<SetApprovalRequest>,
) -> ApiResult<Json<RequirementVersion>> {
    let service = RequirementService::new(state.inner());
    let version = service.get_version_by_id(version_id)?;
    if version.requirement_id != req_id {
        return Err(ApiError::NotFound(
            "version does not belong to requirement".into(),
        ));
    }
    let requirement = service.get_by_id(req_id)?;
    let members = state
        .repo_read()
        .get_members_by_project(requirement.project_id)
        .map_err(ApiError::from)?;
    let u = user.user();
    let can_approve = u.is_admin
        || members
            .iter()
            .any(|m| m.user_id == u.id && (m.role == 1 || m.role == 2));
    if !can_approve {
        return Err(ApiError::Forbidden(
            "only project owners or managers can approve requirement versions".into(),
        ));
    }
    let new_state = payload.state.trim();
    if new_state != "reviewed" && new_state != "approved" {
        return Err(ApiError::BadRequest(
            "state must be 'reviewed' or 'approved'".into(),
        ));
    }
    let updated = state
        .repo_write()
        .set_requirement_version_approval(version_id, new_state, u.id)?;
    Ok(Json(updated))
}

#[post("/requirements", data = "<payload>")]
pub async fn create(
    user: ApiUser,
    state: &State<AppState>,
    payload: Json<RequirementCreateRequest>,
) -> ApiResult<Value> {
    let payload = payload.into_inner();
    let verification_method_ids: Vec<i32> = payload
        .verification_method_ids
        .into_iter()
        .filter(|&id| id > 0)
        .collect();
    if verification_method_ids.is_empty() {
        return Err(ApiError::BadRequest(
            "at least one verification_method_id required".into(),
        ));
    }
    let new_req = NewRequirement {
        id: None,
        title: payload.title,
        description: payload.description,
        author_id: payload.author_id,
        category_id: payload.category_id,
        status_id: payload.status_id,
        parent_id: payload.parent_id,
        reference_code: payload.reference_code,
        reviewer_id: payload.reviewer_id,
        applicability_id: payload.applicability_id,
        justification: payload.justification,
        project_id: payload.project_id,
    };
    let service = RequirementService::new(state.inner());
    let id = service.create(user.user(), new_req, &verification_method_ids)?;

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/requirements/<id>")]
pub async fn delete(user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Status> {
    let service = RequirementService::new(state.inner());
    service.delete(user.user(), id)?;
    Ok(Status::NoContent)
}

#[patch("/requirements/<id>", data = "<patch>")]
pub async fn patch_requirement(
    user: ApiUser,
    state: &State<AppState>,
    id: i32,
    patch: Json<RequirementPatch>,
) -> ApiResult<Value> {
    let patch = patch.into_inner();
    let any_updates = patch.title.is_some()
        || patch.description.is_some()
        || patch.status_id.is_some()
        || patch.verification_method_ids.is_some()
        || patch.author_id.is_some()
        || patch.reviewer_id.is_some()
        || patch.category_id.is_some()
        || patch.applicability_id.is_some()
        || patch.parent_id.is_some();

    if !any_updates {
        return Err(ApiError::BadRequest("no fields provided".into()));
    }

    let service = RequirementService::new(state.inner());
    let mut requirement = service.get_by_id(id)?;

    if let Some(v) = patch.title {
        requirement.title = v;
    }
    if let Some(v) = patch.description {
        requirement.description = v;
    }
    if let Some(v) = patch.status_id {
        requirement.status_id = v;
    }
    if let Some(v) = patch.author_id {
        requirement.author_id = v;
    }
    if let Some(v) = patch.reviewer_id {
        requirement.reviewer_id = v;
    }
    if let Some(v) = patch.category_id {
        requirement.category_id = v;
    }
    if let Some(v) = patch.applicability_id {
        requirement.applicability_id = v;
    }
    if let Some(v) = patch.parent_id {
        requirement.parent_id = if v == 0 { None } else { Some(v) };
    }

    let verification_method_ids = patch
        .verification_method_ids
        .unwrap_or_else(|| service.get_verification_method_ids(id).unwrap_or_default());
    let verification_method_ids: Vec<i32> = verification_method_ids
        .into_iter()
        .filter(|&id| id > 0)
        .collect();

    let payload = NewRequirement {
        id: Some(requirement.id),
        title: requirement.title.clone(),
        description: requirement.description.clone(),
        author_id: requirement.author_id,
        category_id: requirement.category_id,
        status_id: requirement.status_id,
        parent_id: requirement.parent_id,
        reference_code: requirement.reference_code.clone(),
        reviewer_id: requirement.reviewer_id,
        applicability_id: requirement.applicability_id,
        justification: requirement.justification.clone(),
        project_id: requirement.project_id,
    };

    service.update(user.user(), id, payload, &verification_method_ids)?;

    Ok(json!({
        "success": true,
        "message": "Field updated successfully"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::{ContentType, Cookie};
    use rocket::local::asynchronous::Client;
    use serde_json::{json, Value};
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
                    list,
                    get,
                    list_versions,
                    get_version,
                    create,
                    delete,
                    patch_requirement,
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
        json!({
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
    async fn list_returns_empty_array() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .get("/api/requirements")
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let items: Vec<Requirement> = response.into_json().await.unwrap();
        assert!(items.is_empty());
    }

    #[rocket::async_test]
    async fn create_returns_identifier() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(sample_requirement("First").to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("status"), Some(&Value::from("ok")));
        assert_eq!(payload.get("id"), Some(&Value::from(1)));
    }

    #[rocket::async_test]
    async fn patch_updates_fields() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(sample_requirement("Original").to_string())
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let response = client
            .patch(format!("/api/requirements/{id}"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(
                json!({
                    "title": "Updated",
                    "description": "Updated description"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("success"), Some(&Value::from(true)));

        let get_response = client
            .get(format!("/api/requirements/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        let requirement: Requirement = get_response.into_json().await.unwrap();
        assert_eq!(requirement.title, "Updated");
        assert_eq!(requirement.description, "Updated description");
    }

    #[rocket::async_test]
    async fn patch_creates_new_version_and_versions_list_returns_history() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let mut req = sample_requirement("V1 Title");
        req["reference_code"] = serde_json::Value::from("REQ-001");
        let create_response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(req.to_string())
            .dispatch()
            .await;
        assert_eq!(
            create_response.status(),
            Status::Ok,
            "create should succeed"
        );
        let created: Value = create_response.into_json().await.unwrap();
        let id = created
            .get("id")
            .and_then(Value::as_i64)
            .expect("create response should have id") as i32;

        let versions_after_create = client
            .get(format!("/api/requirements/{id}/versions"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(versions_after_create.status(), Status::Ok);
        let versions: Vec<RequirementVersion> = versions_after_create.into_json().await.unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].title, "V1 Title");

        client
            .patch(format!("/api/requirements/{id}"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(json!({ "title": "V2 Updated" }).to_string())
            .dispatch()
            .await;

        let versions_after_patch = client
            .get(format!("/api/requirements/{id}/versions"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(versions_after_patch.status(), Status::Ok);
        let versions: Vec<RequirementVersion> = versions_after_patch.into_json().await.unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].title, "V2 Updated");
        assert_eq!(versions[1].title, "V1 Title");

        let first_version_id = versions[1].id;
        let single = client
            .get(format!(
                "/api/requirements/{id}/versions/{first_version_id}"
            ))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(single.status(), Status::Ok);
        let v: RequirementVersion = single.into_json().await.unwrap();
        assert_eq!(v.id, first_version_id);
        assert_eq!(v.requirement_id, id);
        assert_eq!(v.title, "V1 Title");
    }

    #[rocket::async_test]
    async fn patch_without_fields_returns_bad_request() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(sample_requirement("Original").to_string())
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let response = client
            .patch(format!("/api/requirements/{id}"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(json!({}).to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::BadRequest);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(
            payload.get("message"),
            Some(&Value::from("no fields provided"))
        );
    }

    #[rocket::async_test]
    async fn delete_removes_requirement() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(sample_requirement("Disposable").to_string())
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let delete_response = client
            .delete(format!("/api/requirements/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(delete_response.status(), Status::NoContent);

        let not_found = client
            .get(format!("/api/requirements/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(not_found.status(), Status::NotFound);
    }
}

use rocket::serde::Deserialize;

use crate::api::prelude::*;
use crate::models::{NewRequirement, Requirement};
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
        || patch.applicability_id.is_some();

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
                routes![list, get, create, delete, patch_requirement],
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

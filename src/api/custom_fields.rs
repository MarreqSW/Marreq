//! REST API for project-scoped custom field definitions.

use rocket::State;

use crate::api::prelude::*;
use crate::models::{CustomFieldDefinition, CustomFieldDefinitionPayload};
use crate::repository::ProjectsRepository;
use crate::services::CustomFieldService;

#[get("/projects/<project_id>/custom_fields")]
pub async fn list_by_project(
    _user: ApiUser,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<CustomFieldDefinition>>> {
    let _ = state
        .repo_read()
        .get_project_by_id(project_id)
        .map_err(ApiError::from)?;
    let service = CustomFieldService::new(state.inner());
    let list = service.list_by_project(project_id)?;
    Ok(Json(list))
}

#[get("/projects/<project_id>/custom_fields/<field_id>")]
pub async fn get(
    _user: ApiUser,
    project_id: i32,
    field_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<CustomFieldDefinition>> {
    let _ = state
        .repo_read()
        .get_project_by_id(project_id)
        .map_err(ApiError::from)?;
    let service = CustomFieldService::new(state.inner());
    let def = service.get_by_id(field_id)?;
    if def.project_id != project_id {
        return Err(ApiError::NotFound("custom field not in project".into()));
    }
    Ok(Json(def))
}

#[post("/projects/<project_id>/custom_fields", data = "<payload>")]
pub async fn create(
    _user: ApiUser,
    project_id: i32,
    state: &State<AppState>,
    payload: Json<CustomFieldDefinitionPayload>,
) -> ApiResult<Value> {
    let _ = state
        .repo_read()
        .get_project_by_id(project_id)
        .map_err(ApiError::from)?;
    let service = CustomFieldService::new(state.inner());
    let id = service.create(project_id, payload.into_inner())?;
    Ok(json!({ "status": "ok", "id": id }))
}

#[put("/projects/<project_id>/custom_fields/<field_id>", data = "<payload>")]
pub async fn update(
    _user: ApiUser,
    project_id: i32,
    field_id: i32,
    state: &State<AppState>,
    payload: Json<CustomFieldDefinitionPayload>,
) -> ApiResult<Value> {
    let _ = state
        .repo_read()
        .get_project_by_id(project_id)
        .map_err(ApiError::from)?;
    let service = CustomFieldService::new(state.inner());
    service.update(field_id, payload.into_inner())?;
    let def = service.get_by_id(field_id)?;
    if def.project_id != project_id {
        return Err(ApiError::NotFound("custom field not in project".into()));
    }
    Ok(json!({
        "status": "ok",
        "message": "Custom field updated successfully"
    }))
}

#[delete("/projects/<project_id>/custom_fields/<field_id>")]
pub async fn delete(
    _user: ApiUser,
    project_id: i32,
    field_id: i32,
    state: &State<AppState>,
) -> ApiResult<Value> {
    let _ = state
        .repo_read()
        .get_project_by_id(project_id)
        .map_err(ApiError::from)?;
    let service = CustomFieldService::new(state.inner());
    let def = service.get_by_id(field_id)?;
    if def.project_id != project_id {
        return Err(ApiError::NotFound("custom field not in project".into()));
    }
    let in_use = service.count_versions_using_field(field_id)?;
    if in_use > 0 {
        return Err(ApiError::BadRequest(format!(
            "Cannot delete: field is in use by {} requirement version(s). Remove or update those values first.",
            in_use
        )));
    }
    service.delete(field_id)?;
    Ok(json!({
        "status": "ok",
        "message": "Custom field deleted"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::models::{CustomFieldDefinition, Project};
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use crate::status_enums::ProjectStatus;
    use chrono::NaiveDate;
    use rocket::http::{ContentType, Cookie, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::Value;
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

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build().manage(state_from_repo(repo)).mount(
            "/api",
            routes![list_by_project, get, create, update, delete],
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
                description: Some("Desc".into()),
                creation_date: None,
                update_date: None,
                status: ProjectStatus::Active,
                owner_id: Some(ADMIN_ID),
            },
        );
        repo
    }

    #[rocket::async_test]
    async fn list_by_project_returns_empty_when_no_fields() {
        let client = client_with_repo(repo_with_project()).await;
        let response = client
            .get(format!("/api/projects/{}/custom_fields", PROJECT_ID))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: Vec<CustomFieldDefinition> = response.into_json().await.unwrap();
        assert!(body.is_empty());
    }

    #[rocket::async_test]
    async fn list_by_project_returns_fields_when_present() {
        let mut repo = repo_with_project();
        repo.custom_field_definitions.insert(
            10,
            CustomFieldDefinition {
                id: 10,
                project_id: PROJECT_ID,
                label: "Priority".into(),
                field_type: "text".into(),
                enum_values: None,
                sort_order: 0,
                created_at: epoch(),
            },
        );
        let client = client_with_repo(repo).await;
        let response = client
            .get(format!("/api/projects/{}/custom_fields", PROJECT_ID))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: Vec<CustomFieldDefinition> = response.into_json().await.unwrap();
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].label, "Priority");
    }

    #[rocket::async_test]
    async fn list_by_project_returns_404_when_project_missing() {
        let client = client_with_repo(repo_with_project()).await;
        let response = client
            .get("/api/projects/999/custom_fields")
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn get_returns_field_when_in_project() {
        let mut repo = repo_with_project();
        repo.custom_field_definitions.insert(
            10,
            CustomFieldDefinition {
                id: 10,
                project_id: PROJECT_ID,
                label: "Priority".into(),
                field_type: "text".into(),
                enum_values: None,
                sort_order: 0,
                created_at: epoch(),
            },
        );
        let client = client_with_repo(repo).await;
        let response = client
            .get(format!("/api/projects/{}/custom_fields/10", PROJECT_ID))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: CustomFieldDefinition = response.into_json().await.unwrap();
        assert_eq!(body.id, 10);
        assert_eq!(body.label, "Priority");
    }

    #[rocket::async_test]
    async fn get_returns_404_when_field_not_in_project() {
        let mut repo = repo_with_project();
        repo.custom_field_definitions.insert(
            10,
            CustomFieldDefinition {
                id: 10,
                project_id: 999,
                label: "Other".into(),
                field_type: "text".into(),
                enum_values: None,
                sort_order: 0,
                created_at: epoch(),
            },
        );
        let client = client_with_repo(repo).await;
        let response = client
            .get(format!("/api/projects/{}/custom_fields/10", PROJECT_ID))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn create_returns_id() {
        let client = client_with_repo(repo_with_project()).await;
        let response = client
            .post(format!("/api/projects/{}/custom_fields", PROJECT_ID))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(
                serde_json::json!({
                    "label": "Severity",
                    "field_type": "text",
                    "sort_order": 1
                })
                .to_string(),
            )
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: Value = response.into_json().await.unwrap();
        assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("ok"));
        assert_eq!(body.get("id"), Some(&Value::from(1)));
    }

    #[rocket::async_test]
    async fn update_returns_ok() {
        let mut repo = repo_with_project();
        repo.custom_field_definitions.insert(
            1,
            CustomFieldDefinition {
                id: 1,
                project_id: PROJECT_ID,
                label: "Old".into(),
                field_type: "text".into(),
                enum_values: None,
                sort_order: 0,
                created_at: epoch(),
            },
        );
        repo.next_custom_field_id = 2;
        let client = client_with_repo(repo).await;
        let response = client
            .put(format!("/api/projects/{}/custom_fields/1", PROJECT_ID))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(
                serde_json::json!({
                    "label": "Updated Label",
                    "field_type": "text",
                    "sort_order": 2
                })
                .to_string(),
            )
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: Value = response.into_json().await.unwrap();
        assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("ok"));
    }

    #[rocket::async_test]
    async fn delete_returns_ok_when_not_in_use() {
        let mut repo = repo_with_project();
        repo.custom_field_definitions.insert(
            1,
            CustomFieldDefinition {
                id: 1,
                project_id: PROJECT_ID,
                label: "Temp".into(),
                field_type: "text".into(),
                enum_values: None,
                sort_order: 0,
                created_at: epoch(),
            },
        );
        repo.next_custom_field_id = 2;
        let client = client_with_repo(repo).await;
        let response = client
            .delete(format!("/api/projects/{}/custom_fields/1", PROJECT_ID))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: Value = response.into_json().await.unwrap();
        assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("ok"));
    }

    #[rocket::async_test]
    async fn delete_returns_400_when_field_in_use() {
        let mut repo = repo_with_project();
        repo.custom_field_definitions.insert(
            1,
            CustomFieldDefinition {
                id: 1,
                project_id: PROJECT_ID,
                label: "InUse".into(),
                field_type: "text".into(),
                enum_values: None,
                sort_order: 0,
                created_at: epoch(),
            },
        );
        repo.custom_field_values
            .push((100, 1, Some("value".into())));
        repo.next_custom_field_id = 2;
        let client = client_with_repo(repo).await;
        let response = client
            .delete(format!("/api/projects/{}/custom_fields/1", PROJECT_ID))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest);
    }
}

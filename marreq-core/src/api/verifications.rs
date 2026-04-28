// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::serde::{Deserialize, Serialize};

use crate::api::prelude::*;
use crate::auth::guards::{ApiUser, ProjectAccessOrBearer};
use crate::models::{NewVerification, Verification};
use crate::repository::errors::RepoError;
use crate::repository::VerificationsRepository;
use crate::services::VerificationService;

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FieldUpdateRequest {
    pub field: String,
    pub value: String,
}

#[get("/verifications")]
pub async fn list(_user: ApiUser, state: &State<AppState>) -> ApiResult<Json<Vec<Verification>>> {
    let service = VerificationService::new(state.inner());
    let verifications = service.list_all()?;
    Ok(Json(verifications))
}

/// Project-scoped verifications (tests). Session or Bearer; requires `ViewRequirements`.
#[get("/projects/<project_id>/verifications")]
pub async fn list_by_project(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<Verification>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = VerificationService::new(state.inner());
    Ok(Json(service.list_by_project(project_id)?))
}

#[get("/verifications/<id>")]
pub async fn get(
    _user: ApiUser,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Verification>> {
    let service = VerificationService::new(state.inner());
    let verification = service.get_by_id(id)?;
    Ok(Json(verification))
}

#[post("/verifications", data = "<payload>")]
pub async fn create(
    user: ApiUser,
    state: &State<AppState>,
    payload: Json<NewVerification>,
) -> ApiResult<Value> {
    let payload = payload.into_inner();
    require_project_permission(
        state,
        user.user(),
        payload.project_id,
        Permission::EditRequirements,
    )?;
    require_project_reviewer_unless_verification_create_status_is_initial(
        state,
        user.user(),
        payload.project_id,
        payload.status_id,
    )?;
    let service = VerificationService::new(state.inner());
    let id = service.create(user.user(), payload)?;

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/verifications/<id>")]
pub async fn delete(user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Status> {
    let service = VerificationService::new(state.inner());
    service.delete(user.user(), id)?;
    Ok(Status::NoContent)
}

fn apply_verification_field_update(
    state: &State<AppState>,
    user: &crate::models::User,
    id: i32,
    update: FieldUpdateRequest,
    project_id_match: Option<i32>,
) -> ApiResult<Value> {
    let service = VerificationService::new(state.inner());
    let mut verification = service.get_by_id(id)?;
    if let Some(pid) = project_id_match {
        if verification.project_id != pid {
            return Err(ApiError::NotFound("verification not in project".into()));
        }
    }
    require_project_permission(
        state,
        user,
        verification.project_id,
        Permission::EditRequirements,
    )?;
    let field_key = update.field.as_str();
    if field_key == "status_id" {
        require_project_reviewer(state, user, verification.project_id)?;
    }

    match field_key {
        "name" => verification.name = update.value,
        "description" => verification.description = update.value,
        "source" => verification.source = update.value,
        "status_id" => {
            verification.status_id = update
                .value
                .parse()
                .map_err(|_| RepoError::BadInput("invalid status id".into()))?;
        }
        "reference_code" => verification.reference_code = update.value,
        "parent_id" => {
            verification.parent_id = if update.value.is_empty() || update.value == "0" {
                None
            } else {
                Some(
                    update
                        .value
                        .parse()
                        .map_err(|_| RepoError::BadInput("invalid parent id".into()))?,
                )
            };
        }
        "verification_method_id" => {
            verification.verification_method_id =
                if update.value.is_empty() || update.value == "0" {
                    None
                } else {
                    Some(update.value.parse().map_err(|_| {
                        RepoError::BadInput("invalid verification_method_id".into())
                    })?)
                };
        }
        "author_id" => {
            verification.author_id = update
                .value
                .parse()
                .map_err(|_| RepoError::BadInput("invalid author_id".into()))?;
        }
        "reviewer_id" => {
            verification.reviewer_id = update
                .value
                .parse()
                .map_err(|_| RepoError::BadInput("invalid reviewer_id".into()))?;
        }
        other => {
            return Err(ApiError::from(RepoError::BadInput(format!(
                "unsupported field '{other}'"
            ))))
        }
    }

    let status_changed = field_key == "status_id";
    let payload = NewVerification {
        id: Some(verification.id),
        reference_code: verification.reference_code.clone(),
        name: verification.name.clone(),
        description: verification.description.clone(),
        source: verification.source.clone(),
        status_id: verification.status_id,
        parent_id: verification.parent_id,
        project_id: verification.project_id,
        verification_method_id: verification.verification_method_id,
        author_id: verification.author_id,
        reviewer_id: verification.reviewer_id,
    };

    service.update(user, id, payload)?;
    if status_changed {
        state
            .repo_write()
            .record_verification_status_audit(id, user.id)
            .map_err(ApiError::from)?;
    }

    Ok(json!({
        "success": true,
        "message": "Field updated successfully"
    }))
}

#[post("/verifications/<id>/field", data = "<update>")]
pub async fn update_field(
    user: ApiUser,
    id: i32,
    state: &State<AppState>,
    update: Json<FieldUpdateRequest>,
) -> ApiResult<Value> {
    apply_verification_field_update(state, user.user(), id, update.into_inner(), None)
}

/// Project-scoped verification field update (session or Bearer).
#[post("/projects/<project_id>/verifications/<id>/field", data = "<update>")]
pub async fn update_field_by_project(
    access: ProjectAccessOrBearer,
    project_id: i32,
    id: i32,
    state: &State<AppState>,
    update: Json<FieldUpdateRequest>,
) -> ApiResult<Value> {
    apply_verification_field_update(
        state,
        access.user(),
        id,
        update.into_inner(),
        Some(project_id),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::test_session_cookie_for;

    fn auth_cookie_for(
        client: &rocket::local::asynchronous::Client,
        user_id: i32,
    ) -> rocket::http::Cookie<'static> {
        let state = client.rocket().state::<TestState>().unwrap();
        test_session_cookie_for(state, user_id)
    }
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::ContentType;
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
                    create,
                    delete,
                    update_field,
                    update_field_by_project
                ],
            );
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_cookie(client: &rocket::local::asynchronous::Client) -> rocket::http::Cookie<'static> {
        auth_cookie_for(client, ADMIN_ID)
    }

    fn sample_verification(name: &str) -> Value {
        json!({
            "id": null,
            "name": name,
            "description": format!("{name} description"),
            "source": "manual",
            "status_id": 1,
            "reference_code": "VER-001",
            "parent_id": null,
            "project_id": 1,
            "author_id": 1,
            "reviewer_id": 1
        })
    }

    #[rocket::async_test]
    async fn list_returns_empty_array() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .get("/api/verifications")
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let items: Vec<Verification> = response.into_json().await.unwrap();
        assert!(items.is_empty());
    }

    #[rocket::async_test]
    async fn create_returns_identifier() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .post("/api/verifications")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(sample_verification("Baseline").to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("status"), Some(&Value::from("ok")));
        assert_eq!(payload.get("id"), Some(&Value::from(1)));
    }

    #[rocket::async_test]
    async fn update_field_changes_name() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_response = client
            .post("/api/verifications")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(sample_verification("Scenario").to_string())
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let response = client
            .post(format!("/api/verifications/{id}/field"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(
                json!({
                    "field": "name",
                    "value": "Updated"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("success"), Some(&Value::from(true)));

        let get_response = client
            .get(format!("/api/verifications/{id}"))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        let verification: Verification = get_response.into_json().await.unwrap();
        assert_eq!(verification.name, "Updated");
    }

    #[rocket::async_test]
    async fn delete_removes_verification() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_response = client
            .post("/api/verifications")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(sample_verification("Disposable").to_string())
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let delete_response = client
            .delete(format!("/api/verifications/{id}"))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(delete_response.status(), Status::NoContent);

        let not_found = client
            .get(format!("/api/verifications/{id}"))
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(not_found.status(), Status::NotFound);
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::api::prelude::*;
use crate::models::{NewRequirementStatus, RequirementStatus};
use crate::services::StatusService;

#[get("/status")]
pub async fn list_requirement_statuses(
    user: ApiUser,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<RequirementStatus>>> {
    let service = StatusService::new(state.inner());
    let mut statuses = service.list_requirement_statuses()?;
    let repo = state.repo_read();
    statuses.retain(|status| {
        crate::permissions::has_permission(
            &*repo,
            user.user(),
            status.project_id,
            Permission::ViewRequirements,
        )
    });
    Ok(Json(statuses))
}

#[get("/status/<id>")]
pub async fn get_requirement_status(
    user: ApiUser,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Value>> {
    let service = StatusService::new(state.inner());
    let status = service.get_requirement_status(id)?;
    require_project_permission(
        state,
        user.user(),
        status.project_id,
        Permission::ViewRequirements,
    )?;

    Ok(Json(json!({
        "id": status.id,
        "title": status.title,
        "description": status.description,
        "tag": status.tag,
        "project_id": status.project_id,
        "is_system": status.is_system,
        "tag_color": status.tag_color,
    })))
}

#[put("/status/<id>", data = "<payload>")]
pub async fn update_requirement_status(
    user: ApiUser,
    id: i32,
    state: &State<AppState>,
    payload: Json<NewRequirementStatus>,
) -> ApiResult<Value> {
    let service = StatusService::new(state.inner());
    let current = service.get_requirement_status(id)?;
    require_project_permission(
        state,
        user.user(),
        current.project_id,
        Permission::ManageProjectConfiguration,
    )?;
    let payload = payload.into_inner();
    if payload.project_id != current.project_id {
        return Err(ApiError::UnprocessableEntity(
            "requirement status project cannot be changed".into(),
        ));
    }
    service.update_requirement_status(id, &payload)?;
    Ok(json!({ "status": "ok" }))
}

#[delete("/status/<id>")]
pub async fn delete_requirement_status(
    user: ApiUser,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Status> {
    let service = StatusService::new(state.inner());
    let status = service.get_requirement_status(id)?;
    require_project_permission(
        state,
        user.user(),
        status.project_id,
        Permission::ManageProjectConfiguration,
    )?;
    service.delete_requirement_status(id)?;
    Ok(Status::NoContent)
}

#[post("/status", data = "<payload>")]
pub async fn create_requirement_status(
    user: ApiUser,
    state: &State<AppState>,
    payload: Json<NewRequirementStatus>,
) -> ApiResult<(Status, Value)> {
    let service = StatusService::new(state.inner());
    let payload = payload.into_inner();
    require_project_permission(
        state,
        user.user(),
        payload.project_id,
        Permission::ManageProjectConfiguration,
    )?;
    let id = service.create_requirement_status(payload)?;

    Ok((Status::Created, json!({ "status": "ok", "id": id })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::test_session_cookie_for;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::ContentType;
    use rocket::local::asynchronous::Client;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    fn state_from_repo(mut repo: DieselRepoMock) -> TestState {
        let mut admin = DieselRepoMock::make_user(1, "admin", "");
        admin.is_admin = true;
        repo.users.insert(1, admin);
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    fn admin_cookie(client: &Client) -> rocket::http::Cookie<'static> {
        let state = client.rocket().state::<TestState>().unwrap();
        test_session_cookie_for(state, 1)
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build().manage(state_from_repo(repo)).mount(
            "/api",
            routes![
                list_requirement_statuses,
                get_requirement_status,
                create_requirement_status,
                update_requirement_status,
                delete_requirement_status,
            ],
        );
        Client::tracked(rocket).await.unwrap()
    }

    #[rocket::async_test]
    async fn list_returns_seeded_statuses() {
        let mut repo = DieselRepoMock::default();
        let mut statuses = HashMap::new();
        statuses.insert(
            1,
            RequirementStatus {
                id: 1,
                title: "Draft".into(),
                description: "Initial".into(),
                tag: "DR".into(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            },
        );
        repo.requirement_statuses = statuses;

        let client = client_with_repo(repo).await;
        let response = client
            .get("/api/status")
            .private_cookie(admin_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let items: Vec<RequirementStatus> = response.into_json().await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Draft");
    }

    #[rocket::async_test]
    async fn get_returns_specific_status() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(
            5,
            RequirementStatus {
                id: 5,
                title: "Approved".into(),
                description: "Ready".into(),
                tag: "AP".into(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            },
        );

        let client = client_with_repo(repo).await;
        let response = client
            .get("/api/status/5")
            .private_cookie(admin_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let value: Value = response.into_json().await.unwrap();
        assert_eq!(value.get("title"), Some(&Value::from("Approved")));
        assert_eq!(value.get("id"), Some(&Value::from(5)));
    }

    #[rocket::async_test]
    async fn create_returns_created_identifier() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .post("/api/status")
            .header(ContentType::JSON)
            .private_cookie(admin_cookie(&client))
            .body(
                json!({
                    "title": "In Review",
                    "description": "Under evaluation",
                    "tag": "IR",
                    "project_id": 1
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("status"), Some(&Value::from("ok")));
        assert_eq!(payload.get("id"), Some(&Value::from(1)));
    }
}

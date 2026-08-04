// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! REST API for project-scoped saved views (issue #110).

use rocket::serde::json::Json;
use rocket::State;

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::models::{SavedView, SavedViewPayload, User};
use crate::permissions::{Permission, ROLE_ADMIN};
use crate::repository::{ProjectMembersRepository, ProjectsRepository};
use crate::services::SavedViewService;

fn can_mutate_view(state: &AppState, user: &User, view: &SavedView) -> ApiResult<bool> {
    if user.is_admin || view.owner_id == user.id {
        return Ok(true);
    }
    let members = state
        .repo_read()
        .get_members_by_project(view.project_id)
        .map_err(ApiError::from)?;
    Ok(members
        .iter()
        .any(|m| m.user_id == user.id && m.role == ROLE_ADMIN))
}

fn ensure_visible(_state: &AppState, user: &User, view: &SavedView) -> ApiResult<()> {
    if view.visibility == "shared" || view.owner_id == user.id || user.is_admin {
        return Ok(());
    }
    Err(ApiError::NotFound("saved view not found".into()))
}

#[get("/projects/<project_id>/saved_views")]
pub async fn list(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<SavedView>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let _ = state
        .repo_read()
        .get_project_by_id(project_id)
        .map_err(ApiError::from)?;
    let service = SavedViewService::new(state.inner());
    let list = service.list_for_user(project_id, access.user().id)?;
    Ok(Json(list))
}

#[get("/projects/<project_id>/saved_views/<view_id>")]
pub async fn get(
    access: ProjectAccessOrBearer,
    project_id: i32,
    view_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<SavedView>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = SavedViewService::new(state.inner());
    let view = service.get_by_id(view_id)?;
    if view.project_id != project_id {
        return Err(ApiError::NotFound("saved view not in project".into()));
    }
    ensure_visible(state.inner(), access.user(), &view)?;
    Ok(Json(view))
}

#[post("/projects/<project_id>/saved_views", data = "<payload>")]
pub async fn create(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
    payload: Json<SavedViewPayload>,
) -> ApiResult<Json<SavedView>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = SavedViewService::new(state.inner());
    let view = service.create(project_id, access.user().id, payload.into_inner())?;
    Ok(Json(view))
}

#[patch("/projects/<project_id>/saved_views/<view_id>", data = "<payload>")]
pub async fn update(
    access: ProjectAccessOrBearer,
    project_id: i32,
    view_id: i32,
    state: &State<AppState>,
    payload: Json<SavedViewPayload>,
) -> ApiResult<Json<SavedView>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = SavedViewService::new(state.inner());
    let existing = service.get_by_id(view_id)?;
    if existing.project_id != project_id {
        return Err(ApiError::NotFound("saved view not in project".into()));
    }
    if !can_mutate_view(state.inner(), access.user(), &existing)? {
        return Err(ApiError::Forbidden(
            "only the owner or a project admin can update this view".into(),
        ));
    }
    if existing.locked {
        return Err(ApiError::Conflict(
            "Saved views used in a baseline are immutable".into(),
        ));
    }
    let view = service.update(view_id, payload.into_inner())?;
    Ok(Json(view))
}

#[delete("/projects/<project_id>/saved_views/<view_id>")]
pub async fn delete(
    access: ProjectAccessOrBearer,
    project_id: i32,
    view_id: i32,
    state: &State<AppState>,
) -> ApiResult<Value> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = SavedViewService::new(state.inner());
    let existing = service.get_by_id(view_id)?;
    if existing.project_id != project_id {
        return Err(ApiError::NotFound("saved view not in project".into()));
    }
    if !can_mutate_view(state.inner(), access.user(), &existing)? {
        return Err(ApiError::Forbidden(
            "only the owner or a project admin can delete this view".into(),
        ));
    }
    if existing.locked {
        return Err(ApiError::Conflict(
            "Saved views used in a baseline are immutable".into(),
        ));
    }
    service.delete(view_id)?;
    Ok(json!({ "status": "ok" }))
}

#[cfg(all(test, feature = "test-helpers"))]
mod tests {
    use super::{create, delete, get, list, update};
    use crate::app::AppState;
    use crate::auth::session::test_session_cookie_for;
    use crate::models::{Project, ProjectMember};
    use crate::permissions::{ROLE_AUTHOR, ROLE_VIEWER};
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::repository::CacheRepository;
    use crate::status_enums::ProjectStatus;
    use chrono::{NaiveDate, NaiveDateTime};
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::json;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
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
        let rocket = rocket::build()
            .manage(state_from_repo(repo))
            .mount("/api", routes![list, get, create, update, delete]);
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_cookie_for(client: &Client, user_id: i32) -> rocket::http::Cookie<'static> {
        let state = client.rocket().state::<TestState>().unwrap();
        test_session_cookie_for(state, user_id)
    }

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();
        let mut u1 = DieselRepoMock::make_user(1, "alice", "password");
        u1.is_admin = false;
        let mut u2 = DieselRepoMock::make_user(2, "bob", "password");
        u2.is_admin = false;
        repo.users.insert(1, u1);
        repo.users.insert(2, u2);
        repo.projects.insert(
            1,
            Project {
                id: 1,
                name: "P".into(),
                description: None,
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(1),
                slug: "p".into(),
                group_id: None,
            },
        );
        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 1,
            role: ROLE_AUTHOR,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: 2,
            role: ROLE_VIEWER,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo
    }

    #[rocket::async_test]
    async fn create_list_visibility_and_mutate() {
        let client = client_with_repo(base_repo()).await;
        let alice = auth_cookie_for(&client, 1);
        let bob = auth_cookie_for(&client, 2);

        let body = json!({
            "name": "My drafts",
            "description": null,
            "visibility": "private",
            "definition": { "filters": { "status_id": 1, "q": "alpha" } }
        });
        let res = client
            .post("/api/projects/1/saved_views")
            .header(ContentType::JSON)
            .private_cookie(alice.clone())
            .body(body.to_string())
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);
        let created: serde_json::Value = res.into_json().await.unwrap();
        let view_id = created["id"].as_i64().unwrap();

        let shared = json!({
            "name": "Shared open",
            "visibility": "shared",
            "definition": { "filters": { "q": "" } }
        });
        let res = client
            .post("/api/projects/1/saved_views")
            .header(ContentType::JSON)
            .private_cookie(alice.clone())
            .body(shared.to_string())
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);

        let res = client
            .get("/api/projects/1/saved_views")
            .private_cookie(bob.clone())
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);
        let list: Vec<serde_json::Value> = res.into_json().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["name"], "Shared open");

        let res = client
            .get("/api/projects/1/saved_views")
            .private_cookie(alice.clone())
            .dispatch()
            .await;
        let list: Vec<serde_json::Value> = res.into_json().await.unwrap();
        assert_eq!(list.len(), 2);

        let res = client
            .delete(format!("/api/projects/1/saved_views/{view_id}"))
            .private_cookie(bob)
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Forbidden);

        let res = client
            .delete(format!("/api/projects/1/saved_views/{view_id}"))
            .private_cookie(alice)
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);
    }

    #[rocket::async_test]
    async fn locked_view_rejects_update() {
        let mut repo = base_repo();
        let now = timestamp();
        repo.saved_views.push(crate::models::SavedView {
            id: 9,
            project_id: 1,
            owner_id: 1,
            name: "Locked".into(),
            description: None,
            visibility: "private".into(),
            definition: json!({
                "version": 1,
                "entity": "requirements",
                "filters": {},
                "sort": {"column": null, "dir": "asc"},
                "columns": null,
                "ui": {"view_mode": "table", "page_size": 25}
            }),
            locked: true,
            locked_at: Some(now),
            created_at: now,
            updated_at: now,
        });
        repo.next_saved_view_id = 10;
        let client = client_with_repo(repo).await;
        let alice = auth_cookie_for(&client, 1);
        let res = client
            .patch("/api/projects/1/saved_views/9")
            .header(ContentType::JSON)
            .private_cookie(alice)
            .body(
                json!({
                    "name": "Nope",
                    "visibility": "private",
                    "definition": {}
                })
                .to_string(),
            )
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Conflict);
    }
}

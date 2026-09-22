// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! REST API for project creation.

use rocket::serde::Deserialize;

use crate::api::prelude::*;
use crate::auth::guards::ApiUserOrBearer;
use crate::models::NewProject;
use crate::namespaces::project_base_path;
use crate::repository::GroupsRepository;
use crate::services::project_service::ProjectService;
use crate::status_enums::ProjectStatus;

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub group_id: Option<i32>,
}

/// POST /api/projects — create a new project.
#[post("/projects", data = "<body>")]
pub async fn create(
    auth: ApiUserOrBearer,
    state: &State<AppState>,
    body: Json<CreateProjectRequest>,
) -> ApiResult<Json<Value>> {
    let user = auth.user();
    if let Some(group_id) = body.group_id {
        // Resolve first so a missing namespace is distinguished from an existing
        // group in which the caller is not allowed to create projects.
        state
            .repo_read()
            .get_group_by_id(group_id)
            .map_err(ApiError::from)?;
        require_group_permission(state, user, group_id, GroupPermission::ManageProjects)?;
    }

    let payload = NewProject {
        name: body.name.clone(),
        description: body.description.clone(),
        owner_id: Some(user.id),
        status: ProjectStatus::Active,
        group_id: body.group_id,
    };
    let service = ProjectService::new(state.inner());
    let id = service.create(user, payload).map_err(ApiError::from)?;
    let project = service.get_by_id(id).map_err(ApiError::from)?;
    Ok(Json(json!({
        "id": project.id,
        "name": project.name,
        "slug": project.slug,
        "description": project.description,
        "group_id": project.group_id,
        "project_base_path": project_base_path(&project),
        "status": "ok",
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::test_session_cookie_for;
    use crate::models::{Group, GroupMember};
    use crate::permissions::{
        GROUP_ROLE_CONTRIBUTOR, GROUP_ROLE_MAINTAINER, GROUP_ROLE_OWNER, GROUP_ROLE_VIEWER,
    };
    use crate::repository::{
        diesel_repo_mock::DieselRepoMock, CacheRepository, LookupRepository,
        ProjectMembersRepository, ProjectsRepository,
    };
    use chrono::{NaiveDate, NaiveDateTime};
    use rocket::http::{ContentType, Cookie, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::{json, Value};
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn group(id: i32) -> Group {
        Group {
            id,
            name: format!("Group {id}"),
            slug: format!("group-{id}"),
            description: None,
            owner_id: Some(1),
            created_at: timestamp(),
            updated_at: timestamp(),
        }
    }

    fn repo_with_user(is_admin: bool) -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();
        let mut user = DieselRepoMock::make_user(1, "alice", "");
        user.is_admin = is_admin;
        repo.users.insert(user.id, user);
        repo
    }

    fn add_group_membership(repo: &mut DieselRepoMock, role: i32) {
        repo.groups.insert(10, group(10));
        repo.group_members.push(GroupMember {
            group_id: 10,
            user_id: 1,
            role,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let state = AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        };
        Client::tracked(rocket::build().manage(state).mount("/api", routes![create]))
            .await
            .expect("client")
    }

    fn session_cookie(client: &Client) -> Cookie<'static> {
        let state = client.rocket().state::<TestState>().unwrap();
        test_session_cookie_for(state, 1)
    }

    async fn create_project(
        client: &Client,
        group_id: Option<i32>,
    ) -> rocket::local::asynchronous::LocalResponse<'_> {
        client
            .post("/api/projects")
            .header(ContentType::JSON)
            .private_cookie(session_cookie(client))
            .body(
                json!({
                    "name": "Flight Controller",
                    "description": "Safety-critical controls",
                    "group_id": group_id,
                })
                .to_string(),
            )
            .dispatch()
            .await
    }

    #[rocket::async_test]
    async fn authenticated_user_creates_fully_initialized_personal_project() {
        let client = client_with_repo(repo_with_user(false)).await;

        let response = create_project(&client, None).await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.expect("json response");
        let project_id = payload["id"].as_i64().unwrap() as i32;
        assert_eq!(payload["group_id"], Value::Null);
        assert_eq!(payload["project_base_path"], "/flight-controller");

        let state = client.rocket().state::<TestState>().unwrap();
        let repo = state.repo.read().unwrap();
        let project = repo.get_project_by_id(project_id).unwrap();
        assert_eq!(project.owner_id, Some(1));
        assert_eq!(project.group_id, None);
        let members = repo.get_members_by_project(project_id).unwrap();
        assert!(members
            .iter()
            .any(|member| member.user_id == 1 && member.role == 1));
        assert_eq!(
            repo.get_requirement_status_by_project(project_id)
                .unwrap()
                .len(),
            6
        );
        assert_eq!(
            repo.get_verification_status_by_project(project_id)
                .unwrap()
                .len(),
            4
        );
        assert_eq!(
            repo.get_verification_methods_by_project(project_id)
                .unwrap()
                .len(),
            4
        );
        assert_eq!(repo.get_categories_by_project(project_id).unwrap().len(), 1);
        assert_eq!(
            repo.get_applicability_by_project(project_id).unwrap().len(),
            1
        );
    }

    #[rocket::async_test]
    async fn group_owner_and_maintainer_can_create_projects() {
        for role in [GROUP_ROLE_OWNER, GROUP_ROLE_MAINTAINER] {
            let mut repo = repo_with_user(false);
            add_group_membership(&mut repo, role);
            let client = client_with_repo(repo).await;

            let response = create_project(&client, Some(10)).await;

            assert_eq!(response.status(), Status::Ok, "role {role}");
            let payload: Value = response.into_json().await.expect("json response");
            assert_eq!(payload["group_id"], 10);
        }
    }

    #[rocket::async_test]
    async fn contributor_viewer_and_non_member_cannot_create_group_projects() {
        for role in [Some(GROUP_ROLE_CONTRIBUTOR), Some(GROUP_ROLE_VIEWER), None] {
            let mut repo = repo_with_user(false);
            repo.groups.insert(10, group(10));
            if let Some(role) = role {
                add_group_membership(&mut repo, role);
            }
            let client = client_with_repo(repo).await;

            let response = create_project(&client, Some(10)).await;

            assert_eq!(response.status(), Status::Forbidden, "role {role:?}");
            let state = client.rocket().state::<TestState>().unwrap();
            assert!(state
                .repo
                .read()
                .unwrap()
                .get_projects_all()
                .unwrap()
                .is_empty());
        }
    }

    #[rocket::async_test]
    async fn arbitrary_or_missing_group_id_cannot_bypass_authorization() {
        let mut repo = repo_with_user(false);
        repo.groups.insert(10, group(10));
        let client = client_with_repo(repo).await;

        assert_eq!(
            create_project(&client, Some(10)).await.status(),
            Status::Forbidden
        );
        assert_eq!(
            create_project(&client, Some(999)).await.status(),
            Status::NotFound
        );
        assert_eq!(
            create_project(&client, Some(0)).await.status(),
            Status::NotFound
        );
    }

    #[rocket::async_test]
    async fn site_admin_can_create_project_in_any_existing_group() {
        let mut repo = repo_with_user(true);
        repo.groups.insert(10, group(10));
        let client = client_with_repo(repo).await;

        assert_eq!(create_project(&client, Some(10)).await.status(), Status::Ok);
    }
}

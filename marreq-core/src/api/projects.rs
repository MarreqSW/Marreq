// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! REST API for project creation and editing.

use rocket::serde::{Deserialize, Deserializer};

use crate::api::prelude::*;
use crate::auth::guards::{ApiUserOrBearer, ProjectAccess, ProjectAccessOrBearer};
use crate::models::{NewProject, Project, UpdateProject};
use crate::namespaces::project_base_path;
use crate::repository::errors::RepoError;
use crate::repository::{GroupsRepository, ProjectMembersRepository};
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

/// Keeps "field absent" (`None`) apart from "field is null" (`Some(None)`) for PATCH bodies.
fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// Body of `PATCH /api/projects/<id>`. Absent fields stay unchanged. The slug is not
/// editable (project URLs stay stable), so unknown fields such as `slug` are rejected.
#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", deny_unknown_fields)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    /// `null` or an empty string clears the description.
    #[serde(default, deserialize_with = "present")]
    pub description: Option<Option<String>>,
    pub status: Option<ProjectStatus>,
    pub owner_id: Option<i32>,
    /// `null` moves the project to the personal namespace (no group).
    #[serde(default, deserialize_with = "present")]
    pub group_id: Option<Option<i32>>,
}

fn project_json(project: &Project) -> Value {
    json!({
        "id": project.id,
        "name": project.name,
        "slug": project.slug,
        "description": project.description,
        "status": project.status,
        "owner_id": project.owner_id,
        "group_id": project.group_id,
        "creation_date": project.creation_date,
        "update_date": project.update_date,
        "project_base_path": project_base_path(project),
    })
}

/// PATCH /api/projects/<project_id> — edit name, description, status, owner or group.
///
/// Requires `ManageProjectConfiguration` (project Admin) or instance admin. Moving the
/// project to another group also requires `ManageProjects` in both the current and the
/// target group. The change is recorded in the audit log with old and new values.
#[patch("/projects/<project_id>", data = "<body>")]
pub async fn update(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
    body: Json<UpdateProjectRequest>,
) -> ApiResult<Json<Value>> {
    let user = access.user();
    require_project_permission(
        state,
        user,
        project_id,
        Permission::ManageProjectConfiguration,
    )?;
    let body = body.into_inner();
    let service = ProjectService::new(state.inner());
    let current = service.get_by_id(project_id).map_err(ApiError::from)?;

    if let Some(owner_id) = body.owner_id {
        let members = state
            .repo_read()
            .get_members_by_project(project_id)
            .map_err(ApiError::from)?;
        if !members.iter().any(|member| member.user_id == owner_id) {
            return Err(ApiError::UnprocessableEntity(
                "owner_id must be a member of the project".into(),
            ));
        }
    }

    let group_id = body.group_id.unwrap_or(current.group_id);
    if group_id != current.group_id {
        if let Some(from) = current.group_id {
            require_group_permission(state, user, from, GroupPermission::ManageProjects)?;
        }
        if let Some(to) = group_id {
            state
                .repo_read()
                .get_group_by_id(to)
                .map_err(ApiError::from)?;
            require_group_permission(state, user, to, GroupPermission::ManageProjects)?;
        }
    }

    let description = match body.description {
        None => current.description.clone(),
        Some(value) => value.filter(|text| !text.trim().is_empty()),
    };
    let payload = UpdateProject {
        name: body.name.unwrap_or_else(|| current.name.clone()),
        description,
        owner_id: body.owner_id.or(current.owner_id),
        status: Some(body.status.unwrap_or(current.status)),
        slug: None,
        group_id,
    };
    let project = service
        .update(user, project_id, payload)
        .map_err(ApiError::from)?;
    Ok(Json(project_json(&project)))
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", deny_unknown_fields)]
pub struct DeleteProjectRequest {
    /// Must equal the project's slug, so a project is never deleted by mistake.
    pub confirm_slug: String,
}

/// DELETE /api/projects/<project_id> — delete the project and everything in it (issue #349).
///
/// Only the project owner or an instance administrator, and only from a browser
/// session (API tokens and MCP clients cannot delete projects). The body must
/// repeat the project's slug. The deletion is recorded in the audit log.
#[delete("/projects/<project_id>", data = "<body>")]
pub async fn delete(
    access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
    body: Json<DeleteProjectRequest>,
) -> ApiResult<Status> {
    let user = access.user();
    let service = ProjectService::new(state.inner());
    let project = service.get_by_id(project_id).map_err(ApiError::from)?;
    if !ProjectService::can_delete(user, &project) {
        return Err(ApiError::Forbidden(
            "only the project owner or an instance administrator can delete this project".into(),
        ));
    }
    if body.confirm_slug.trim() != project.slug {
        return Err(ApiError::BadRequest(
            "confirm_slug does not match the project".into(),
        ));
    }
    service.delete(user, project_id).map_err(ApiError::from)?;
    Ok(Status::NoContent)
}

/// POST /api/projects/<project_id>/archive — make the project read-only for
/// everyone, site administrators included (issue #381). It stays listed,
/// readable and exportable.
///
/// Only the project owner or an instance administrator, from a browser
/// session (like deletion). Recorded in the audit log.
#[post("/projects/<project_id>/archive")]
pub async fn archive(
    access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Project>> {
    let project = ProjectService::new(state.inner())
        .archive(access.user(), project_id)
        .map_err(archive_error)?;
    Ok(Json(project))
}

/// POST /api/projects/<project_id>/unarchive — make an archived project
/// editable again, unchanged. Same rule as archiving.
#[post("/projects/<project_id>/unarchive")]
pub async fn unarchive(
    access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Project>> {
    let project = ProjectService::new(state.inner())
        .unarchive(access.user(), project_id)
        .map_err(archive_error)?;
    Ok(Json(project))
}

fn archive_error(error: RepoError) -> ApiError {
    match error {
        RepoError::Unauthorized => ApiError::Forbidden(
            "only the project owner or an instance administrator can archive or unarchive this project"
                .into(),
        ),
        other => other.into(),
    }
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
        CacheRepository, LookupRepository, ProjectMembersRepository, ProjectsRepository,
        diesel_repo_mock::DieselRepoMock,
    };
    use chrono::{NaiveDate, NaiveDateTime};
    use rocket::http::{ContentType, Cookie, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::{Value, json};
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
        assert!(
            members
                .iter()
                .any(|member| member.user_id == 1 && member.role == 1)
        );
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
            assert!(
                state
                    .repo
                    .read()
                    .unwrap()
                    .get_projects_all()
                    .unwrap()
                    .is_empty()
            );
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

    // ── PATCH /api/projects/<id> (issue #289) ───────────────────────────

    const PROJECT: i32 = 7;
    const ADMIN_MEMBER: i32 = 2; // project Admin
    const VIEWER_MEMBER: i32 = 3;
    const OUTSIDER: i32 = 4;
    const INSTANCE_ADMIN: i32 = 5; // not a member
    const AUTHOR_MEMBER: i32 = 6;

    fn edit_repo() -> DieselRepoMock {
        use crate::models::{Project, ProjectMember};
        use crate::permissions::{ROLE_ADMIN, ROLE_AUTHOR, ROLE_VIEWER};
        let mut repo = DieselRepoMock::default();
        for (id, name) in [
            (ADMIN_MEMBER, "bob"),
            (VIEWER_MEMBER, "carol"),
            (OUTSIDER, "dave"),
            (INSTANCE_ADMIN, "root"),
            (AUTHOR_MEMBER, "eve"),
        ] {
            let mut user = DieselRepoMock::make_user(id, name, "");
            user.is_admin = id == INSTANCE_ADMIN;
            repo.users.insert(id, user);
        }
        repo.groups.insert(10, group(10));
        repo.groups.insert(20, group(20));
        repo.projects.insert(
            PROJECT,
            Project {
                id: PROJECT,
                name: "Space Project".into(),
                description: Some("Satellite requirements".into()),
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(ADMIN_MEMBER),
                slug: "space-project".into(),
                group_id: Some(10),
                archived_at: None,
                archived_by: None,
            },
        );
        for (user_id, role) in [
            (ADMIN_MEMBER, ROLE_ADMIN),
            (VIEWER_MEMBER, ROLE_VIEWER),
            (AUTHOR_MEMBER, ROLE_AUTHOR),
        ] {
            repo.project_members.push(ProjectMember {
                project_id: PROJECT,
                user_id,
                role,
                created_at: timestamp(),
                updated_at: timestamp(),
            });
        }
        repo
    }

    fn grant_group(repo: &mut DieselRepoMock, group_id: i32, user_id: i32, role: i32) {
        repo.group_members.push(GroupMember {
            group_id,
            user_id,
            role,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
    }

    async fn edit_client(repo: DieselRepoMock) -> Client {
        let state = AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        };
        Client::tracked(rocket::build().manage(state).mount("/api", routes![update]))
            .await
            .expect("client")
    }

    async fn patch_as(client: &Client, user_id: Option<i32>, body: Value) -> (Status, Value) {
        let mut request = client
            .patch(format!("/api/projects/{PROJECT}"))
            .header(ContentType::JSON)
            .body(body.to_string());
        if let Some(user_id) = user_id {
            let state = client.rocket().state::<TestState>().unwrap();
            request = request.private_cookie(test_session_cookie_for(state, user_id));
        }
        let response = request.dispatch().await;
        let status = response.status();
        let json = response.into_json().await.unwrap_or(Value::Null);
        (status, json)
    }

    fn stored(client: &Client) -> crate::models::Project {
        let state = client.rocket().state::<TestState>().unwrap();
        state
            .repo
            .read()
            .unwrap()
            .get_project_by_id(PROJECT)
            .unwrap()
    }

    #[rocket::async_test]
    async fn patch_changes_only_the_fields_sent() {
        let client = edit_client(edit_repo()).await;
        let (status, body) = patch_as(
            &client,
            Some(ADMIN_MEMBER),
            json!({ "name": "Space Project II" }),
        )
        .await;
        assert_eq!(status, Status::Ok, "{body}");
        assert_eq!(body["name"], "Space Project II");
        assert_eq!(body["slug"], "space-project", "slug never changes");
        assert_eq!(body["project_base_path"], "/space-project");
        let project = stored(&client);
        assert_eq!(project.name, "Space Project II");
        assert_eq!(
            project.description.as_deref(),
            Some("Satellite requirements")
        );
        assert_eq!(project.status, ProjectStatus::Active);
        assert_eq!(project.owner_id, Some(ADMIN_MEMBER));
        assert_eq!(
            project.group_id,
            Some(10),
            "absent group_id keeps the group"
        );
    }

    #[rocket::async_test]
    async fn patch_sets_status_and_owner_and_clears_description() {
        let client = edit_client(edit_repo()).await;
        let (status, body) = patch_as(
            &client,
            Some(ADMIN_MEMBER),
            json!({ "status": "OnHold", "owner_id": AUTHOR_MEMBER, "description": null }),
        )
        .await;
        assert_eq!(status, Status::Ok, "{body}");
        assert_eq!(body["status"], "OnHold");
        let project = stored(&client);
        assert_eq!(project.status, ProjectStatus::OnHold);
        assert_eq!(project.owner_id, Some(AUTHOR_MEMBER));
        assert_eq!(project.description, None);

        let (status, _) =
            patch_as(&client, Some(ADMIN_MEMBER), json!({ "description": "  " })).await;
        assert_eq!(status, Status::Ok);
        assert_eq!(stored(&client).description, None, "blank clears too");
    }

    #[rocket::async_test]
    async fn patch_rejects_invalid_input() {
        let client = edit_client(edit_repo()).await;
        for (body, expected) in [
            (json!({ "name": "" }), Status::BadRequest),
            (json!({ "name": "X" }), Status::BadRequest),
            (json!({ "name": "x".repeat(101) }), Status::BadRequest),
            (
                json!({ "description": "x".repeat(1001) }),
                Status::BadRequest,
            ),
            (json!({ "status": "Paused" }), Status::UnprocessableEntity),
            (json!({ "slug": "new-url" }), Status::UnprocessableEntity),
            (json!({ "owner_id": OUTSIDER }), Status::UnprocessableEntity),
        ] {
            let (status, _) = patch_as(&client, Some(ADMIN_MEMBER), body.clone()).await;
            assert_eq!(status, expected, "{body}");
        }
        assert_eq!(stored(&client).name, "Space Project", "nothing was saved");
    }

    #[rocket::async_test]
    async fn patch_requires_project_admin_or_instance_admin() {
        let client = edit_client(edit_repo()).await;
        let rename = json!({ "name": "Renamed" });
        assert_eq!(
            patch_as(&client, None, rename.clone()).await.0,
            Status::Unauthorized
        );
        assert_eq!(
            patch_as(&client, Some(VIEWER_MEMBER), rename.clone())
                .await
                .0,
            Status::Forbidden
        );
        assert_eq!(
            patch_as(&client, Some(AUTHOR_MEMBER), rename.clone())
                .await
                .0,
            Status::Forbidden
        );
        assert_eq!(
            patch_as(&client, Some(OUTSIDER), rename.clone()).await.0,
            Status::Forbidden
        );
        assert_eq!(
            patch_as(&client, Some(INSTANCE_ADMIN), rename).await.0,
            Status::Ok
        );
    }

    #[rocket::async_test]
    async fn group_move_needs_manage_projects_in_both_groups() {
        // Project admin without group rights.
        let client = edit_client(edit_repo()).await;
        let to_twenty = json!({ "group_id": 20 });
        assert_eq!(
            patch_as(&client, Some(ADMIN_MEMBER), to_twenty.clone())
                .await
                .0,
            Status::Forbidden
        );

        // Rights in the source group only.
        let mut repo = edit_repo();
        grant_group(&mut repo, 10, ADMIN_MEMBER, GROUP_ROLE_MAINTAINER);
        grant_group(&mut repo, 20, ADMIN_MEMBER, GROUP_ROLE_CONTRIBUTOR);
        let client = edit_client(repo).await;
        assert_eq!(
            patch_as(&client, Some(ADMIN_MEMBER), to_twenty.clone())
                .await
                .0,
            Status::Forbidden
        );
        assert_eq!(
            patch_as(&client, Some(ADMIN_MEMBER), json!({ "group_id": 999 }))
                .await
                .0,
            Status::NotFound
        );
        assert_eq!(stored(&client).group_id, Some(10));

        // Rights in both groups; then back to the personal namespace.
        let mut repo = edit_repo();
        grant_group(&mut repo, 10, ADMIN_MEMBER, GROUP_ROLE_OWNER);
        grant_group(&mut repo, 20, ADMIN_MEMBER, GROUP_ROLE_MAINTAINER);
        let client = edit_client(repo).await;
        let (status, body) = patch_as(&client, Some(ADMIN_MEMBER), to_twenty).await;
        assert_eq!(status, Status::Ok, "{body}");
        assert_eq!(stored(&client).group_id, Some(20));
        let (status, _) = patch_as(&client, Some(ADMIN_MEMBER), json!({ "group_id": null })).await;
        assert_eq!(status, Status::Ok);
        assert_eq!(stored(&client).group_id, None);
        assert_eq!(
            stored(&client).slug,
            "space-project",
            "URL unchanged by moves"
        );
    }

    #[rocket::async_test]
    async fn instance_admin_moves_groups_without_group_roles() {
        let client = edit_client(edit_repo()).await;
        let (status, _) = patch_as(&client, Some(INSTANCE_ADMIN), json!({ "group_id": 20 })).await;
        assert_eq!(status, Status::Ok);
        assert_eq!(stored(&client).group_id, Some(20));
    }

    // ── DELETE /api/projects/<id> (issue #349) ──────────────────────────

    const SECOND_ADMIN: i32 = 8; // project Admin, not the owner

    fn delete_repo() -> DieselRepoMock {
        use crate::models::ProjectMember;
        use crate::permissions::ROLE_ADMIN;
        let mut repo = edit_repo();
        repo.users.insert(
            SECOND_ADMIN,
            DieselRepoMock::make_user(SECOND_ADMIN, "frank", ""),
        );
        repo.project_members.push(ProjectMember {
            project_id: PROJECT,
            user_id: SECOND_ADMIN,
            role: ROLE_ADMIN,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo
    }

    async fn delete_client(repo: DieselRepoMock) -> Client {
        let state = AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        };
        Client::tracked(rocket::build().manage(state).mount("/api", routes![delete]))
            .await
            .expect("client")
    }

    async fn delete_as(
        client: &Client,
        user_id: Option<i32>,
        project_id: i32,
        body: Value,
    ) -> Status {
        let mut request = client
            .delete(format!("/api/projects/{project_id}"))
            .header(ContentType::JSON)
            .body(body.to_string());
        if let Some(user_id) = user_id {
            let state = client.rocket().state::<TestState>().unwrap();
            request = request.private_cookie(test_session_cookie_for(state, user_id));
        }
        request.dispatch().await.status()
    }

    fn project_exists(client: &Client) -> bool {
        let state = client.rocket().state::<TestState>().unwrap();
        state
            .repo_read()
            .inner_repo()
            .projects
            .contains_key(&PROJECT)
    }

    #[rocket::async_test]
    async fn the_owner_deletes_the_project() {
        let client = delete_client(delete_repo()).await;
        let status = delete_as(
            &client,
            Some(ADMIN_MEMBER),
            PROJECT,
            json!({ "confirm_slug": "space-project" }),
        )
        .await;
        assert_eq!(status, Status::NoContent);
        assert!(!project_exists(&client));
        let state = client.rocket().state::<TestState>().unwrap();
        let repo = state.repo_read();
        assert!(
            repo.inner_repo()
                .project_members
                .iter()
                .all(|m| m.project_id != PROJECT)
        );
    }

    #[rocket::async_test]
    async fn an_instance_admin_who_is_not_a_member_deletes_the_project() {
        let client = delete_client(delete_repo()).await;
        let status = delete_as(
            &client,
            Some(INSTANCE_ADMIN),
            PROJECT,
            json!({ "confirm_slug": "space-project" }),
        )
        .await;
        assert_eq!(status, Status::NoContent);
        assert!(!project_exists(&client));
    }

    #[rocket::async_test]
    async fn only_the_owner_or_an_instance_admin_may_delete() {
        let client = delete_client(delete_repo()).await;
        for user in [SECOND_ADMIN, VIEWER_MEMBER, AUTHOR_MEMBER, OUTSIDER] {
            let status = delete_as(
                &client,
                Some(user),
                PROJECT,
                json!({ "confirm_slug": "space-project" }),
            )
            .await;
            assert_eq!(status, Status::Forbidden, "user {user}");
        }
        assert_eq!(
            delete_as(
                &client,
                None,
                PROJECT,
                json!({ "confirm_slug": "space-project" })
            )
            .await,
            Status::Unauthorized
        );
        assert!(project_exists(&client));
    }

    #[rocket::async_test]
    async fn the_slug_must_be_confirmed() {
        let client = delete_client(delete_repo()).await;
        assert_eq!(
            delete_as(
                &client,
                Some(ADMIN_MEMBER),
                PROJECT,
                json!({ "confirm_slug": "Space Project" })
            )
            .await,
            Status::BadRequest
        );
        assert_eq!(
            delete_as(&client, Some(ADMIN_MEMBER), PROJECT, json!({})).await,
            Status::UnprocessableEntity
        );
        assert!(project_exists(&client));
        assert_eq!(
            delete_as(
                &client,
                Some(INSTANCE_ADMIN),
                999,
                json!({ "confirm_slug": "x" })
            )
            .await,
            Status::NotFound
        );
    }
}

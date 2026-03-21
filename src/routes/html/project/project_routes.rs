// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![allow(clippy::result_large_err)]

use super::prelude::*;
use crate::repository::GroupsRepository;
use crate::routes::html::helpers::{can_user_view_group, list_all_groups_sorted};
use crate::services::{ProjectService, RequirementService, VerificationService};

#[get("/<namespace>/<project_id>")]
pub fn show_project_id(
    project_access: HtmlProjectAccess,
    namespace: &str,
    project_id: &str,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let project_id = project_access.project_id();
    let user = project_access.into_user();

    // Get the specific project
    let project_service = ProjectService::new(state.inner());
    let selected_project = match project_service.get_by_id(project_id) {
        Ok(proj) => proj,
        Err(_) => {
            let ctx = json!({
                "page_title": "Project Not Found",
                "message": "The project you're looking for could not be found.",
                "details": format!("Project slug {} could not be resolved", project_slug),
                "user": user
            });
            return Ok(Template::render("error", ctx));
        }
    };

    let selected_project_name = selected_project.name.clone();
    let (group_name, group_slug, can_view_group) = selected_project
        .group_id
        .and_then(|group_id| {
            state
                .repo_read()
                .get_group_by_id(group_id)
                .ok()
                .map(|group| {
                    (
                        Some(group.name),
                        Some(group.slug),
                        can_user_view_group(state, &user, group_id),
                    )
                })
        })
        .unwrap_or((None, None, false));

    let requirement_service = RequirementService::new(state.inner());
    let verification_service = VerificationService::new(state.inner());

    let requirements_count = requirement_service
        .list_by_project(project_id)
        .map(|reqs| reqs.len())
        .unwrap_or(0);

    let tests_count = verification_service
        .list_by_project(project_id)
        .map(|verifications| verifications.len())
        .unwrap_or(0);

    let ctx = json!({
        "user": user,
        "selected_project_id": project_id,
        "selected_project_slug": project_slug,
        "page_title": format!("{} - Project", selected_project_name),
        "selected_project_name": selected_project_name,
        "group_name": group_name,
        "group_slug": group_slug,
        "can_view_group": can_view_group,
        "requirements_count": requirements_count,
        "tests_count": tests_count,
    });

    Ok(Template::render("project", ctx))
}

#[get("/<namespace>/<project_id>/edit")]
pub fn get_edit_project(
    admin: AdminOnly,
    namespace: &str,
    project_id: &str,
    state: &State<AppState>,
) -> Template {
    let user = admin.into_inner();
    let project_service = ProjectService::new(state.inner());
    let project = match project_service.get_by_namespace_and_slug(namespace, project_id) {
        Ok(project) => project,
        Err(err) => {
            #[cfg(debug_assertions)]
            eprintln!("Failed to load project {project_id}: {err:?}");
            let ctx = json!({
                "page_title": "Project Not Found",
                "message": "The project you're trying to edit could not be found.",
                "details": err.to_string(),
                "user": user
            });
            return Template::render("error", ctx);
        }
    };
    let users = state.repo_read().get_users_all().unwrap_or_default();
    let groups = list_all_groups_sorted(state);

    let ctx = json!({
        "project": crate::routes::html::helpers::project_to_template_value(state, &project),
        "users": users,
        "groups": groups,
        "user": user,
        "page_title": format!("Edit {} - Project", project.name)
    });
    Template::render("edit_project", ctx)
}

#[post("/<namespace>/<project_id>/edit", data = "<project>")]
pub fn post_edit_project(
    admin: AdminOnly,
    namespace: &str,
    project_id: &str,
    project: Form<UpdateProject>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = admin.into_inner();
    let project_service = ProjectService::new(state.inner());
    let resolved_project_id = match project_service.get_by_namespace_and_slug(namespace, project_id)
    {
        Ok(project) => project.id,
        Err(_) => return Ok(Redirect::to(uri!("/projects"))),
    };

    match project_service.update(&user, resolved_project_id, project.into_inner()) {
        Ok(_) => Ok(Redirect::to(uri!("/projects"))),
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!("Failed to update project {project_id}: {_err:?}");
            Ok(Redirect::to(format!("/{namespace}/{project_id}/edit")))
        }
    }
}

#[delete("/<namespace>/<project_id>/delete")]
pub fn delete_project_route(
    admin: AdminOnly,
    namespace: &str,
    project_id: &str,
    state: &State<AppState>,
) -> Result<rocket::http::Status, Redirect> {
    let user = admin.into_inner();
    let project_service = ProjectService::new(state.inner());
    let resolved_project_id = match project_service.get_by_namespace_and_slug(namespace, project_id)
    {
        Ok(project) => project.id,
        Err(_) => return Ok(rocket::http::Status::NotFound),
    };

    match project_service.delete(&user, resolved_project_id) {
        Ok(_) => Ok(rocket::http::Status::Ok),
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!("Failed to delete project {project_id}: {_err:?}");
            Ok(rocket::http::Status::InternalServerError)
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![
        show_project_id,
        get_edit_project,
        post_edit_project,
        delete_project_route
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::routes::html::project::test_helpers::{
        delete_with_session, get_with_session, post_form_with_session, timestamp, TestAppState,
    };
    use crate::status_enums::ProjectStatus;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;
    use rocket::Request;

    const ADMIN_ID: i32 = 1;
    const MEMBER_ID: i32 = 2;
    const OUTSIDER_ID: i32 = 3;
    const PRIMARY_PROJECT: i32 = 1;
    const PRIMARY_GROUP: i32 = 9;
    const ADMIN_NAMESPACE: &str = "site-admin";

    fn sample_project(id: i32, name: &str) -> Project {
        Project {
            id,
            name: name.to_string(),
            description: Some(format!("{name} project")),
            creation_date: Some(timestamp()),
            update_date: Some(timestamp()),
            status: ProjectStatus::Active,
            owner_id: Some(ADMIN_ID),
            slug: name.to_lowercase().replace(' ', "-"),
            group_id: None,
        }
    }

    fn sample_group(id: i32, name: &str) -> Group {
        Group {
            id,
            name: name.to_string(),
            slug: name.to_lowercase().replace(' ', "-"),
            description: Some(format!("{name} group")),
            owner_id: Some(ADMIN_ID),
            created_at: timestamp(),
            updated_at: timestamp(),
        }
    }

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        let mut admin = DieselRepoMock::make_user(ADMIN_ID, ADMIN_NAMESPACE, "");
        admin.is_admin = true;
        admin.name = "Admin User".into();
        repo.users.insert(ADMIN_ID, admin);

        let mut member = DieselRepoMock::make_user(MEMBER_ID, "member", "");
        member.name = "Project Member".into();
        repo.users.insert(MEMBER_ID, member);

        repo.projects
            .insert(PRIMARY_PROJECT, sample_project(PRIMARY_PROJECT, "Orbiter"));

        repo.project_members.push(ProjectMember {
            project_id: PRIMARY_PROJECT,
            user_id: ADMIN_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo.project_members.push(ProjectMember {
            project_id: PRIMARY_PROJECT,
            user_id: MEMBER_ID,
            role: 2,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo
    }

    async fn project_client(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(crate::routes::html::project::test_helpers::managed_state(
                repo,
            ))
            .attach(Template::fairing())
            .mount(
                "/",
                routes![
                    show_project_id,
                    get_edit_project,
                    post_edit_project,
                    delete_project_route
                ],
            )
            .register("/", catchers![forbidden_catcher]);

        Client::tracked(rocket).await.expect("client")
    }

    #[catch(403)]
    fn forbidden_catcher(_req: &Request) -> Redirect {
        Redirect::to(uri!("/projects"))
    }

    #[rocket::async_test]
    async fn show_project_id_renders_for_admin() {
        let mut repo = base_repo();
        repo.groups
            .insert(PRIMARY_GROUP, sample_group(PRIMARY_GROUP, "Flight Systems"));
        let project = repo
            .projects
            .get_mut(&PRIMARY_PROJECT)
            .expect("project present");
        project.group_id = Some(PRIMARY_GROUP);

        let client = project_client(repo).await;
        let response = get_with_session(&client, "/flight-systems/orbiter", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Orbiter"));
        assert!(body.contains("Project Members"));
        assert!(body.contains("Admin User"));
        assert!(body.contains("Group:"));
        assert!(body.contains("href=\"/flight-systems\""));
    }

    #[rocket::async_test]
    async fn show_project_id_allows_project_member() {
        let mut repo = base_repo();
        repo.groups
            .insert(PRIMARY_GROUP, sample_group(PRIMARY_GROUP, "Flight Systems"));
        let project = repo
            .projects
            .get_mut(&PRIMARY_PROJECT)
            .expect("project present");
        project.group_id = Some(PRIMARY_GROUP);

        let client = project_client(repo).await;
        let response = get_with_session(&client, "/flight-systems/orbiter", MEMBER_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Orbiter"));
        assert!(body.contains("Project Member"));
        assert!(body.contains("Flight Systems"));
        assert!(!body.contains("href=\"/flight-systems\""));
    }

    #[rocket::async_test]
    async fn show_project_id_redirects_non_member() {
        let mut repo = base_repo();
        let mut outsider = DieselRepoMock::make_user(OUTSIDER_ID, "outsider", "");
        outsider.name = "Curious User".into();
        repo.users.insert(OUTSIDER_ID, outsider);

        let client = project_client(repo).await;
        let path = format!("/{ADMIN_NAMESPACE}/orbiter");
        let response = get_with_session(&client, &path, OUTSIDER_ID).await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/projects"));
    }

    #[rocket::async_test]
    async fn show_project_id_renders_error_when_missing() {
        let mut repo = DieselRepoMock::default();
        let mut admin = DieselRepoMock::make_user(ADMIN_ID, ADMIN_NAMESPACE, "");
        admin.is_admin = true;
        repo.users.insert(ADMIN_ID, admin);

        let client = project_client(repo).await;
        let path = format!("/{ADMIN_NAMESPACE}/nonexistent-slug");
        let response = get_with_session(&client, &path, ADMIN_ID).await;

        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn show_project_id_allows_reserved_owner_namespace() {
        let mut repo = DieselRepoMock::default();

        let mut reserved_owner = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        reserved_owner.name = "System Administrator".into();
        reserved_owner.is_admin = true;
        repo.users.insert(ADMIN_ID, reserved_owner);

        let mut member = DieselRepoMock::make_user(MEMBER_ID, "alice", "");
        member.name = "Alice Johnson".into();
        repo.users.insert(MEMBER_ID, member);

        let project = Project {
            id: PRIMARY_PROJECT,
            name: "Empty Project".into(),
            description: Some("Reserved-owner project".into()),
            creation_date: Some(timestamp()),
            update_date: Some(timestamp()),
            status: ProjectStatus::Active,
            owner_id: Some(ADMIN_ID),
            slug: "empty-project".into(),
            group_id: None,
        };
        repo.projects.insert(PRIMARY_PROJECT, project);
        repo.project_members.push(ProjectMember {
            project_id: PRIMARY_PROJECT,
            user_id: ADMIN_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo.project_members.push(ProjectMember {
            project_id: PRIMARY_PROJECT,
            user_id: MEMBER_ID,
            role: 2,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        let client = project_client(repo).await;
        let response = get_with_session(&client, "/admin/empty-project", MEMBER_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Empty Project"));
        assert!(body.contains("href=\"/admin/empty-project\""));
    }

    #[rocket::async_test]
    async fn get_edit_project_renders_form() {
        let mut repo = base_repo();
        repo.groups
            .insert(PRIMARY_GROUP, sample_group(PRIMARY_GROUP, "Flight Systems"));
        let project = repo
            .projects
            .get_mut(&PRIMARY_PROJECT)
            .expect("project present");
        project.group_id = Some(PRIMARY_GROUP);

        let client = project_client(repo).await;
        let response = get_with_session(&client, "/flight-systems/orbiter/edit", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Edit Project"));
        assert!(body.contains("value=\"Orbiter\""));
        assert!(body.contains("name=\"group_id\""));
        assert!(body.contains("No group"));
        assert!(body.contains("<option value=\"9\" selected>"));
        assert!(body.contains("Flight Systems"));
    }

    #[rocket::async_test]
    async fn post_edit_project_updates_project() {
        let mut repo = base_repo();
        repo.groups
            .insert(PRIMARY_GROUP, sample_group(PRIMARY_GROUP, "Flight Systems"));

        let client = project_client(repo).await;
        let path = format!("/{ADMIN_NAMESPACE}/orbiter/edit");
        let response = post_form_with_session(
            &client,
            &path,
            "name=Orbiter+II&description=Updated+mission+plan&status=active&owner_id=1&group_id=9",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/projects"));

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let project = repo
            .get_project_by_id(PRIMARY_PROJECT)
            .expect("project present");
        assert_eq!(project.name, "Orbiter II");
        assert_eq!(project.description.as_deref(), Some("Updated mission plan"));
        assert_eq!(project.status, ProjectStatus::Active);
        assert_eq!(project.owner_id, Some(ADMIN_ID));
        assert_eq!(project.group_id, Some(PRIMARY_GROUP));
    }

    #[rocket::async_test]
    async fn post_edit_project_clears_group_when_no_group_selected() {
        let mut repo = base_repo();
        repo.groups
            .insert(PRIMARY_GROUP, sample_group(PRIMARY_GROUP, "Flight Systems"));
        let project = repo
            .projects
            .get_mut(&PRIMARY_PROJECT)
            .expect("project present");
        project.group_id = Some(PRIMARY_GROUP);

        let client = project_client(repo).await;
        let response = post_form_with_session(
            &client,
            "/flight-systems/orbiter/edit",
            "name=Orbiter&description=Updated+mission+plan&status=active&owner_id=1&group_id=0",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/projects"));

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let project = repo
            .get_project_by_id(PRIMARY_PROJECT)
            .expect("project present");
        assert_eq!(project.group_id, None);
    }

    #[rocket::async_test]
    async fn post_edit_project_redirects_back_on_validation_error() {
        let client = project_client(base_repo()).await;
        let path = format!("/{ADMIN_NAMESPACE}/orbiter/edit");
        let response = post_form_with_session(
            &client,
            &path,
            "name=&description=&status=completed&owner_id=&group_id=0",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/site-admin/orbiter/edit")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let project = repo
            .get_project_by_id(PRIMARY_PROJECT)
            .expect("project present");
        assert_eq!(project.name, "Orbiter");
    }

    #[rocket::async_test]
    async fn delete_project_route_removes_project() {
        let client = project_client(base_repo()).await;
        let path = format!("/{ADMIN_NAMESPACE}/orbiter/delete");
        let response = delete_with_session(&client, &path, ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        assert!(repo.get_project_by_id(PRIMARY_PROJECT).is_err());
    }

    #[rocket::async_test]
    async fn delete_project_route_returns_error_on_missing_project() {
        let mut repo = base_repo();
        repo.projects.remove(&PRIMARY_PROJECT);

        let client = project_client(repo).await;
        let path = format!("/{ADMIN_NAMESPACE}/orbiter/delete");
        let response = delete_with_session(&client, &path, ADMIN_ID).await;

        assert_eq!(response.status(), Status::NotFound);
    }
}

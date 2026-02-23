// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

#![allow(clippy::result_large_err)]

use super::helpers::*;
use super::prelude::*;
use crate::repository::errors::RepoError;
use crate::services::project_service::ProjectService;
use rocket::serde::json::Value;

#[get("/projects")]
pub fn show_projects(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    let (projects, selected_project_id) = get_user_projects_and_selection(state, &user, cookies);
    let decorated_projects = decorate_projects_for_listing(state, &user, &projects);

    let ctx = json!({
        "projects": decorated_projects,
        "user": user,
        "selected_project_id": selected_project_id,
        "page_title": "Projects"
    });

    Ok(Template::render("projects", ctx))
}

#[get("/new_project?<error>")]
pub fn new_project(admin: AdminOnly, state: &State<AppState>, error: Option<String>) -> Template {
    let user = admin.into_inner();
    render_new_project_form(state, &user, default_new_project_form(), error)
}

#[post("/new_project", data = "<new_project>")]
pub fn post_project(
    admin: AdminOnly,
    new_project: Form<NewProject>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = admin.into_inner();
    let project_service = ProjectService::new(state.inner());
    let submitted = new_project.into_inner();

    match project_service.create(&user, submitted) {
        Ok(_) => Ok(Redirect::to(uri!(show_projects))),
        Err(err) => {
            #[cfg(debug_assertions)]
            eprintln!("Failed to create project: {err:?}");
            let message = match err {
                RepoError::BadInput(reason) => reason,
                _ => "Failed to create project. Please try again.".to_string(),
            };

            Err(Redirect::to(uri!(new_project(error = Some(message)))))
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![show_projects, new_project, post_project]
}

fn render_new_project_form(
    state: &State<AppState>,
    user: &User,
    form: Value,
    error: Option<String>,
) -> Template {
    let users = state.repo_read().get_users_all().unwrap_or_default();

    let ctx = json!({
        "users": users,
        "user": user,
        "form": form,
        "error": error,
        "page_title": "New Project"
    });

    Template::render("new_project", ctx)
}

fn default_new_project_form() -> Value {
    json!({
        "name": "",
        "description": "",
        "status_id": "active",
        "owner_id": null,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::models::ProjectMember;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use crate::status_enums::ProjectStatus;
    use chrono::{NaiveDate, NaiveDateTime};
    use rocket::http::{ContentType, Cookie, Status};
    use rocket::local::asynchronous::{Client, LocalResponse};
    use rocket_dyn_templates::Template;
    use std::sync::{Arc, RwLock};

    type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;
    const USER_ID: i32 = 2;
    const OWNER_ID: i32 = 3;

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    fn admin_user() -> User {
        let mut user = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        user.is_admin = true;
        user.name = "Admin User".into();
        user.email = "admin@example.com".into();
        user
    }

    fn standard_user() -> User {
        let mut user = DieselRepoMock::make_user(USER_ID, "jane", "");
        user.name = "Jane Doe".into();
        user.email = "jane@example.com".into();
        user
    }

    fn owner_user() -> User {
        let mut user = DieselRepoMock::make_user(OWNER_ID, "owner", "");
        user.name = "Mission Owner".into();
        user.email = "owner@example.com".into();
        user
    }

    fn project(id: i32, name: &str, owner_id: i32) -> Project {
        Project {
            id,
            name: name.into(),
            description: Some("Mission critical project".into()),
            creation_date: Some(timestamp()),
            update_date: Some(timestamp()),
            status: ProjectStatus::Active,
            owner_id: Some(owner_id),
        }
    }

    fn managed_state(repo: DieselRepoMock) -> TestAppState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(managed_state(repo))
            .attach(Template::fairing())
            .mount("/", routes![show_projects, new_project, post_project]);
        Client::tracked(rocket).await.expect("client")
    }

    fn session_cookie(id: i32) -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, id.to_string());
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie
    }

    async fn get_with_session<'c>(client: &'c Client, path: &'c str, id: i32) -> LocalResponse<'c> {
        client
            .get(path)
            .private_cookie(session_cookie(id))
            .dispatch()
            .await
    }

    async fn post_with_session<'c>(
        client: &'c Client,
        path: &'c str,
        body: &'c str,
        id: i32,
    ) -> LocalResponse<'c> {
        client
            .post(path)
            .header(ContentType::Form)
            .body(body)
            .private_cookie(session_cookie(id))
            .dispatch()
            .await
    }

    #[rocket::async_test]
    async fn show_projects_lists_only_accessible_projects() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(USER_ID, standard_user());
        let owner = owner_user();
        let owner_id = owner.id;
        repo.users.insert(owner_id, owner);

        let accessible = project(7, "Mars Lander", owner_id);
        let accessible_id = accessible.id;
        let inaccessible = project(8, "Venus Rover", owner_id);
        repo.projects.insert(accessible_id, accessible);
        repo.projects.insert(inaccessible.id, inaccessible);

        repo.project_members.push(ProjectMember {
            project_id: accessible_id,
            user_id: USER_ID,
            role: 2,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        let client = test_client(repo).await;
        let response = get_with_session(&client, "/projects", USER_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Mars Lander"));
        assert!(!body.contains("Venus Rover"));
        assert!(body.contains("Role: Manager"));
        assert!(body.contains("Owned by Mission Owner"));
    }

    #[rocket::async_test]
    async fn new_project_page_renders_admin_form() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(ADMIN_ID, admin_user());
        repo.users.insert(USER_ID, standard_user());

        let client = test_client(repo).await;
        let response = get_with_session(&client, "/new_project", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Create New Project"));
        assert!(body.contains("Project Owner *"));
        assert!(body.contains("Admin User (admin)"));
        assert!(body.contains("Jane Doe (jane)"));
    }

    #[rocket::async_test]
    async fn post_project_creates_project_and_redirects() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(ADMIN_ID, admin_user());

        let client = test_client(repo).await;
        let response = post_with_session(
            &client,
            "/new_project",
            "name=New+Initiative&description=Launch+prep&owner_id=1",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/projects"));

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo_guard = state.repo.read().expect("repo lock");
        let inner = repo_guard.inner_repo();
        assert_eq!(inner.projects.len(), 1);
        let project = inner.projects.values().next().expect("project stored");
        assert_eq!(project.name, "New Initiative");
        assert_eq!(project.description.as_deref(), Some("Launch prep"));
        assert_eq!(project.status, ProjectStatus::Active);
        assert_eq!(project.owner_id, Some(ADMIN_ID));
    }

    #[rocket::async_test]
    async fn post_project_redirects_back_on_validation_error() {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(ADMIN_ID, admin_user());

        let client = test_client(repo).await;
        let response = post_with_session(
            &client,
            "/new_project",
            "name=&description=&owner_id=",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/new_project?error=Field%20'name'%20is%20required")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo_guard = state.repo.read().expect("repo lock");
        let inner = repo_guard.inner_repo();
        assert!(inner.projects.is_empty());
    }
}

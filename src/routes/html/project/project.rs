use super::helpers::*;
use super::prelude::*;
use crate::services::project_service::ProjectService;
use chrono::Utc;

#[get("/<project_id>")]
pub fn show_project_id(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    let project_service = ProjectService::new(state.inner());
    if !user.is_admin {
        let memberships = state
            .repo_read()
            .get_projects_for_user(user.user_id)
            .unwrap_or_default();

        let has_access = memberships
            .iter()
            .any(|membership| membership.project_id == project_id);

        if !has_access {
            return Err(Redirect::to(uri!("/projects")));
        }
    }
    let project = project_service
        .get_by_id(project_id)
        .unwrap_or_else(|_| fallback_project());

    let members = state
        .repo_read()
        .get_members_by_project(project_id)
        .unwrap_or_default();

    let user_map: HashMap<i32, User> = state
        .repo_read()
        .get_users_all()
        .unwrap_or_default()
        .into_iter()
        .map(|u| (u.user_id, u))
        .collect();

    let decorated_members: Vec<_> = members
        .into_iter()
        .map(|membership| {
            let role_label = describe_project_role(membership.role).to_string();
            if let Some(user) = user_map.get(&membership.user_id) {
                json!({
                    "user_id": user.user_id,
                    "user_name": user.user_name,
                    "user_username": user.user_username,
                    "user_email": user.user_email,
                    "role_label": role_label,
                    "role_id": membership.role,
                    "is_admin": user.is_admin
                })
            } else {
                json!({
                    "user_id": membership.user_id,
                    "user_name": format!("Unknown User #{}", membership.user_id),
                    "user_username": "unknown",
                    "user_email": "",
                    "role_label": role_label,
                    "role_id": membership.role,
                    "is_admin": false
                })
            }
        })
        .collect();

    let mut ctx = build_context_with_projects(state, user.clone(), cookies);
    if let Some(ctx_obj) = ctx.as_object_mut() {
        ctx_obj.insert("project".to_string(), json!(project));
        ctx_obj.insert("members".to_string(), json!(decorated_members));
        ctx_obj.insert("user".to_string(), json!(user));
    }

    Ok(Template::render("project_detail", ctx))
}

#[get("/<project_id>/edit")]
pub fn get_edit_project(admin: AdminOnly, project_id: i32, state: &State<AppState>) -> Template {
    let user = admin.into_inner();
    let project_service = ProjectService::new(state.inner());
    let project = project_service
        .get_by_id(project_id)
        .unwrap_or_else(|_| fallback_project());
    let users = state.repo_read().get_users_all().unwrap_or_default();

    let ctx = json!({
        "project": project,
        "users": users,
        "user": user
    });
    Template::render("edit_project", ctx)
}

#[post("/<project_id>/edit", data = "<project>")]
pub fn post_edit_project(
    admin: AdminOnly,
    project_id: i32,
    project: Form<UpdateProject>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = admin.into_inner();
    let project_service = ProjectService::new(state.inner());

    match project_service.update(&user, project_id, project.into_inner()) {
        Ok(_) => Ok(Redirect::to(uri!("/projects"))),
        Err(err) => {
            #[cfg(debug_assertions)]
            eprintln!("Failed to update project {project_id}: {err:?}");
            Ok(Redirect::to(uri!(get_edit_project(project_id))))
        }
    }
}

#[delete("/<project_id>/delete")]
pub fn delete_project_route(
    admin: AdminOnly,
    project_id: i32,
    state: &State<AppState>,
) -> Result<rocket::http::Status, Redirect> {
    let user = admin.into_inner();
    let project_service = ProjectService::new(state.inner());

    match project_service.delete(&user, project_id) {
        Ok(_) => Ok(rocket::http::Status::Ok),
        Err(err) => {
            #[cfg(debug_assertions)]
            eprintln!("Failed to delete project {project_id}: {err:?}");
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

fn fallback_project() -> Project {
    Project {
        project_id: 0,
        project_name: "Unknown Project".to_string(),
        project_description: Some("Unknown project".to_string()),
        project_creation_date: Some(Utc::now().naive_utc()),
        project_update_date: Some(Utc::now().naive_utc()),
        project_status: Some("Unknown".to_string()),
        project_owner_id: Some(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::routes::html::project::test_helpers::{
        client_with_routes, delete_with_session, get_with_session, post_form_with_session, timestamp,
        TestAppState,
    };
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    const ADMIN_ID: i32 = 1;
    const MEMBER_ID: i32 = 2;
    const OUTSIDER_ID: i32 = 3;
    const PRIMARY_PROJECT: i32 = 1;

    fn sample_project(id: i32, name: &str) -> Project {
        Project {
            project_id: id,
            project_name: name.to_string(),
            project_description: Some(format!("{name} project")),
            project_creation_date: Some(timestamp()),
            project_update_date: Some(timestamp()),
            project_status: Some("active".to_string()),
            project_owner_id: Some(ADMIN_ID),
        }
    }

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        let mut admin = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        admin.is_admin = true;
        admin.user_name = "Admin User".into();
        repo.users.insert(ADMIN_ID, admin);

        let mut member = DieselRepoMock::make_user(MEMBER_ID, "member", "");
        member.user_name = "Project Member".into();
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
        client_with_routes(
            repo,
            routes![show_project_id, get_edit_project, post_edit_project, delete_project_route],
        )
        .await
    }

    #[rocket::async_test]
    async fn show_project_id_renders_for_admin() {
        let client = project_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Orbiter"));
        assert!(body.contains("Project Members"));
        assert!(body.contains("Admin User"));
    }

    #[rocket::async_test]
    async fn show_project_id_allows_project_member() {
        let client = project_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1", MEMBER_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Orbiter"));
        assert!(body.contains("Project Member"));
    }

    #[rocket::async_test]
    async fn show_project_id_redirects_non_member() {
        let mut repo = base_repo();
        let mut outsider = DieselRepoMock::make_user(OUTSIDER_ID, "outsider", "");
        outsider.user_name = "Curious User".into();
        repo.users.insert(OUTSIDER_ID, outsider);

        let client = project_client(repo).await;
        let response = get_with_session(&client, "/p/1", OUTSIDER_ID).await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/projects"));
    }

    #[rocket::async_test]
    async fn show_project_id_uses_fallback_when_missing() {
        let mut repo = DieselRepoMock::default();
        let mut admin = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        admin.is_admin = true;
        repo.users.insert(ADMIN_ID, admin);

        let client = project_client(repo).await;
        let response = get_with_session(&client, "/p/42", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Unknown Project"));
    }

    #[rocket::async_test]
    async fn get_edit_project_renders_form() {
        let client = project_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/edit", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Edit Project"));
        assert!(body.contains("value=\"Orbiter\""));
    }

    #[rocket::async_test]
    async fn post_edit_project_updates_project() {
        let client = project_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/edit",
            "project_name=Orbiter+II&project_description=Updated+mission+plan&project_status=inactive&project_owner_id=1",
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
        assert_eq!(project.project_name, "Orbiter II");
        assert_eq!(project.project_description.as_deref(), Some("Updated mission plan"));
        assert_eq!(project.project_status.as_deref(), Some("inactive"));
        assert_eq!(project.project_owner_id, Some(ADMIN_ID));
    }

    #[rocket::async_test]
    async fn post_edit_project_redirects_back_on_validation_error() {
        let client = project_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/edit",
            "project_name=&project_description=&project_status=active&project_owner_id=",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/p/1/edit"));

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let project = repo
            .get_project_by_id(PRIMARY_PROJECT)
            .expect("project present");
        assert_eq!(project.project_name, "Orbiter");
    }

    #[rocket::async_test]
    async fn delete_project_route_removes_project() {
        let client = project_client(base_repo()).await;
        let response = delete_with_session(&client, "/p/1/delete", ADMIN_ID).await;

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
        let response = delete_with_session(&client, "/p/1/delete", ADMIN_ID).await;

        assert_eq!(response.status(), Status::InternalServerError);
    }
}

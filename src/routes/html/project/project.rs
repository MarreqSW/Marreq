use super::prelude::*;
use crate::services::{ProjectService, RequirementService, TestService};

#[get("/<project_id>")]
pub fn show_project_id(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    // Get the specific project
    let project_service = ProjectService::new(state.inner());
    let selected_project = match project_service.get_by_id(project_id) {
        Ok(proj) => proj,
        Err(_) => {
            let ctx = json!({
                "page_title": "Project Not Found",
                "message": "The project you're looking for could not be found.",
                "details": format!("Project ID {} does not exist", project_id),
                "user": user
            });
            return Ok(Template::render("error", ctx));
        }
    };

    let selected_project_name = selected_project.name.clone();

    let requirement_service = RequirementService::new(state.inner());
    let test_service = TestService::new(state.inner());

    let requirements_count = requirement_service
        .list_by_project(project_id)
        .map(|reqs| reqs.len())
        .unwrap_or(0);

    let tests_count = test_service
        .list_by_project(project_id)
        .map(|tests| tests.len())
        .unwrap_or(0);

    let ctx = json!({
        "user": user,
        "selected_project_id": project_id,
        "page_title": format!("{} - Project", selected_project_name),
        "selected_project_name": selected_project_name,
        "requirements_count": requirements_count,
        "tests_count": tests_count,
    });

    Ok(Template::render("project", ctx))
}

#[get("/<project_id>/edit")]
pub fn get_edit_project(admin: AdminOnly, project_id: i32, state: &State<AppState>) -> Template {
    let user = admin.into_inner();
    let project_service = ProjectService::new(state.inner());
    let project = match project_service.get_by_id(project_id) {
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

    let ctx = json!({
        "project": project,
        "users": users,
        "user": user,
        "page_title": format!("Edit {} - Project", project.name)
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
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!("Failed to update project {project_id}: {_err:?}");
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

    fn sample_project(id: i32, name: &str) -> Project {
        Project {
            id: id,
            name: name.to_string(),
            description: Some(format!("{name} project")),
            creation_date: Some(timestamp()),
            update_date: Some(timestamp()),
            status: ProjectStatus::Active,
            owner_id: Some(ADMIN_ID),
        }
    }

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        let mut admin = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
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
                "/p",
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
        outsider.name = "Curious User".into();
        repo.users.insert(OUTSIDER_ID, outsider);

        let client = project_client(repo).await;
        let response = get_with_session(&client, "/p/1", OUTSIDER_ID).await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/projects"));
    }

    #[rocket::async_test]
    async fn show_project_id_renders_error_when_missing() {
        let mut repo = DieselRepoMock::default();
        let mut admin = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        admin.is_admin = true;
        repo.users.insert(ADMIN_ID, admin);

        let client = project_client(repo).await;
        let response = get_with_session(&client, "/p/42", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Project Not Found"));
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
            "name=Orbiter+II&description=Updated+mission+plan&status_id=0&owner_id=1",
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
    }

    #[rocket::async_test]
    async fn post_edit_project_redirects_back_on_validation_error() {
        let client = project_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/edit",
            "name=&description=&status_id=1&owner_id=",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/1/edit"));

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

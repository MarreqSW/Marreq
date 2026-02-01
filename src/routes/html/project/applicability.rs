use super::helpers::*;
use super::prelude::*;
use crate::services::ApplicabilityService;
use rocket::http::Status;

#[get("/<project_id>/applicability")]
pub async fn show_applicability(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);
    let service = ApplicabilityService::new(state.inner());
    let apps = service.list_by_project(project_id).unwrap_or_default();

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "applicability": apps,
        "page_title": "Applicability"
    });

    Ok(Template::render("applicability/applicability", ctx))
}

#[get("/<project_id>/applicability/new?<error>")]
pub async fn new_applicability(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "error": error,
        "page_title": "New Applicability"
    });

    Ok(Template::render("applicability/new_applicability", ctx))
}

#[post("/<project_id>/applicability/new", data = "<form>")]
pub async fn post_applicability(
    project_access: ProjectAccess,
    project_id: i32,
    form: Form<NewApplicability>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let service = ApplicabilityService::new(state.inner());

    let new_url = uri!(
        "/p",
        new_applicability(
            project_id = project_id,
            error = Some("Failed to create applicability".to_string())
        )
    );
    let show_url = uri!("/p", show_applicability(project_id = project_id));

    let new_applicability = NewApplicability {
        project_id,
        ..form.into_inner()
    };

    if let Err(_err) = service.create(&user, new_applicability) {
        #[cfg(debug_assertions)]
        eprintln!("Error inserting applicability: {_err:?}");
        return Ok(Redirect::to(new_url));
    }

    Ok(Redirect::to(show_url))
}

#[get("/<project_id>/applicability/edit/<applicability_id>?<error>")]
pub async fn get_edit_applicability(
    project_access: ProjectAccess,
    project_id: i32,
    applicability_id: i32,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);
    let service = ApplicabilityService::new(state.inner());
    let applicability = service
        .get_by_id(applicability_id)
        .map_err(|_| Redirect::to(uri!("/p", show_applicability(project_id = project_id))))?;

    if applicability.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_applicability(project_id = applicability.project_id)
        )));
    }

    let ctx = json!({
        "applicability": applicability,
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "error": error,
        "page_title": format!("Edit {} - Applicability", applicability.title)
    });
    Ok(Template::render("applicability/edit_applicability", ctx))
}

#[post("/<project_id>/applicability/edit/<applicability_id>", data = "<form>")]
pub async fn post_edit_applicability(
    project_access: ProjectAccess,
    project_id: i32,
    applicability_id: i32,
    form: Form<NewApplicability>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let service = ApplicabilityService::new(state.inner());

    let edit_url = uri!(
        "/p",
        get_edit_applicability(
            project_id = project_id,
            applicability_id = applicability_id,
            error = Some("Failed to update applicability".to_string())
        )
    );
    let show_url = uri!("/p", show_applicability(project_id = project_id));

    let old = service
        .get_by_id(applicability_id)
        .map_err(|_| Redirect::to(show_url.clone()))?;
    if old.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_applicability(project_id = old.project_id)
        )));
    }

    let new = NewApplicability {
        project_id,
        ..form.into_inner()
    };

    if let Err(_err) = service.update(&user, applicability_id, new) {
        #[cfg(debug_assertions)]
        eprintln!("Error updating applicability {applicability_id}: {_err:?}");
        return Ok(Redirect::to(edit_url));
    }

    Ok(Redirect::to(show_url))
}

#[delete("/<project_id>/applicability/delete/<applicability_id>")]
pub async fn delete_applicability_route(
    project_access: ProjectAccess,
    project_id: i32,
    applicability_id: i32,
    state: &State<AppState>,
) -> Result<Status, Redirect> {
    let user = project_access.into_user();
    let show_url = uri!("/p", show_applicability(project_id = project_id));
    let service = ApplicabilityService::new(state.inner());
    let applicability = service
        .get_by_id(applicability_id)
        .map_err(|_| Redirect::to(show_url.clone()))?;
    if applicability.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_applicability(project_id = applicability.project_id)
        )));
    }

    match service.delete(&user, applicability_id) {
        Ok(_) => Ok(Status::Ok),
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!("Error deleting applicability {applicability_id}: {_err:?}");
            Ok(Status::InternalServerError)
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![
        show_applicability,
        new_applicability,
        post_applicability,
        get_edit_applicability,
        post_edit_applicability,
        delete_applicability_route
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Applicability, Project, ProjectMember};
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::routes::html::project::test_helpers::{
        client_with_routes, get_with_session, post_form_with_session, timestamp,
    };
    use crate::status_enums::ProjectStatus;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    const ADMIN_ID: i32 = 1;
    const PRIMARY_PROJECT: i32 = 1;

    fn sample_project(id: i32, name: &str) -> Project {
        Project {
            id,
            name: name.to_string(),
            description: Some(format!("{name} project")),
            creation_date: Some(timestamp()),
            update_date: Some(timestamp()),
            status: ProjectStatus::Active,
            owner_id: Some(ADMIN_ID),
        }
    }

    fn sample_applicability(id: i32, project_id: i32, title: &str) -> Applicability {
        Applicability {
            id,
            title: title.to_string(),
            description: format!("Description for {title}"),
            tag: title.to_ascii_lowercase(),
            project_id,
        }
    }

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();
        let mut admin = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        admin.is_admin = true;
        repo.users.insert(ADMIN_ID, admin);
        repo.projects
            .insert(PRIMARY_PROJECT, sample_project(PRIMARY_PROJECT, "Mars"));
        repo.applicability
            .insert(1, sample_applicability(1, PRIMARY_PROJECT, "Flight"));
        repo.project_members.push(ProjectMember {
            project_id: PRIMARY_PROJECT,
            user_id: ADMIN_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        client_with_routes(
            repo,
            routes![
                show_applicability,
                new_applicability,
                post_applicability,
                get_edit_applicability,
                post_edit_applicability,
                delete_applicability_route
            ],
        )
        .await
    }

    #[rocket::async_test]
    async fn show_applicability_renders_known_items() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/applicability", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("#1"));
        assert!(body.contains("Flight"));
    }

    #[rocket::async_test]
    async fn new_applicability_form_renders() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/applicability/new", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("New Applicability"));
        assert!(body.contains("Create Applicability"));
    }

    #[rocket::async_test]
    async fn get_edit_applicability_returns_prefilled_form() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/applicability/edit/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Edit Applicability"));
        assert!(body.contains("value=\"Flight\""));
    }

    #[rocket::async_test]
    async fn get_edit_applicability_redirects_when_project_mismatch() {
        let mut repo = base_repo();
        repo.projects.insert(2, sample_project(2, "Venus"));
        repo.applicability
            .insert(2, sample_applicability(2, 2, "Surface"));
        let client = test_client(repo).await;
        let response = get_with_session(&client, "/p/1/applicability/edit/2", ADMIN_ID).await;
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/2/applicability")
        );
    }

    #[rocket::async_test]
    async fn post_edit_applicability_redirects_when_project_mismatch() {
        let mut repo = base_repo();
        repo.projects.insert(2, sample_project(2, "Venus"));
        repo.applicability
            .insert(2, sample_applicability(2, 2, "Surface"));
        let client = test_client(repo).await;
        let response = post_form_with_session(
            &client,
            "/p/1/applicability/edit/2",
            "id=2&project_id=2&title=Surface&description=Planet&tag=surface",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/2/applicability")
        );
    }
}

use super::helpers::*;
use super::prelude::*;
use crate::services::VerificationService;
use rocket::http::Status;

#[get("/<project_id>/verification")]
pub async fn show_verification(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);
    let service = VerificationService::new(state.inner());
    let verifications = service.list_by_project(project_id).unwrap_or_default();

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "verifications": verifications,
        "page_title": "Verification methods"
    });

    Ok(Template::render("verification/verification", ctx))
}

#[get("/<project_id>/verification/new?<error>")]
pub async fn new_verification(
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
        "page_title": "New Verification Method"
    });

    Ok(Template::render("verification/new_verification", ctx))
}

#[post("/<project_id>/verification/new", data = "<form>")]
pub async fn post_verification(
    project_access: ProjectAccess,
    project_id: i32,
    form: Form<NewVerificationMethod>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let _user = project_access.into_user();
    let service = VerificationService::new(state.inner());

    let new_url = uri!(
        "/p",
        new_verification(
            project_id = project_id,
            error = Some("Failed to create verification method".to_string())
        )
    );
    let show_url = uri!("/p", show_verification(project_id = project_id));

    let new_verification = NewVerificationMethod {
        project_id,
        ..form.into_inner()
    };

    if let Err(_err) = service.create(new_verification) {
        #[cfg(debug_assertions)]
        eprintln!("Error inserting verification method: {_err:?}");
        return Ok(Redirect::to(new_url));
    }

    Ok(Redirect::to(show_url))
}

#[get("/<project_id>/verification/edit/<verification_id>?<error>")]
pub async fn get_edit_verification(
    project_access: ProjectAccess,
    project_id: i32,
    verification_id: i32,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);
    let service = VerificationService::new(state.inner());
    let verification = service
        .get_by_id(verification_id)
        .map_err(|_| Redirect::to(uri!("/p", show_verification(project_id = project_id))))?;

    if verification.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_verification(project_id = verification.project_id)
        )));
    }

    let ctx = json!({
        "verification": verification,
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "error": error,
        "page_title": format!("Edit {} - Verification method", verification.title)
    });
    Ok(Template::render("verification/edit_verification", ctx))
}

#[post("/<project_id>/verification/edit/<verification_id>", data = "<form>")]
pub async fn post_edit_verification(
    project_access: ProjectAccess,
    project_id: i32,
    verification_id: i32,
    form: Form<NewVerificationMethod>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let _user = project_access.into_user();
    let service = VerificationService::new(state.inner());

    let edit_url = uri!(
        "/p",
        get_edit_verification(
            project_id = project_id,
            verification_id = verification_id,
            error = Some("Failed to update verification method".to_string())
        )
    );
    let show_url = uri!("/p", show_verification(project_id = project_id));

    let old = service
        .get_by_id(verification_id)
        .map_err(|_| Redirect::to(show_url.clone()))?;
    if old.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_verification(project_id = old.project_id)
        )));
    }

    let mut payload = form.into_inner();
    payload.id = Some(verification_id);
    payload.project_id = project_id;

    if let Err(_err) = service.update(verification_id, payload) {
        #[cfg(debug_assertions)]
        eprintln!("Error updating verification method {verification_id}: {_err:?}");
        return Ok(Redirect::to(edit_url));
    }

    Ok(Redirect::to(show_url))
}

#[delete("/<project_id>/verification/delete/<verification_id>")]
pub async fn delete_verification_route(
    project_access: ProjectAccess,
    project_id: i32,
    verification_id: i32,
    state: &State<AppState>,
) -> Result<Status, Redirect> {
    let _user = project_access.into_user();
    let show_url = uri!("/p", show_verification(project_id = project_id));
    let service = VerificationService::new(state.inner());
    let verification = service
        .get_by_id(verification_id)
        .map_err(|_| Redirect::to(show_url.clone()))?;
    if verification.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_verification(project_id = verification.project_id)
        )));
    }

    match service.delete(verification_id) {
        Ok(_) => Ok(Status::Ok),
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!("Error deleting verification method {verification_id}: {_err:?}");
            Ok(Status::InternalServerError)
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![
        show_verification,
        new_verification,
        post_verification,
        get_edit_verification,
        post_edit_verification,
        delete_verification_route
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Project, ProjectMember, VerificationMethod};
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::routes::html::project::test_helpers::{
        client_with_routes, delete_with_session, get_with_session, post_form_with_session,
        timestamp, TestAppState,
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

    fn sample_verification(id: i32, project_id: i32, title: &str) -> VerificationMethod {
        VerificationMethod {
            id,
            title: title.to_string(),
            description: format!("Description for {title}"),
            tag: title.to_uppercase(),
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
        repo.verifications
            .insert(1, sample_verification(1, PRIMARY_PROJECT, "Analysis"));
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
                show_verification,
                new_verification,
                post_verification,
                get_edit_verification,
                post_edit_verification,
                delete_verification_route
            ],
        )
        .await
    }

    #[rocket::async_test]
    async fn show_verification_renders_known_items() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/verification", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Verification methods"));
        assert!(body.contains("#1"));
        assert!(body.contains("Analysis"));
    }

    #[rocket::async_test]
    async fn new_verification_form_renders() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/verification/new", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("New Verification Method"));
        assert!(body.contains("Create"));
        assert!(body.contains("Verification Method"));
    }

    #[rocket::async_test]
    async fn post_verification_creates_new_entry() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/verification/new",
            "title=Test&description=Test+verification&tag=TEST&project_id=1",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/verification")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let list = repo
            .get_verification_by_project(PRIMARY_PROJECT)
            .expect("verifications");
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|v| v.title == "Test"));
    }

    #[rocket::async_test]
    async fn get_edit_verification_returns_prefilled_form() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/verification/edit/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Edit Verification"));
        assert!(body.contains("value=\"Analysis\""));
    }

    #[rocket::async_test]
    async fn get_edit_verification_redirects_when_project_mismatch() {
        let mut repo = base_repo();
        repo.projects.insert(2, sample_project(2, "Venus"));
        repo.verifications
            .insert(2, sample_verification(2, 2, "Review"));
        let client = test_client(repo).await;
        let response = get_with_session(&client, "/p/1/verification/edit/2", ADMIN_ID).await;
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/2/verification")
        );
    }

    #[rocket::async_test]
    async fn post_edit_verification_updates_existing_entry() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/verification/edit/1",
            "id=1&project_id=1&title=Analysis+Rev&description=Updated+desc&tag=ANALYSIS",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/verification")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let verification = repo.get_verification_by_id(1).expect("verification");
        assert_eq!(verification.title, "Analysis Rev");
        assert_eq!(verification.description, "Updated desc");
    }

    #[rocket::async_test]
    async fn post_edit_verification_redirects_when_project_mismatch() {
        let mut repo = base_repo();
        repo.projects.insert(2, sample_project(2, "Venus"));
        repo.verifications
            .insert(2, sample_verification(2, 2, "Review"));
        let client = test_client(repo).await;
        let response = post_form_with_session(
            &client,
            "/p/1/verification/edit/2",
            "id=2&project_id=1&title=Review&description=Stay&tag=REVIEW",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/2/verification")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let verification = repo.get_verification_by_id(2).expect("verification");
        assert_eq!(verification.project_id, 2);
    }

    #[rocket::async_test]
    async fn delete_verification_route_removes_verification() {
        let client = test_client(base_repo()).await;
        let response = delete_with_session(&client, "/p/1/verification/delete/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let list = repo
            .get_verification_by_project(PRIMARY_PROJECT)
            .expect("verifications");
        assert!(list.is_empty());
    }

    #[rocket::async_test]
    async fn delete_verification_route_redirects_on_project_mismatch() {
        let mut repo = base_repo();
        repo.projects.insert(2, sample_project(2, "Venus"));
        repo.verifications
            .insert(2, sample_verification(2, 2, "Review"));
        let client = test_client(repo).await;
        let response = delete_with_session(&client, "/p/1/verification/delete/2", ADMIN_ID).await;
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/2/verification")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        assert!(repo.get_verification_by_id(2).is_ok());
    }

    #[rocket::async_test]
    async fn delete_verification_route_redirects_when_not_found() {
        let client = test_client(base_repo()).await;
        let response = delete_with_session(&client, "/p/1/verification/delete/99", ADMIN_ID).await;
        // get_by_id fails -> Redirect to show_url
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/verification")
        );
    }
}

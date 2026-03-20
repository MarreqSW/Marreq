// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::helpers::*;
use super::prelude::*;
use rocket::http::Status;

#[get("/<project_id>/verification")]
pub async fn show_verification(
    project_access: HtmlProjectAccess,
    project_id: String,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);
    let verifications = state
        .repo_read()
        .get_verification_methods_by_project(project_id)
        .unwrap_or_default();

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "selected_project_slug": project_slug,
        "verifications": verifications,
        "page_title": "Verification methods"
    });

    Ok(Template::render("verification/verification", ctx))
}

#[get("/<project_id>/verification/new?<error>")]
pub async fn new_verification(
    project_access: HtmlProjectAccess,
    project_id: String,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "selected_project_slug": project_slug,
        "error": error,
        "page_title": "New Verification Method"
    });

    Ok(Template::render("verification/new_verification", ctx))
}

#[post("/<project_id>/verification/new", data = "<form>")]
pub async fn post_verification(
    project_access: HtmlProjectAccess,
    project_id: String,
    form: Form<NewVerificationMethod>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let _user = project_access.into_user();

    let new_url = format!(
        "/p/{project_slug}/verification/new?error=Failed%20to%20create%20verification%20method"
    );
    let show_url = format!("/p/{project_slug}/verification");

    let new_verification = NewVerificationMethod {
        project_id,
        ..form.into_inner()
    };

    if let Err(_err) = state
        .repo_write()
        .insert_new_verification_method(&new_verification)
    {
        #[cfg(debug_assertions)]
        eprintln!("Error inserting verification method: {_err:?}");
        return Ok(Redirect::to(new_url));
    }

    Ok(Redirect::to(show_url))
}

#[get("/<project_id>/verification/edit/<verification_id>?<error>")]
pub async fn get_edit_verification(
    project_access: HtmlProjectAccess,
    project_id: String,
    verification_id: i32,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);
    let verification = state
        .repo_read()
        .get_verification_method_by_id(verification_id)
        .map_err(|_| Redirect::to(format!("/p/{project_slug}/verification")))?;

    if verification.project_id != project_id {
        let verification_project_slug =
            get_project_slug_by_id_pooled_safe(state, verification.project_id);
        return Err(Redirect::to(format!(
            "/p/{verification_project_slug}/verification"
        )));
    }

    let ctx = json!({
        "verification": verification,
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "selected_project_slug": project_slug,
        "error": error,
        "page_title": format!("Edit {} - Verification method", verification.title)
    });
    Ok(Template::render("verification/edit_verification", ctx))
}

#[post("/<project_id>/verification/edit/<verification_id>", data = "<form>")]
pub async fn post_edit_verification(
    project_access: HtmlProjectAccess,
    project_id: String,
    verification_id: i32,
    form: Form<NewVerificationMethod>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let _user = project_access.into_user();

    let edit_url = format!(
        "/p/{project_slug}/verification/edit/{verification_id}?error=Failed%20to%20update%20verification%20method"
    );
    let show_url = format!("/p/{project_slug}/verification");

    let old = state
        .repo_read()
        .get_verification_method_by_id(verification_id)
        .map_err(|_| Redirect::to(show_url.clone()))?;
    if old.project_id != project_id {
        let old_project_slug = get_project_slug_by_id_pooled_safe(state, old.project_id);
        return Err(Redirect::to(format!("/p/{old_project_slug}/verification")));
    }

    let mut payload = form.into_inner();
    payload.id = Some(verification_id);
    payload.project_id = project_id;

    if let Err(_err) = state.repo_write().edit_verification_method(&payload) {
        #[cfg(debug_assertions)]
        eprintln!("Error updating verification method {verification_id}: {_err:?}");
        return Ok(Redirect::to(edit_url));
    }

    Ok(Redirect::to(show_url))
}

#[delete("/<project_id>/verification/delete/<verification_id>")]
pub async fn delete_verification_route(
    project_access: HtmlProjectAccess,
    project_id: String,
    verification_id: i32,
    state: &State<AppState>,
) -> Result<Status, Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let _user = project_access.into_user();
    let show_url = format!("/p/{project_slug}/verification");
    let verification = state
        .repo_read()
        .get_verification_method_by_id(verification_id)
        .map_err(|_| Redirect::to(show_url.clone()))?;
    if verification.project_id != project_id {
        let verification_project_slug =
            get_project_slug_by_id_pooled_safe(state, verification.project_id);
        return Err(Redirect::to(format!(
            "/p/{verification_project_slug}/verification"
        )));
    }

    match state
        .repo_write()
        .delete_verification_method(verification_id)
    {
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
            slug: name.to_lowercase().replace(' ', "-"),
            group_id: None,
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
        repo.verification_methods
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
        let response = get_with_session(&client, "/p/mars/verification", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Verification methods"));
        assert!(body.contains("#1"));
        assert!(body.contains("Analysis"));
    }

    #[rocket::async_test]
    async fn new_verification_form_renders() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/mars/verification/new", ADMIN_ID).await;
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
            "/p/mars/verification/new",
            "title=Test&description=Test+verification&tag=TEST&project_id=1",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/mars/verification")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let list = repo
            .get_verification_methods_by_project(PRIMARY_PROJECT)
            .expect("verifications");
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|v| v.title == "Test"));
    }

    #[rocket::async_test]
    async fn get_edit_verification_returns_prefilled_form() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/mars/verification/edit/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Edit Verification"));
        assert!(body.contains("value=\"Analysis\""));
    }

    #[rocket::async_test]
    async fn get_edit_verification_redirects_when_project_mismatch() {
        let mut repo = base_repo();
        repo.projects.insert(2, sample_project(2, "Venus"));
        repo.verification_methods
            .insert(2, sample_verification(2, 2, "Review"));
        let client = test_client(repo).await;
        let response = get_with_session(&client, "/p/mars/verification/edit/2", ADMIN_ID).await;
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/venus/verification")
        );
    }

    #[rocket::async_test]
    async fn post_edit_verification_updates_existing_entry() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/mars/verification/edit/1",
            "id=1&project_id=1&title=Analysis+Rev&description=Updated+desc&tag=ANALYSIS",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/mars/verification")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let verification = repo.get_verification_method_by_id(1).expect("verification");
        assert_eq!(verification.title, "Analysis Rev");
        assert_eq!(verification.description, "Updated desc");
    }

    #[rocket::async_test]
    async fn post_edit_verification_redirects_when_project_mismatch() {
        let mut repo = base_repo();
        repo.projects.insert(2, sample_project(2, "Venus"));
        repo.verification_methods
            .insert(2, sample_verification(2, 2, "Review"));
        let client = test_client(repo).await;
        let response = post_form_with_session(
            &client,
            "/p/mars/verification/edit/2",
            "id=2&project_id=1&title=Review&description=Stay&tag=REVIEW",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/venus/verification")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let verification = repo.get_verification_method_by_id(2).expect("verification");
        assert_eq!(verification.project_id, 2);
    }

    #[rocket::async_test]
    async fn delete_verification_route_removes_verification() {
        let client = test_client(base_repo()).await;
        let response =
            delete_with_session(&client, "/p/mars/verification/delete/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let list = repo
            .get_verification_methods_by_project(PRIMARY_PROJECT)
            .expect("verifications");
        assert!(list.is_empty());
    }

    #[rocket::async_test]
    async fn delete_verification_route_redirects_on_project_mismatch() {
        let mut repo = base_repo();
        repo.projects.insert(2, sample_project(2, "Venus"));
        repo.verification_methods
            .insert(2, sample_verification(2, 2, "Review"));
        let client = test_client(repo).await;
        let response =
            delete_with_session(&client, "/p/mars/verification/delete/2", ADMIN_ID).await;
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/venus/verification")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        assert!(repo.get_verification_method_by_id(2).is_ok());
    }

    #[rocket::async_test]
    async fn delete_verification_route_redirects_when_not_found() {
        let client = test_client(base_repo()).await;
        let response =
            delete_with_session(&client, "/p/mars/verification/delete/99", ADMIN_ID).await;
        // get_by_id fails -> Redirect to show_url
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/mars/verification")
        );
    }
}

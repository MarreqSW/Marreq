// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::helpers::*;
use super::prelude::*;
use crate::services::CategoryService;

#[get("/<project_id>/categories")]
async fn show_categories(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);
    let service = CategoryService::new(state.inner());
    let categories = service.list_by_project(project_id).unwrap_or_default();

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "categories": categories,
        "page_title": "Categories"
    });

    Ok(Template::render("categories/categories", ctx))
}

#[get("/<project_id>/categories/new")]
async fn new_category(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "page_title": "New Category"
    });
    Ok(Template::render("categories/new_category", ctx))
}

#[post("/<project_id>/categories/new", data = "<new_category>")]
async fn post_category(
    project_access: ProjectAccess,
    project_id: i32,
    new_category: Form<NewCategory>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let service = CategoryService::new(state.inner());

    let new_url = uri!("/p", new_category(project_id));
    let show_url = uri!("/p", show_categories(project_id));

    let mut category = new_category.into_inner();
    category.project_id = project_id;

    if let Err(_e) = service.create(&user, category) {
        #[cfg(debug_assertions)]
        eprintln!("insert_new_category error: {:?}", _e);
        return Ok(Redirect::to(new_url.clone()));
    }

    Ok(Redirect::to(show_url))
}

#[get("/<project_id>/categories/edit/<category_id>")]
async fn get_edit_category(
    project_access: ProjectAccess,
    project_id: i32,
    category_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let service = CategoryService::new(state.inner());

    let category = service
        .get_by_id(category_id)
        .map_err(|_| Redirect::to(uri!("/p", show_categories(project_id))))?;

    if category.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_categories(project_id = category.project_id)
        )));
    }

    let projects = get_accessible_projects(state, &user);

    let ctx = json!({
        "categories": category,
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "page_title": format!("Edit {} - Category", category.title)
    });

    Ok(Template::render("categories/edit_category", ctx))
}

#[post("/<project_id>/categories/edit/<category_id>", data = "<category>")]
async fn post_edit_category(
    project_access: ProjectAccess,
    project_id: i32,
    category_id: i32,
    category: Form<NewCategory>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let service = CategoryService::new(state.inner());

    let edit_url = uri!("/p", get_edit_category(project_id, category_id));
    let show_url = uri!("/p", show_categories(project_id));

    let old = service
        .get_by_id(category_id)
        .map_err(|_| Redirect::to(show_url.clone()))?;

    if old.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_categories(project_id = old.project_id)
        )));
    }

    let mut edited = category.into_inner();
    edited.id = Some(category_id);
    edited.project_id = project_id;

    if let Err(_e) = service.update(&user, category_id, edited) {
        #[cfg(debug_assertions)]
        eprintln!("edit_category error: {:?}", _e);
        return Ok(Redirect::to(edit_url.clone()));
    }

    Ok(Redirect::to(show_url))
}

#[delete("/<project_id>/categories/delete/<category_id>")]
async fn delete_category_route(
    project_access: ProjectAccess,
    project_id: i32,
    category_id: i32,
    state: &State<AppState>,
) -> Result<rocket::http::Status, Redirect> {
    let user = project_access.into_user();
    let service = CategoryService::new(state.inner());

    let category = match service.get_by_id(category_id) {
        Ok(c) => c,
        Err(_) => return Ok(rocket::http::Status::NotFound),
    };

    if category.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_categories(project_id = category.project_id)
        )));
    }

    match service.delete(&user, category_id) {
        Ok(_) => Ok(rocket::http::Status::Ok),
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("delete_category error: {:?}", _e);
            Ok(rocket::http::Status::InternalServerError)
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![
        show_categories,
        new_category,
        post_category,
        get_edit_category,
        post_edit_category,
        delete_category_route
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Category, Project, ProjectMember};
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

    fn sample_category(id: i32, project_id: i32, title: &str) -> Category {
        Category {
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
            .insert(PRIMARY_PROJECT, sample_project(PRIMARY_PROJECT, "Orbiter"));
        repo.categories
            .insert(1, sample_category(1, PRIMARY_PROJECT, "Systems"));
        repo.project_members.push(ProjectMember {
            project_id: PRIMARY_PROJECT,
            user_id: ADMIN_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo
    }

    fn repo_with_secondary_category() -> DieselRepoMock {
        let mut repo = base_repo();
        repo.projects.insert(2, sample_project(2, "Lander"));
        repo.categories.insert(2, sample_category(2, 2, "Surface"));
        repo
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        client_with_routes(
            repo,
            routes![
                show_categories,
                new_category,
                post_category,
                get_edit_category,
                post_edit_category,
                delete_category_route
            ],
        )
        .await
    }

    #[rocket::async_test]
    async fn show_categories_lists_known_items() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/categories", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Categories"));
        assert!(body.contains("Systems"));
        assert!(body.contains("Description for Systems"));
    }

    #[rocket::async_test]
    async fn new_category_form_renders() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/categories/new", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("New Category"));
        assert!(body.contains("Create Category"));
    }

    #[rocket::async_test]
    async fn post_category_creates_new_entry() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/categories/new",
            "title=Avionics&description=Avionics+systems&tag=avionics&project_id=1",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/categories")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let categories = repo
            .get_categories_by_project(PRIMARY_PROJECT)
            .expect("categories");
        assert_eq!(categories.len(), 2);
        assert!(categories.iter().any(|cat| cat.title == "Avionics"));
    }

    #[rocket::async_test]
    async fn get_edit_category_renders_existing_data() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/categories/edit/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Edit Category"));
        assert!(body.contains("value=\"Systems\""));
    }

    #[rocket::async_test]
    async fn get_edit_category_redirects_on_project_mismatch() {
        let client = test_client(repo_with_secondary_category()).await;
        let response = get_with_session(&client, "/p/1/categories/edit/2", ADMIN_ID).await;
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/2/categories")
        );
    }

    #[rocket::async_test]
    async fn post_edit_category_updates_existing_entry() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/categories/edit/1",
            "id=1&project_id=1&title=Systems+Rev&description=Updated&tag=systems",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/categories")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let category = repo.get_category_by_id(1).expect("category");
        assert_eq!(category.title, "Systems Rev");
        assert_eq!(category.description, "Updated");
    }

    #[rocket::async_test]
    async fn post_edit_category_redirects_when_category_missing() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/categories/edit/99",
            "id=99&project_id=1&title=Ghost&description=None&tag=ghost",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/categories")
        );
    }

    #[rocket::async_test]
    async fn post_edit_category_redirects_when_project_mismatch() {
        let client = test_client(repo_with_secondary_category()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/categories/edit/2",
            "id=2&project_id=1&title=Surface&description=Stay&tag=surface",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/2/categories")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let category = repo.get_category_by_id(2).expect("category");
        assert_eq!(category.project_id, 2);
    }

    #[rocket::async_test]
    async fn delete_category_route_removes_category() {
        let client = test_client(base_repo()).await;
        let response = delete_with_session(&client, "/p/1/categories/delete/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let categories = repo
            .get_categories_by_project(PRIMARY_PROJECT)
            .expect("categories");
        assert!(categories.is_empty());
    }

    #[rocket::async_test]
    async fn delete_category_route_redirects_on_project_mismatch() {
        let client = test_client(repo_with_secondary_category()).await;
        let response = delete_with_session(&client, "/p/1/categories/delete/2", ADMIN_ID).await;
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/2/categories")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        assert!(repo.get_category_by_id(2).is_ok());
    }

    #[rocket::async_test]
    async fn delete_category_route_returns_not_found_for_missing() {
        let client = test_client(base_repo()).await;
        let response = delete_with_session(&client, "/p/1/categories/delete/99", ADMIN_ID).await;
        assert_eq!(response.status(), Status::NotFound);
    }
}

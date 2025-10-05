use super::helpers::*;
use super::prelude::*;

#[get("/<project_id>/categories")]
async fn show_categories(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = get_accessible_projects(state, &user);
    let categories = state
        .repo_read()
        .get_categories_by_project(project_id)
        .unwrap_or_default();

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "categories": categories,
    });

    Ok(Template::render("categories", ctx))
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
        "selected_project_id": project_id
    });
    Ok(Template::render("new_category", ctx))
}

#[post("/<project_id>/categories/new", data = "<new_category>")]
async fn post_category(
    project_access: ProjectAccess,
    project_id: i32,
    new_category: Form<NewCategory>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user_id = project_access.into_user().user_id;

    let new_url = uri!("/p", new_category(project_id));
    let show_url = uri!("/p", show_categories(project_id));

    let mut category = new_category.into_inner();
    category.project_id = project_id;

    let category_id = state
        .repo_write()
        .insert_new_category(&category)
        .map_err(|_e| {
            #[cfg(debug_assertions)]
            eprintln!("insert_new_category error: {:?}", _e);
            Redirect::to(new_url.clone())
        })?;

    if let Ok(mut conn) = get_db_connection(state) {
        if let Ok(full) = state.repo_read().get_category_by_id(category_id) {
            let log_ctx = LogCtx::new(user_id);
            let _ = Logger::created(&mut conn, &log_ctx, category_id, &full);
        }
    }

    Ok(Redirect::to(show_url))
}

#[get("/<project_id>/categories/edit/<cat_id>")]
async fn get_edit_category(
    project_access: ProjectAccess,
    project_id: i32,
    cat_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    let category = state
        .repo_read()
        .get_category_by_id(cat_id)
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
        "selected_project_id": project_id
    });

    Ok(Template::render("edit_category", ctx))
}

#[post("/<project_id>/categories/edit/<cat_id>", data = "<category>")]
async fn post_edit_category(
    project_access: ProjectAccess,
    project_id: i32,
    cat_id: i32,
    category: Form<NewCategory>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user_id = project_access.into_user().user_id;

    let edit_url = uri!("/p", get_edit_category(project_id, cat_id));
    let show_url = uri!("/p", show_categories(project_id));

    let old = state
        .repo_read()
        .get_category_by_id(cat_id)
        .map_err(|_| Redirect::to(show_url.clone()))?;

    if old.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_categories(project_id = old.project_id)
        )));
    }

    let mut edited = category.into_inner();
    edited.cat_id = Some(cat_id);
    edited.project_id = project_id;

    state.repo_write().edit_category(&edited).map_err(|_e| {
        #[cfg(debug_assertions)]
        eprintln!("edit_category error: {:?}", _e);
        Redirect::to(edit_url.clone())
    })?;

    if let Ok(mut conn) = get_db_connection(state) {
        if let Ok(new_row) = state.repo_read().get_category_by_id(cat_id) {
            let log_ctx = LogCtx::new(user_id);
            let _ = Logger::updated(&mut conn, &log_ctx, &old, &new_row);
        }
    }

    Ok(Redirect::to(show_url))
}

#[delete("/<project_id>/categories/delete/<cat_id>")]
async fn delete_category_route(
    project_access: ProjectAccess,
    project_id: i32,
    cat_id: i32,
    state: &State<AppState>,
) -> Result<rocket::http::Status, Redirect> {
    let user_id = project_access.into_user().user_id;

    let category = match state.repo_read().get_category_by_id(cat_id) {
        Ok(c) => c,
        Err(_) => return Ok(rocket::http::Status::NotFound),
    };

    if category.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_categories(project_id = category.project_id)
        )));
    }

    let deleted = match state.repo_write().delete_category(cat_id) {
        Ok(c) => c,
        Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("delete_category error: {:?}", _e);
            return Ok(rocket::http::Status::InternalServerError);
        }
    };

    if let Ok(mut conn) = get_db_connection(state) {
        let log_ctx = LogCtx::new(user_id);
        let _ = Logger::deleted(conn.as_mut(), &log_ctx, &deleted);
    }

    Ok(rocket::http::Status::Ok)
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
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    const ADMIN_ID: i32 = 1;
    const PRIMARY_PROJECT: i32 = 1;

    fn sample_project(id: i32, name: &str) -> Project {
        Project {
            project_id: id,
            project_name: name.to_string(),
            project_description: Some(format!("{name} project")),
            project_creation_date: Some(timestamp()),
            project_update_date: Some(timestamp()),
            project_status: Some("Active".to_string()),
            project_owner_id: Some(ADMIN_ID),
        }
    }

    fn sample_category(id: i32, project_id: i32, title: &str) -> Category {
        Category {
            cat_id: id,
            cat_title: title.to_string(),
            cat_description: format!("Description for {title}"),
            cat_tag: title.to_ascii_lowercase(),
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
        assert!(body.contains("CAT-1"));
        assert!(body.contains("Systems"));
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
            "cat_title=Avionics&cat_description=Avionics+systems&cat_tag=avionics&project_id=1",
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
        assert!(categories.iter().any(|cat| cat.cat_title == "Avionics"));
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
            "cat_id=1&project_id=1&cat_title=Systems+Rev&cat_description=Updated&cat_tag=systems",
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
        assert_eq!(category.cat_title, "Systems Rev");
        assert_eq!(category.cat_description, "Updated");
    }

    #[rocket::async_test]
    async fn post_edit_category_redirects_when_category_missing() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/categories/edit/99",
            "cat_id=99&project_id=1&cat_title=Ghost&cat_description=None&cat_tag=ghost",
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
            "cat_id=2&project_id=1&cat_title=Surface&cat_description=Stay&cat_tag=surface",
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

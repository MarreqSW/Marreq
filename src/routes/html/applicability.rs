use super::helpers::*;
use super::prelude::*;
use rocket::serde::Serialize;
use rocket::http::Status;

#[derive(Serialize)]
struct ApplicabilityCtx<'a> {
    user: &'a User,
    selected_project_id: i32,
    applicability: Option<Vec<Applicability>>,
}

#[get("/<project_id>/applicability")]
pub async fn show_applicability(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {

    let apps = state
        .repo_read()
        .get_applicability_by_project(project_id)
        .unwrap_or_default();

    let ctx = ApplicabilityCtx {
        user: &project_access.into_user(),
        selected_project_id: project_id,
        applicability: Some(apps),
    };

    Ok(Template::render("applicability", &ctx))
}

#[get("/<project_id>/applicability/new")]
pub async fn new_applicability(
    project_access: ProjectAccess,
    project_id: i32,
) -> Result<Template, Redirect> {
    let ctx = ApplicabilityCtx {
        user: &project_access.into_user(),
        selected_project_id: project_id,
        applicability: None,
    };

    Ok(Template::render("new_applicability", ctx))
}

#[post("/<project_id>/applicability/new", data = "<form>")]
pub async fn post_applicability(
    project_access: ProjectAccess,
    project_id: i32,
    form: Form<NewApplicability>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();

    let new_url  = uri!("/p", new_applicability(project_id = project_id));
    let show_url = uri!("/p", show_applicability(project_id = project_id));

    let new_applicability = NewApplicability {
        project_id,
        ..form.into_inner()
    };

    let applicability_id = match state.repo_write().insert_new_applicability(&new_applicability) {
        Ok(id) => id,
        Err(err) => {
            #[cfg(debug_assertions)]
            eprintln!("Error inserting applicability: {:?}", err);
            return Ok(Redirect::to(new_url));
        }
    };

    let log_ctx = LogCtx::new(user.user_id);
    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(new_url.clone())
    })?;
    let _ = Logger::created(connection, &log_ctx, applicability_id, &new_applicability);

    Ok(Redirect::to(show_url))
}


#[get("/<project_id>/applicability/edit/<app_id>")]
pub async fn get_edit_applicability(
    project_access: ProjectAccess,
    project_id: i32,
    app_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let applicability = get_applicability_by_id_cached(state, app_id);

    if applicability.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_applicability(project_id = applicability.project_id)
        )));
    }

    let ctx = json!({
        "applicability": applicability,
        "user": user,
        "selected_project_id": project_id
    });
    Ok(Template::render("edit_applicability", ctx))
}

#[post("/<project_id>/applicability/edit/<app_id>", data = "<form>")]
pub async fn post_edit_applicability(
    project_access: ProjectAccess,
    project_id: i32,
    app_id: i32,
    form: Form<NewApplicability>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();

    let edit_url = uri!(
        "/p",
        get_edit_applicability(project_id = project_id, app_id = app_id)
    );
    let show_url = uri!("/p", show_applicability(project_id = project_id));

    let old = get_applicability_by_id_cached(state, app_id);
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

    if let Err(err) = state.repo_write().edit_applicability(&new) {
        #[cfg(debug_assertions)]
        eprintln!("Error updating applicability {app_id}: {err:?}");
        return Ok(Redirect::to(edit_url));
    }

    let updated = state
        .repo_read()
        .get_applicability_by_id(app_id)
        .expect("Error reading table Applicability after update");

    let log_ctx = LogCtx::new(user.user_id);
    let conn = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(edit_url.clone())
    })?;
    let _ = Logger::updated(conn, &log_ctx, &old, &updated);

    Ok(Redirect::to(show_url))
}


#[delete("/<project_id>/applicability/delete/<app_id>")]
pub async fn delete_applicability_route(
    project_access: ProjectAccess,
    project_id: i32,
    app_id: i32,
    state: &State<AppState>,
) -> Result<Status, Redirect> {
    let user = project_access.into_user();
    let show_url = uri!("/p", show_applicability(project_id = project_id));

    let applicability = get_applicability_by_id_cached(state, app_id);
    if applicability.project_id != project_id {
        return Err(Redirect::to(uri!(
            "/p",
            show_applicability(project_id = applicability.project_id)
        )));
    }

    let mut conn = get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(show_url.clone())
    })?;

    let deleted = match state.repo_write().delete_applicability(app_id) {
        Ok(app) => app,
        Err(err) => {
            #[cfg(debug_assertions)]
            eprintln!("Error deleting applicability {app_id}: {err:?}");
            return Ok(Status::InternalServerError);
        }
    };

    let log_ctx = LogCtx::new(user.user_id);
    let _ = Logger::deleted(conn.as_mut(), &log_ctx, &deleted);

    Ok(Status::Ok)
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
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::models::{Applicability, Project, ProjectMember};
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use chrono::{NaiveDate, NaiveDateTime};
    use rocket::http::{ContentType, Cookie, Status};
    use rocket::local::asynchronous::{Client, LocalResponse};
    use rocket_dyn_templates::Template;
    use std::sync::{Arc, RwLock};

    type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;
    const PRIMARY_PROJECT: i32 = 1;

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

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

    fn sample_applicability(id: i32, project_id: i32, title: &str) -> Applicability {
        Applicability {
            app_id: id,
            app_title: title.to_string(),
            app_description: format!("Description for {title}"),
            app_tag: title.to_ascii_lowercase(),
            project_id,
        }
    }

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();
        let mut admin = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        admin.is_admin = true;
        repo.users.insert(ADMIN_ID, admin);
        repo.projects.insert(PRIMARY_PROJECT, sample_project(PRIMARY_PROJECT, "Mars"));
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

    fn managed_state(repo: DieselRepoMock) -> TestAppState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(managed_state(repo))
            .attach(Template::fairing())
            .mount(
                "/p",
                routes![
                    show_applicability,
                    new_applicability,
                    post_applicability,
                    get_edit_applicability,
                    post_edit_applicability,
                    delete_applicability_route
                ],
            );
        Client::tracked(rocket).await.expect("client")
    }

    fn auth_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
        cookie.set_path("/");
        cookie
    }

    async fn get<'c>(client: &'c Client, path: &'c str) -> LocalResponse<'c> {
        client
            .get(path)
            .private_cookie(auth_cookie())
            .dispatch()
            .await
    }

    async fn post_form<'c>(client: &'c Client, path: &'c str, body: &'c str) -> LocalResponse<'c> {
        client
            .post(path)
            .header(ContentType::Form)
            .private_cookie(auth_cookie())
            .body(body)
            .dispatch()
            .await
    }

    #[rocket::async_test]
    async fn show_applicability_renders_known_items() {
        let client = test_client(base_repo()).await;
        let response = get(&client, "/p/1/applicability").await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("APP-1"));
        assert!(body.contains("Flight"));
    }

    #[rocket::async_test]
    async fn new_applicability_form_renders() {
        let client = test_client(base_repo()).await;
        let response = get(&client, "/p/1/applicability/new").await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("New Applicability"));
        assert!(body.contains("Create Applicability"));
    }

    #[rocket::async_test]
    async fn post_applicability_stores_new_entry_even_when_logging_fails() {
        let client = test_client(base_repo()).await;
        let response = post_form(
            &client,
            "/p/1/applicability/new",
            "app_title=Thermal&app_description=Heat+rules&app_tag=thermal&project_id=1",
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/applicability/new")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let items = repo.get_applicability_by_project(PRIMARY_PROJECT).unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|app| app.app_title == "Thermal"));
    }

    #[rocket::async_test]
    async fn get_edit_applicability_returns_prefilled_form() {
        let client = test_client(base_repo()).await;
        let response = get(&client, "/p/1/applicability/edit/1").await;
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
        let response = get(&client, "/p/1/applicability/edit/2").await;
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/2/applicability")
        );
    }

    #[rocket::async_test]
    async fn post_edit_applicability_updates_entry_before_logging_error() {
        let client = test_client(base_repo()).await;
        let response = post_form(
            &client,
            "/p/1/applicability/edit/1",
            "app_id=1&project_id=1&app_title=Flight+Rev&app_description=Updated&app_tag=flight",
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/applicability/edit/1")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let updated = repo.get_applicability_by_id(1).unwrap();
        assert_eq!(updated.app_title, "Flight Rev");
        assert_eq!(updated.app_description, "Updated");
    }

    #[rocket::async_test]
    async fn post_edit_applicability_redirects_when_project_mismatch() {
        let mut repo = base_repo();
        repo.projects.insert(2, sample_project(2, "Venus"));
        repo.applicability
            .insert(2, sample_applicability(2, 2, "Surface"));
        let client = test_client(repo).await;
        let response = post_form(
            &client,
            "/p/1/applicability/edit/2",
            "app_id=2&project_id=2&app_title=Surface&app_description=Planet&app_tag=surface",
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/2/applicability")
        );
    }

    #[rocket::async_test]
    async fn delete_applicability_redirects_when_logging_connection_unavailable() {
        let client = test_client(base_repo()).await;
        let response = client
            .delete("/p/1/applicability/delete/1")
            .private_cookie(auth_cookie())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/applicability")
        );
    }
}

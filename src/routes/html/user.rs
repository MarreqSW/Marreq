use super::helpers::build_context_with_projects;
use super::prelude::*;
use crate::services::UserService;

#[get("/profile?<updated>")]
async fn profile(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
    updated: Option<bool>,
) -> Template {
    let user = session_user.into_inner();
    let mut ctx = build_context_with_projects(state, user.clone(), cookies);

    if let Some(ctx_obj) = ctx.as_object_mut() {
        ctx_obj.insert("user".to_string(), json!(user));
        ctx_obj.insert("title".to_string(), json!("My Profile"));
        ctx_obj.insert(
            "profile_updated".to_string(),
            json!(updated.unwrap_or(false)),
        );
    }

    Template::render("user_profile", ctx)
}

#[get("/profile/edit?<error>")]
async fn edit_profile(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
    error: Option<String>,
) -> Template {
    let user = session_user.into_inner();
    let mut ctx = build_context_with_projects(state, user.clone(), cookies);

    if let Some(ctx_obj) = ctx.as_object_mut() {
        ctx_obj.insert("user".to_string(), json!(user));
        ctx_obj.insert("title".to_string(), json!("Edit Profile"));
        if let Some(error_msg) = error {
            ctx_obj.insert("profile_error".to_string(), json!(error_msg));
        }
    }

    Template::render("edit_profile", ctx)
}

#[post("/profile/edit", data = "<user_form>")]
async fn post_edit_profile(
    session_user: SessionUser,
    user_form: Form<UpdateUser>,
    state: &State<AppState>,
) -> Redirect {
    let actor = session_user.into_inner();
    let mut user_data = user_form.into_inner();
    user_data.id = Some(actor.id);
    user_data.is_admin = actor.is_admin;

    let service = UserService::new(state.inner());

    if service.update_without_password(&actor, &user_data).is_ok() {
        Redirect::to(uri!(profile(updated = Some(true))))
    } else {
        Redirect::to(uri!(edit_profile(
            error = Some("Failed to update profile".to_string())
        )))
    }
}

#[get("/<id>/show")]
async fn show_user_id(
    admin: AdminOnly,
    id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let current_user = admin.into_inner();
    let service = UserService::new(state.inner());

    let user = service
        .get_by_id(id)
        .map_err(|_| Redirect::to(uri!("/dashboard")))?;

    let ctx = json!({
        "user": current_user,
        "name": user.name,
        "username": user.username,
        "email": user.email,
        "id": user.id,
        "creation_date": user.creation_date,
        "last_login": user.last_login,
        "is_admin": user.is_admin
    });

    Ok(Template::render("user_by_id", ctx))
}

#[get("/<id>/edit?<error>")]
async fn edit_user(
    admin: AdminOnly,
    id: i32,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    let current_user = admin.into_inner();
    let service = UserService::new(state.inner());

    let user = service
        .get_by_id(id)
        .map_err(|_| Redirect::to(uri!("/dashboard")))?;

    let ctx = json!({
        "users": user,
        "user": current_user,
        "error": error
    });

    Ok(Template::render("edit_user_by_id", ctx))
}

#[post("/<id>/edit", data = "<user_form>")]
async fn post_edit_user(
    admin: AdminOnly,
    id: i32,
    user_form: Form<UpdateUser>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let mut user_data = user_form.into_inner();
    user_data.id = Some(id);
    let service = UserService::new(state.inner());

    match service.update_without_password(&admin.into_inner(), &user_data) {
        Ok(_) => Ok(Redirect::to(uri!(show_user_id(id)))),
        Err(_) => Ok(Redirect::to(uri!(edit_user(
            id = id,
            error = Some("Failed to update user".to_string())
        )))),
    }
}

#[get("/new?<error>")]
async fn new_user(
    admin: AdminOnly,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    let user = admin.into_inner();
    let status = state.repo_read().get_status_all().unwrap_or_default();
    let status_json = json!(status);

    let ctx = json!({
        "status": status_json,
        "user": user,
        "error": error
    });
    Ok(Template::render("new_user", ctx))
}

#[post("/new", data = "<new_user>")]
async fn post_user(
    admin: AdminOnly,
    new_user: Form<NewUser>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let service = UserService::new(state.inner());
    let mut user_data = new_user.into_inner();

    match hash_password(&user_data.password_hash) {
        Ok(hashed_password) => {
            user_data.password_hash = hashed_password;
            match service.create(&admin.into_inner(), user_data) {
                Ok(id) => Ok(Redirect::to(uri!(show_user_id(id)))),
                Err(_) => Ok(Redirect::to(uri!(new_user(
                    error = Some("Failed to create user".to_string())
                )))),
            }
        }
        Err(_) => Ok(Redirect::to(uri!(new_user(
            error = Some("Password hashing failed".to_string())
        )))),
    }
}

pub fn routes() -> Vec<Route> {
    routes![
        profile,
        edit_profile,
        post_edit_profile,
        show_user_id,
        edit_user,
        post_edit_user,
        new_user,
        post_user
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::{ContentType, Cookie, Status};
    use rocket::local::asynchronous::{Client, LocalResponse};
    use rocket_dyn_templates::Template;
    use std::sync::{Arc, RwLock};

    type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;
    const USER_ID: i32 = 2;

    fn make_admin() -> crate::models::User {
        let mut admin = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        admin.is_admin = true;
        admin.name = "Admin User".into();
        admin.email = "admin@example.com".into();
        admin
    }

    fn make_standard_user() -> crate::models::User {
        let mut user = DieselRepoMock::make_user(USER_ID, "jane", "");
        user.name = "Jane Doe".into();
        user.email = "jane@example.com".into();
        user
    }

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(ADMIN_ID, make_admin());
        repo.users.insert(USER_ID, make_standard_user());
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
                "/user",
                routes![
                    profile,
                    edit_profile,
                    post_edit_profile,
                    show_user_id,
                    edit_user,
                    post_edit_user,
                    new_user,
                    post_user
                ],
            );
        Client::tracked(rocket).await.expect("client")
    }

    fn admin_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
        cookie.set_path("/");
        cookie
    }

    fn user_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, USER_ID.to_string());
        cookie.set_path("/");
        cookie
    }

    async fn get<'c>(client: &'c Client, path: &'c str) -> LocalResponse<'c> {
        client
            .get(path)
            .private_cookie(admin_cookie())
            .dispatch()
            .await
    }

    async fn get_as_standard_user<'c>(client: &'c Client, path: &'c str) -> LocalResponse<'c> {
        client
            .get(path)
            .private_cookie(user_cookie())
            .dispatch()
            .await
    }

    #[rocket::async_test]
    async fn show_user_id_displays_profile_information() {
        let client = test_client(base_repo()).await;
        let response = get(&client, "/user/2/show").await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("User profile"));
        assert!(body.contains("Jane Doe"));
        assert!(body.contains("jane@example.com"));
    }

    #[rocket::async_test]
    async fn edit_user_form_renders_existing_values() {
        let client = test_client(base_repo()).await;
        let response = get(&client, "/user/2/edit").await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Edit User"));
        assert!(body.contains("value=\"Jane Doe\""));
        assert!(body.contains("value=\"jane@example.com\""));
    }

    #[rocket::async_test]
    async fn new_user_page_renders_creation_form() {
        let client = test_client(base_repo()).await;
        let response = get(&client, "/user/new").await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("New User"));
        assert!(body.contains("Create User"));
    }

    #[rocket::async_test]
    async fn profile_page_displays_current_user_information() {
        let client = test_client(base_repo()).await;
        let response = get_as_standard_user(&client, "/user/profile").await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("My Profile"));
        assert!(body.contains("Jane Doe"));
        assert!(body.contains("jane@example.com"));
    }

    #[rocket::async_test]
    async fn edit_profile_form_prefills_user_details() {
        let client = test_client(base_repo()).await;
        let response = get_as_standard_user(&client, "/user/profile/edit").await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Edit Profile"));
        assert!(body.contains("value=\"Jane Doe\""));
        assert!(body.contains("value=\"jane@example.com\""));
    }

    #[rocket::async_test]
    async fn edit_profile_updates_user_information() {
        let client = test_client(base_repo()).await;
        let response = client
            .post("/user/profile/edit")
            .header(ContentType::Form)
            .body(
                "name=Jane+Updated&username=jane_updated&email=jane_updated%40example.com&is_admin=false&id=2",
            )
            .private_cookie(user_cookie())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/profile?updated=true")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo_read();
        let updated = repo.get_user_by_id(USER_ID).expect("user");
        assert_eq!(updated.name, "Jane Updated");
        assert_eq!(updated.username, "jane_updated");
        assert_eq!(updated.email, "jane_updated@example.com");
    }
}

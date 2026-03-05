// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

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
        ctx_obj.insert("page_title".to_string(), json!("My Profile"));
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
        ctx_obj.insert("page_title".to_string(), json!("Edit Profile"));
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

#[get("/<user_id>/show")]
async fn show_user_id(
    admin: AdminOnly,
    user_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let current_user = admin.into_inner();
    let service = UserService::new(state.inner());

    let user = service
        .get_by_id(user_id)
        .map_err(|_| Redirect::to(uri!("/dashboard")))?;

    let ctx = json!({
        "user": current_user,
        "name": user.name,
        "username": user.username,
        "email": user.email,
        "id": user.id,
        "creation_date": user.creation_date,
        "last_login": user.last_login,
        "is_admin": user.is_admin,
        "can_delete": current_user.id != user.id,
        "page_title": format!("{} - User Profile", user.name)
    });

    Ok(Template::render("user_by_id", ctx))
}

#[delete("/<user_id>/delete")]
async fn delete_user_route(
    admin: AdminOnly,
    user_id: i32,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let current_user = admin.into_inner();
    if current_user.id == user_id {
        return Err(Redirect::to(uri!(
            "/user",
            edit_user(
                user_id = user_id,
                error = Some("You cannot delete your own account.".to_string())
            )
        )));
    }
    let service = UserService::new(state.inner());
    match service.delete(&current_user, user_id) {
        Ok(_) => Ok(Redirect::to("/admin/users")),
        Err(_) => Err(Redirect::to("/admin/users")),
    }
}

#[get("/<user_id>/edit?<error>")]
async fn edit_user(
    admin: AdminOnly,
    user_id: i32,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    let current_user = admin.into_inner();
    let service = UserService::new(state.inner());

    let user = service
        .get_by_id(user_id)
        .map_err(|_| Redirect::to(uri!("/dashboard")))?;

    let ctx = json!({
        "users": user,
        "user": current_user,
        "error": error,
        "page_title": format!("Edit {} - User", user.name)
    });

    Ok(Template::render("edit_user_by_id", ctx))
}

#[post("/<user_id>/edit", data = "<user_form>")]
async fn post_edit_user(
    admin: AdminOnly,
    user_id: i32,
    user_form: Form<UpdateUser>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let mut user_data = user_form.into_inner();
    user_data.id = Some(user_id);
    let service = UserService::new(state.inner());

    match service.update_without_password(&admin.into_inner(), &user_data) {
        Ok(_) => Ok(Redirect::to(uri!("/user", show_user_id(user_id)))),
        Err(_) => Ok(Redirect::to(uri!(
            "/user",
            edit_user(
                user_id = user_id,
                error = Some("Failed to update user".to_string())
            )
        ))),
    }
}

#[get("/<user_id>/change_password?<error>&<success>")]
async fn admin_change_password_page(
    admin: AdminOnly,
    user_id: i32,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
    error: Option<String>,
    success: Option<String>,
) -> Result<Template, Redirect> {
    let current_user = admin.into_inner();
    let target_user = state
        .repo_read()
        .get_user_by_id(user_id)
        .map_err(|_| Redirect::to(uri!("/admin/users")))?;

    let mut ctx = build_context_with_projects(state, current_user.clone(), cookies);
    if let Some(ctx_obj) = ctx.as_object_mut() {
        ctx_obj.insert("target_user".to_string(), json!(target_user));
        ctx_obj.insert("user_id".to_string(), json!(user_id));
        ctx_obj.insert("error".to_string(), json!(error));
        ctx_obj.insert("success".to_string(), json!(success));
        ctx_obj.insert(
            "page_title".to_string(),
            json!(format!("Change password - {}", target_user.username)),
        );
    }

    Ok(Template::render("admin/change_user_password", ctx))
}

#[post("/<user_id>/change_password", data = "<form>")]
async fn post_admin_change_password(
    admin: AdminOnly,
    user_id: i32,
    form: Form<AdminSetPasswordForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let _admin = admin.into_inner();
    let form = form.into_inner();

    if form.new_password != form.confirm_password {
        return Err(Redirect::to(uri!(
            "/user",
            admin_change_password_page(
                user_id = user_id,
                error = Some("New passwords do not match".to_string()),
                success = Option::<String>::None
            )
        )));
    }

    let mut repo = state.repo_write();
    match crate::auth::admin_set_user_password(&mut *repo, user_id, &form.new_password) {
        Ok(()) => Ok(Redirect::to(uri!(
            "/user",
            admin_change_password_page(
                user_id = user_id,
                error = Option::<String>::None,
                success = Some("Password changed successfully".to_string())
            )
        ))),
        Err(err) => {
            let error_msg = match err {
                AuthError::PasswordPolicy(reason) => reason,
                AuthError::Verify(_) => "Password verification failed".to_string(),
                AuthError::Repo(_) => "User not found or database error".to_string(),
                _ => "Failed to set password".to_string(),
            };
            Err(Redirect::to(uri!(
                "/user",
                admin_change_password_page(
                    user_id = user_id,
                    error = Some(error_msg),
                    success = Option::<String>::None
                )
            )))
        }
    }
}

#[get("/new?<error>")]
async fn new_user(
    admin: AdminOnly,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    let user = admin.into_inner();
    let status = state
        .repo_read()
        .get_requirement_status_all()
        .unwrap_or_default();
    let status_json = json!(status);

    let ctx = json!({
        "status": status_json,
        "user": user,
        "error": error,
        "page_title": "New User"
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
    let user_data = new_user.into_inner();

    // Convert NewUser form to UserCreateRequest for password hashing
    let request = UserCreateRequest {
        username: user_data.username,
        name: user_data.name,
        email: user_data.email,
        password: user_data.password_hash, // HTML form uses this field for plain password
        is_admin: user_data.is_admin,
    };

    match service.create(&admin.into_inner(), request) {
        Ok(id) => Ok(Redirect::to(uri!("/user", show_user_id(id)))),
        Err(err) => {
            let error = match err {
                crate::repository::errors::RepoError::BadInput(message) => message,
                crate::repository::errors::RepoError::Duplicate(message) => message,
                _ => "Failed to create user".to_string(),
            };
            Ok(Redirect::to(uri!("/user", new_user(error = Some(error)))))
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![
        profile,
        edit_profile,
        post_edit_profile,
        show_user_id,
        delete_user_route,
        edit_user,
        post_edit_user,
        admin_change_password_page,
        post_admin_change_password,
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
    use rocket::http::{ContentType, Cookie, SameSite, Status};
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
                    delete_user_route,
                    edit_user,
                    post_edit_user,
                    admin_change_password_page,
                    post_admin_change_password,
                    new_user,
                    post_user
                ],
            );
        Client::tracked(rocket).await.expect("client")
    }

    fn admin_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(SameSite::Strict);
        cookie
    }

    fn user_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, USER_ID.to_string());
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(SameSite::Strict);
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

    async fn delete_with_admin<'c>(client: &'c Client, path: &'c str) -> LocalResponse<'c> {
        client
            .delete(path)
            .private_cookie(admin_cookie())
            .dispatch()
            .await
    }

    #[rocket::async_test]
    async fn delete_user_route_removes_user() {
        let client = test_client(base_repo()).await;
        let response = delete_with_admin(&client, "/user/2/delete").await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/admin/users"));

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo_read();
        assert!(repo.get_user_by_id(USER_ID).is_err());
    }

    #[rocket::async_test]
    async fn delete_user_route_forbids_self_delete() {
        let client = test_client(base_repo()).await;
        let response = delete_with_admin(&client, "/user/1/delete").await;

        assert_eq!(response.status(), Status::SeeOther);
        assert!(response
            .headers()
            .get_one("Location")
            .map(|l| l.contains("/user/1/edit"))
            .unwrap_or(false));

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo_read();
        assert!(repo.get_user_by_id(ADMIN_ID).is_ok());
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
        assert!(body.contains("type=\"password\""));
        assert!(body.contains("autocomplete=\"new-password\""));
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

    #[rocket::async_test]
    async fn admin_change_password_page_accessible_by_admin() {
        let client = test_client(base_repo()).await;
        let response = get(&client, "/user/2/change_password").await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Change Password"));
        assert!(body.contains("jane"));
    }

    #[rocket::async_test]
    async fn admin_change_password_success_updates_password() {
        let client = test_client(base_repo()).await;
        let response = client
            .post("/user/2/change_password")
            .header(ContentType::Form)
            .body("new_password=new-password-123&confirm_password=new-password-123")
            .private_cookie(admin_cookie())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::SeeOther);
        let loc = response
            .headers()
            .get_one("Location")
            .expect("Location header");
        assert!(loc.contains("/user/2/change_password"));
        assert!(loc.contains("success"));

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo_read();
        let updated = repo.get_user_by_id(USER_ID).expect("user");
        assert!(crate::auth::verify_password("new-password-123", &updated.password_hash).unwrap());
    }

    #[rocket::async_test]
    async fn admin_change_password_page_forbidden_for_non_admin() {
        let client = test_client(base_repo()).await;
        let response = client
            .get("/user/2/change_password")
            .private_cookie(user_cookie())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn post_admin_change_password_forbidden_for_non_admin() {
        let client = test_client(base_repo()).await;
        let response = client
            .post("/user/2/change_password")
            .header(ContentType::Form)
            .body("new_password=new-password-123&confirm_password=new-password-123")
            .private_cookie(user_cookie())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);
    }
}

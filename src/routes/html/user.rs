use super::helpers::*;
use super::prelude::*;

#[get("/show/<user_id>")]
async fn show_user_id(
    admin: AdminOnly,
    user_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let current_user = admin.into_inner();
    let user = state
        .repo_read()
        .get_user_by_id(user_id)
        .expect("Error reading table Users");
    let ctx = json!({
        "user": current_user,
        "user_name": user.user_name,
        "user_username": user.user_username,
        "user_email": user.user_email,
        "user_id": user.user_id,
        "user_creation_date": user.user_creation_date,
        "user_last_login": user.user_last_login,
        "is_admin": user.is_admin
    });

    Ok(Template::render("user_by_id", ctx))
}

#[get("/edit/<user_id>")]
async fn edit_user(
    admin: AdminOnly,
    user_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let current_user = admin.into_inner();
    let user = state
        .repo_read()
        .get_user_by_id(user_id)
        .expect("Error reading table Users");
    #[cfg(debug_assertions)]
    println!("USer: {:?}", user);
    let ctx = json!({
        "users": user,
        "user": current_user
    });
    #[cfg(debug_assertions)]
    println!("edit user: {:?}", ctx);
    Ok(Template::render("edit_user_by_id", ctx))
}

#[post("/edit/<user_id>", data = "<user_form>")]
async fn post_edit_user(
    admin: AdminOnly,
    user_id: i32,
    user_form: Form<UpdateUser>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let current_user = admin.into_inner();

    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(edit_user(user_id)))
    })?;

    let old_user = state
        .repo_read()
        .get_user_by_id(user_id)
        .expect("Error reading table Users");

    let mut user_data = user_form.into_inner();
    user_data.user_id = Some(user_id);

    match state.repo_write().update_user_without_password(&user_data) {
        Ok(_) => {
            let log_ctx = LogCtx::new(current_user.user_id);
            let _ = Logger::updated(
                connection,
                &log_ctx,
                &old_user,
                &state
                    .repo_read()
                    .get_user_by_id(user_id)
                    .expect("Error reading table Users after update"),
            );
            Ok(Redirect::to(uri!(show_user_id(user_id))))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(edit_user(user_id))))
        }
    }
}

#[get("/new")]
async fn new_user(admin: AdminOnly, state: &State<AppState>) -> Result<Template, Redirect> {
    let user = admin.into_inner();
    let status = state.repo_read().get_status_all().unwrap_or_default();
    let status_json = json!(status);

    let ctx = json!({
        "status": status_json,
        "user": user
    });
    Ok(Template::render("new_user", ctx))
}

#[post("/new", data = "<new_user>")]
async fn post_user(
    admin: AdminOnly,
    new_user: Form<NewUser>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let connection = &mut get_db_connection(state).map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(new_user))
    })?;

    let mut user_with_hashed_password = new_user.into_inner();
    match hash_password(&user_with_hashed_password.user_password) {
        Ok(hashed_password) => {
            user_with_hashed_password.user_password = hashed_password;
            let user_id = state
                .repo_write()
                .insert_user(&user_with_hashed_password)
                .map_err(|e| {
                    eprintln!("Error inserting new user: {:?}", e);
                    Redirect::to(uri!(new_user))
                })?;

            let user = state
                .repo_read()
                .get_user_by_id(user_id)
                .expect("Error reading table Users");
            let log_ctx = LogCtx::new(admin.into_inner().user_id);
            let _ = Logger::created(connection, &log_ctx, user_id, &user);

            Ok(Redirect::to(uri!(show_user_id(user_id))))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(new_user)))
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![
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
        admin.user_name = "Admin User".into();
        admin.user_email = "admin@example.com".into();
        admin
    }

    fn make_standard_user() -> crate::models::User {
        let mut user = DieselRepoMock::make_user(USER_ID, "jane", "");
        user.user_name = "Jane Doe".into();
        user.user_email = "jane@example.com".into();
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
                routes![show_user_id, edit_user, post_edit_user, new_user, post_user],
            );
        Client::tracked(rocket).await.expect("client")
    }

    fn admin_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
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

    async fn post_form<'c>(client: &'c Client, path: &'c str, body: &'c str) -> LocalResponse<'c> {
        client
            .post(path)
            .header(ContentType::Form)
            .private_cookie(admin_cookie())
            .body(body)
            .dispatch()
            .await
    }

    #[rocket::async_test]
    async fn show_user_id_displays_profile_information() {
        let client = test_client(base_repo()).await;
        let response = get(&client, "/user/show/2").await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("User profile"));
        assert!(body.contains("Jane Doe"));
        assert!(body.contains("jane@example.com"));
    }

    #[rocket::async_test]
    async fn edit_user_form_renders_existing_values() {
        let client = test_client(base_repo()).await;
        let response = get(&client, "/user/edit/2").await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Edit User"));
        assert!(body.contains("value=\"Jane Doe\""));
        assert!(body.contains("value=\"jane@example.com\""));
    }

    #[rocket::async_test]
    async fn post_edit_user_redirects_when_connection_fails() {
        let client = test_client(base_repo()).await;
        let response = post_form(
            &client,
            "/user/edit/2",
            "user_id=2&user_name=Updated+Name&user_username=jane&user_email=jane%40example.com&is_admin=false",
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/edit/2"));
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
    async fn post_user_redirects_back_to_form_when_connection_fails() {
        let client = test_client(base_repo()).await;
        let response = post_form(
            &client,
            "/user/new",
            "user_username=alex&user_name=Alex+Smith&user_email=alex%40example.com&user_password=pass1234&is_admin=false",
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(response.headers().get_one("Location"), Some("/new"));
    }
}

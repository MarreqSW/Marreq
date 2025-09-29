use super::prelude::*;

fn render_login_error(err: AuthError) -> Template {
    let (title, msg) = match err {
        AuthError::InvalidCredentials => ("Login", "Invalid username or password".to_string()),
        AuthError::Verify(_) => ("Login", "Password verification failed".to_string()),
        AuthError::Db(e) => ("Error", format!("Database error: {e}")),
        AuthError::Audit(_) => ("Login", "Logged in but failed to audit login".to_string()),
        AuthError::NotLoggedIn => ("Login", "Not logged in".to_string()),
        AuthError::InvalidSession => ("Login", "Invalid session".to_string()),
        AuthError::Repo(_) => ("Login", "Internal server error".to_string()),
    };

    Template::render("login", json!({ "title": title, "error": msg }))
}

fn render_change_password_error(err: AuthError) -> Template {
    let (title, msg) = match err {
        AuthError::InvalidCredentials => {
            ("Change Password", "Invalid current password".to_string())
        }
        AuthError::Verify(_) => (
            "Change Password",
            "Password verification failed".to_string(),
        ),
        AuthError::Db(e) => ("Error", format!("Database error: {e}")),
        AuthError::NotLoggedIn => ("Change Password", "Not logged in".to_string()),
        AuthError::InvalidSession => ("Change Password", "Invalid session".to_string()),
        AuthError::Audit(_) => (
            "Change Password",
            "Failed to log password change".to_string(),
        ),
        AuthError::Repo(_) => ("Change Password", "Internal server error".to_string()),
    };

    Template::render("change_password", json!({ "title": title, "error": msg }))
}

#[get("/login")]
pub fn login_page() -> Template {
    let ctx = json!({
        "title": "Login"
    });
    Template::render("login", ctx)
}

#[post("/login", data = "<login_form>")]
pub fn login(
    login_form: Form<LoginForm>,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Redirect, Template> {
    let repo = state.repo_read();

    let form = login_form.into_inner();

    match login_user(&*repo, &form, cookies) {
        Ok(()) => Ok(Redirect::to(uri!(crate::routes::html::dashboard::index))),
        Err(err) => Err(render_login_error(err)),
    }
}

#[get("/logout")]
pub fn logout(cookies: &CookieJar<'_>) -> Redirect {
    logout_user(cookies);
    Redirect::to(uri!(login_page))
}

#[get("/change_password")]
pub fn change_password_page(state: &State<AppState>) -> Template {
    // Get projects for navigation
    let projects = state.repo_read().get_projects_all().unwrap_or_default();
    let selected_project_id: Option<i32> = None; // No project selected on change password page

    let ctx = json!({
        "title": "Change Password",
        "projects": projects,
        "selected_project_id": selected_project_id
    });
    Template::render("change_password", ctx)
}

#[post("/change_password", data = "<password_form>")]
pub fn change_password(
    password_form: Form<ChangePasswordForm>,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Template> {
    // Validate passwords
    if password_form.new_password != password_form.confirm_password {
        let ctx = json!({
            "title": "Change Password",
            "error": "New passwords do not match",
        });
        return Err(Template::render("change_password", ctx));
    }

    if password_form.new_password.len() < 8 {
        let ctx = json!({
            "title": "Change Password",
            "error": "New password must be at least 8 characters long",
        });
        return Err(Template::render("change_password", ctx));
    }

    let mut repo = state.repo_write();

    match change_user_password(
        &mut *repo,
        &password_form.current_password,
        &password_form.new_password,
        cookies,
    ) {
        Ok(()) => {
            let ctx = json!({
                "title": "Change Password",
                "success": "Password changed successfully",
            });
            Ok(Template::render("change_password", ctx))
        }
        Err(err) => Err(render_change_password_error(err)),
    }
}

pub fn routes() -> Vec<Route> {
    routes![
        login_page,
        login,
        logout,
        change_password_page,
        change_password
    ]
}

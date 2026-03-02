// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![allow(clippy::result_large_err)]

use super::prelude::*;

#[get("/login?<error>")]
pub fn login_page(error: Option<String>) -> Template {
    let ctx = json!({
        "page_title": "Login",
        "error": error
    });
    Template::render("login", ctx)
}

#[post("/login", data = "<login_form>")]
pub fn login(
    login_form: Form<LoginForm>,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let mut repo = state.repo_write();
    let form = login_form.into_inner();

    match login_user(&mut *repo, &form, cookies) {
        Ok(()) => Ok(Redirect::to(uri!(crate::routes::html::dashboard::index))),
        Err(err) => {
            let error_msg = match err {
                AuthError::InvalidCredentials => "Invalid username or password",
                AuthError::Verify(_) => "Password verification failed",
                AuthError::Db(_) => "Database error occurred",
                AuthError::Audit(_) => "Login successful but failed to audit",
                AuthError::PasswordPolicy(_) => "Password policy violation",
                AuthError::NotLoggedIn => "Not logged in",
                AuthError::InvalidSession => "Invalid session",
                AuthError::Repo(_) => "Internal server error",
            };
            Err(Redirect::to(uri!(login_page(
                error = Some(error_msg.to_string())
            ))))
        }
    }
}

#[get("/logout")]
pub fn logout(cookies: &CookieJar<'_>, state: &State<AppState>) -> Redirect {
    let mut repo = state.repo_write();
    logout_user(cookies, &mut *repo);
    Redirect::to(uri!(login_page(error = Option::<String>::None)))
}

#[get("/change_password?<error>&<success>")]
pub fn change_password_page(
    state: &State<AppState>,
    error: Option<String>,
    success: Option<String>,
) -> Template {
    // Get projects for navigation
    let projects = state.repo_read().get_projects_all().unwrap_or_default();
    let selected_project_id: Option<i32> = None; // No project selected on change password page

    let ctx = json!({
        "page_title": "Change Password",
        "projects": projects,
        "selected_project_id": selected_project_id,
        "error": error,
        "success": success
    });
    Template::render("change_password", ctx)
}

#[post("/change_password", data = "<password_form>")]
pub fn change_password(
    password_form: Form<ChangePasswordForm>,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    // Validate passwords
    if password_form.new_password != password_form.confirm_password {
        return Err(Redirect::to(uri!(change_password_page(
            error = Some("New passwords do not match".to_string()),
            success = Option::<String>::None
        ))));
    }

    let mut repo = state.repo_write();

    match change_user_password(
        &mut *repo,
        &password_form.current_password,
        &password_form.new_password,
        cookies,
    ) {
        Ok(()) => Ok(Redirect::to(uri!(change_password_page(
            error = Option::<String>::None,
            success = Some("Password changed successfully".to_string())
        )))),
        Err(err) => {
            let error_msg = match err {
                AuthError::InvalidCredentials => "Invalid current password".to_string(),
                AuthError::Verify(_) => "Password verification failed".to_string(),
                AuthError::PasswordPolicy(reason) => reason,
                AuthError::Db(_) => "Database error occurred".to_string(),
                AuthError::NotLoggedIn => "Not logged in".to_string(),
                AuthError::InvalidSession => "Invalid session".to_string(),
                AuthError::Audit(_) => "Failed to log password change".to_string(),
                AuthError::Repo(_) => "Internal server error".to_string(),
            };
            Err(Redirect::to(uri!(change_password_page(
                error = Some(error_msg),
                success = Option::<String>::None
            ))))
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::repository::CacheRepository;
    use rocket::State;
    use std::sync::{Arc, RwLock};

    fn app_state() -> AppState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(
                DieselRepoMock::default(),
                60,
            ))),
        }
    }

    fn state_guard(state: &AppState) -> &State<AppState> {
        State::from(state)
    }

    #[test]
    fn login_page_without_error() {
        let template = login_page(None);
        let rendered = format!("{:?}", template);
        assert!(rendered.contains("login"));
    }

    #[test]
    fn login_page_with_error() {
        let template = login_page(Some("Invalid credentials".to_string()));
        let rendered = format!("{:?}", template);
        assert!(rendered.contains("login"));
    }

    #[test]
    fn change_password_page_without_messages() {
        let state = app_state();
        let template = change_password_page(state_guard(&state), None, None);
        let rendered = format!("{:?}", template);
        assert!(rendered.contains("change_password"));
    }

    #[test]
    fn change_password_page_with_error() {
        let state = app_state();
        let template = change_password_page(
            state_guard(&state),
            Some("Password too short".to_string()),
            None,
        );
        let rendered = format!("{:?}", template);
        assert!(rendered.contains("change_password"));
    }

    #[test]
    fn change_password_page_with_success() {
        let state = app_state();
        let template = change_password_page(
            state_guard(&state),
            None,
            Some("Password changed".to_string()),
        );
        let rendered = format!("{:?}", template);
        assert!(rendered.contains("change_password"));
    }
}

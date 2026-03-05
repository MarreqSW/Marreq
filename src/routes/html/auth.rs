// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![allow(clippy::result_large_err)]

use super::prelude::*;
use rocket::response::Responder;

/// A redirect response that also sends `Clear-Site-Data` to instruct the
/// browser to purge cached data on logout (ASVS V14.3.1).
pub struct ClearSiteDataRedirect {
    inner: Redirect,
}

impl ClearSiteDataRedirect {
    pub fn to<U: TryInto<rocket::http::uri::Reference<'static>>>(uri: U) -> Self {
        Self {
            inner: Redirect::to(uri),
        }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for ClearSiteDataRedirect {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        let mut response = self.inner.respond_to(req)?;
        response.set_header(rocket::http::Header::new(
            "Clear-Site-Data",
            r#""cache", "cookies", "storage""#,
        ));
        Ok(response)
    }
}

#[get("/login?<error>")]
pub fn login_page(cookies: &CookieJar<'_>, error: Option<String>) -> Template {
    // Mint a CSRF token for this unauthenticated session so the layout meta
    // tag is populated and AJAX callers can read it before authentication.
    let csrf_token = crate::auth::csrf::get_or_create_csrf_token(cookies);
    let ctx = json!({
        "page_title": "Login",
        "error": error,
        "csrf_token": csrf_token
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
pub fn logout(cookies: &CookieJar<'_>, state: &State<AppState>) -> ClearSiteDataRedirect {
    let mut repo = state.repo_write();
    logout_user(cookies, &mut *repo);
    ClearSiteDataRedirect::to(uri!(login_page(error = Option::<String>::None)))
}

#[get("/change_password?<error>&<success>")]
pub fn change_password_page(
    state: &State<AppState>,
    cookies: &CookieJar<'_>,
    error: Option<String>,
    success: Option<String>,
) -> Template {
    // Get projects for navigation
    let projects = state.repo_read().get_projects_all().unwrap_or_default();
    let selected_project_id: Option<i32> = None; // No project selected on change password page
    let csrf_token = crate::auth::csrf::get_or_create_csrf_token(cookies);

    let ctx = json!({
        "page_title": "Change Password",
        "projects": projects,
        "selected_project_id": selected_project_id,
        "error": error,
        "success": success,
        "csrf_token": csrf_token
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
    use rocket::local::blocking::Client;
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

    /// Build a minimal Rocket instance and return its cookie jar so that
    /// handlers that call `get_or_create_csrf_token` have a working key.
    fn test_client() -> Client {
        Client::tracked(rocket::build()).expect("valid rocket instance")
    }

    #[test]
    fn login_page_without_error() {
        let client = test_client();
        let cookies = client.cookies();
        let template = login_page(&cookies, None);
        let rendered = format!("{:?}", template);
        assert!(rendered.contains("login"));
    }

    #[test]
    fn login_page_with_error() {
        let client = test_client();
        let cookies = client.cookies();
        let template = login_page(&cookies, Some("Invalid credentials".to_string()));
        let rendered = format!("{:?}", template);
        assert!(rendered.contains("login"));
    }

    #[test]
    fn change_password_page_without_messages() {
        let client = test_client();
        let cookies = client.cookies();
        let state = app_state();
        let template = change_password_page(state_guard(&state), &cookies, None, None);
        let rendered = format!("{:?}", template);
        assert!(rendered.contains("change_password"));
    }

    #[test]
    fn change_password_page_with_error() {
        let client = test_client();
        let cookies = client.cookies();
        let state = app_state();
        let template = change_password_page(
            state_guard(&state),
            &cookies,
            Some("Password too short".to_string()),
            None,
        );
        let rendered = format!("{:?}", template);
        assert!(rendered.contains("change_password"));
    }

    #[test]
    fn change_password_page_with_success() {
        let client = test_client();
        let cookies = client.cookies();
        let state = app_state();
        let template = change_password_page(
            state_guard(&state),
            &cookies,
            None,
            Some("Password changed".to_string()),
        );
        let rendered = format!("{:?}", template);
        assert!(rendered.contains("change_password"));
    }
}

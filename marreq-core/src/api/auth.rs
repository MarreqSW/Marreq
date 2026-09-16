// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! JSON authentication endpoints for SPA / API-only clients (session cookies + CSRF).

use std::net::IpAddr;

use rocket::http::{CookieJar, Status};
use rocket::serde::json::{json, Json};

use crate::api::guards::OptionalSessionUser;
use crate::api::prelude::*;
use crate::auth::csrf::clear_csrf_cookie;
use crate::auth::login::login_user;
use crate::auth::logout::logout_user;
use crate::auth::password::change_user_password;
use crate::auth::rate_limiter::LoginRateLimiter;
use crate::auth::session::clear_session_cookie;
use crate::auth::AuthError;
use crate::models::forms::{ChangePasswordForm, LoginForm};

/// Mint or return the CSRF token for the current anonymous or authenticated session.
/// Safe method — no CSRF body required. SPA calls this before `POST /api/auth/login`.
#[get("/auth/csrf")]
pub fn auth_csrf(cookies: &CookieJar<'_>) -> Json<serde_json::Value> {
    let token = crate::auth::csrf::get_or_create_csrf_token(cookies);
    Json(json!({ "csrf_token": token }))
}

/// JSON login: sets session + CSRF cookies on success (same as HTML form login).
#[post("/auth/login", data = "<body>", format = "json")]
pub fn auth_login(
    body: Json<LoginForm>,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
    limiter: &State<LoginRateLimiter>,
    client_ip: Option<IpAddr>,
) -> ApiResult<(Status, Json<serde_json::Value>)> {
    let form = body.into_inner();
    let ip = client_ip;

    match limiter.check_and_delay(&form.username, ip) {
        crate::auth::rate_limiter::RateLimitOutcome::Locked(_) => {
            return Err(ApiError::BadRequest(
                "Too many failed attempts. Please try again later.".into(),
            ));
        }
        crate::auth::rate_limiter::RateLimitOutcome::Allowed => {}
    }

    let mut repo = state.repo_write();

    // Capture the client IP for the session row; User-Agent extraction would
    // require an extra request guard and is left as a follow-up.
    let ip_str: Option<String> = ip.map(|i| i.to_string());

    match login_user(&mut *repo, &form, cookies, None, ip_str) {
        Ok(user) => {
            limiter.record_success(&form.username, ip);
            Ok((
                Status::Ok,
                Json(json!({
                    "status": "ok",
                    "user": user,
                })),
            ))
        }
        Err(err) => {
            limiter.record_failure(&form.username, ip);
            let msg = match err {
                AuthError::InvalidCredentials => "Invalid username or password",
                AuthError::Verify(_) => "Password verification failed",
                AuthError::Db(_) => "Database error occurred",
                AuthError::Audit(_) => "Login successful but failed to audit",
                AuthError::PasswordPolicy(_) => "Password policy violation",
                AuthError::NotLoggedIn => "Not logged in",
                AuthError::InvalidSession => "Invalid session",
                AuthError::Repo(_) => "Internal server error",
                AuthError::EmailNotVerified => "Email address has not been verified",
            };
            Err(ApiError::BadRequest(msg.into()))
        }
    }
}

/// End session and clear CSRF cookie (same as HTML logout).
#[post("/auth/logout")]
pub fn auth_logout(
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let mut repo = state.repo_write();
    logout_user(cookies, &mut *repo);
    Ok(Json(json!({ "status": "ok" })))
}

/// Change the signed-in user's password. Requires the current password.
/// On success, all sessions (including this one) are revoked and cookies cleared.
#[post("/auth/change-password", data = "<body>", format = "json")]
pub fn auth_change_password(
    opt: OptionalSessionUser,
    body: Json<ChangePasswordForm>,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let _user = opt
        .0
        .ok_or_else(|| ApiError::Unauthorized("not authenticated".into()))?;
    let form = body.into_inner();
    if form.new_password != form.confirm_password {
        return Err(ApiError::BadRequest("Passwords do not match".into()));
    }

    let mut repo = state.repo_write();
    match change_user_password(
        &mut *repo,
        &form.current_password,
        &form.new_password,
        cookies,
    ) {
        Ok(()) => {
            clear_session_cookie(cookies, &mut *repo);
            clear_csrf_cookie(cookies);
            Ok(Json(json!({ "status": "ok" })))
        }
        Err(err) => Err(map_change_password_error(err)),
    }
}

fn map_change_password_error(err: AuthError) -> ApiError {
    match err {
        AuthError::NotLoggedIn | AuthError::InvalidSession => {
            ApiError::Unauthorized("not authenticated".into())
        }
        AuthError::InvalidCredentials => {
            ApiError::BadRequest("Current password is incorrect".into())
        }
        AuthError::PasswordPolicy(msg) => ApiError::BadRequest(msg),
        AuthError::Verify(_) => ApiError::BadRequest("Password verification failed".into()),
        AuthError::Db(_) | AuthError::Repo(_) => ApiError::Internal("Internal server error".into()),
        AuthError::Audit(_) => ApiError::Internal("Internal server error".into()),
        AuthError::EmailNotVerified => {
            ApiError::BadRequest("Email address has not been verified".into())
        }
    }
}

/// Current authenticated user (session cookie). JSON `401` if not logged in (not HTML login page).
#[get("/auth/me")]
pub fn auth_me(opt: OptionalSessionUser) -> ApiResult<Json<crate::models::User>> {
    let user = opt
        .0
        .ok_or_else(|| ApiError::Unauthorized("not authenticated".into()))?;
    Ok(Json(user))
}

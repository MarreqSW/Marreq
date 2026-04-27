// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! JSON authentication endpoints for SPA / API-only clients (session cookies + CSRF).

use std::net::IpAddr;

use rocket::http::{CookieJar, Status};
use rocket::serde::json::{json, Json};

use crate::api::guards::OptionalSessionUser;
use crate::api::prelude::*;
use crate::auth::login::login_user;
use crate::auth::logout::logout_user;
use crate::auth::rate_limiter::LoginRateLimiter;
use crate::auth::AuthError;
use crate::models::forms::LoginForm;

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

    match login_user(&mut *repo, &form, cookies) {
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

/// Current authenticated user (session cookie). JSON `401` if not logged in (not HTML login page).
#[get("/auth/me")]
pub fn auth_me(opt: OptionalSessionUser) -> ApiResult<Json<crate::models::User>> {
    let user = opt
        .0
        .ok_or_else(|| ApiError::Unauthorized("not authenticated".into()))?;
    Ok(Json(user))
}

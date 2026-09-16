// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! JSON authentication endpoints for SPA / API-only clients (session cookies + CSRF).

use std::net::IpAddr;

use rocket::form::FromForm;
use rocket::http::{CookieJar, Status};
use rocket::response::Redirect;
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
use crate::repository::errors::RepoError;

#[derive(FromForm)]
pub struct ExternalStartQuery {
    return_to: Option<String>,
}

#[derive(FromForm)]
pub struct ExternalCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

#[get("/auth/providers")]
pub fn auth_providers() -> Json<crate::auth::ProviderDiscovery> {
    Json(crate::auth::AuthConfig::current().discovery())
}

#[get("/auth/external/<provider>/start?<query..>")]
pub async fn auth_external_start(
    provider: &str,
    query: ExternalStartQuery,
    cookies: &CookieJar<'_>,
) -> ApiResult<Redirect> {
    let provider_config = crate::auth::AuthConfig::current()
        .provider(provider)
        .ok_or_else(|| ApiError::NotFound("authentication provider not found".into()))?;
    let transaction = crate::auth::OAuthTransaction::new(
        provider,
        query.return_to.as_deref().unwrap_or("/"),
        crate::auth::OAuthOperation::Login,
        chrono::Utc::now().naive_utc(),
    )
    .map_err(|_| ApiError::BadRequest("invalid return path".into()))?;
    let url = crate::auth::provider::authorization_url(
        provider_config,
        &transaction,
        &crate::config::AppConfig::current().public_base_url,
    )
    .await
    .map_err(ApiError::Internal)?;
    crate::auth::transaction_cookie::store(cookies, &transaction)
        .map_err(|_| ApiError::Internal("failed to store authentication transaction".into()))?;
    Ok(Redirect::to(url))
}

#[post("/auth/external/<provider>/link", data = "<body>", format = "json")]
pub async fn auth_external_link_start(
    provider: &str,
    body: Json<serde_json::Value>,
    session: OptionalSessionUser,
    cookies: &CookieJar<'_>,
) -> ApiResult<Json<serde_json::Value>> {
    let user = session
        .0
        .ok_or_else(|| ApiError::Unauthorized("not authenticated".into()))?;
    let provider_config = crate::auth::AuthConfig::current()
        .provider(provider)
        .ok_or_else(|| ApiError::NotFound("authentication provider not found".into()))?;
    let return_to = body
        .get("return_to")
        .and_then(|value| value.as_str())
        .unwrap_or("/settings/account");
    let transaction = crate::auth::OAuthTransaction::for_link(
        provider,
        return_to,
        user.id,
        chrono::Utc::now().naive_utc(),
    )
    .map_err(|_| ApiError::BadRequest("invalid return path".into()))?;
    let url = crate::auth::provider::authorization_url(
        provider_config,
        &transaction,
        &crate::config::AppConfig::current().public_base_url,
    )
    .await
    .map_err(ApiError::Internal)?;
    crate::auth::transaction_cookie::store(cookies, &transaction)
        .map_err(|_| ApiError::Internal("failed to store authentication transaction".into()))?;
    Ok(Json(json!({ "authorization_url": url })))
}

#[get("/auth/external/<provider>/callback?<query..>")]
pub async fn auth_external_callback(
    provider: &str,
    query: ExternalCallbackQuery,
    session: OptionalSessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> ApiResult<Redirect> {
    let transaction = crate::auth::transaction_cookie::take(cookies).ok_or_else(|| {
        ApiError::BadRequest("authentication transaction is missing or already used".into())
    })?;
    transaction
        .validate_callback(
            provider,
            query.state.as_deref(),
            chrono::Utc::now().naive_utc(),
        )
        .map_err(|_| {
            ApiError::BadRequest("invalid or expired authentication transaction".into())
        })?;
    if query.error.is_some() {
        return Err(ApiError::BadRequest(
            "authentication was declined by the provider".into(),
        ));
    }
    let code = query
        .code
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("authorization code is missing".into()))?;
    let provider_config = crate::auth::AuthConfig::current()
        .provider(provider)
        .ok_or_else(|| ApiError::NotFound("authentication provider not found".into()))?;
    let external = crate::auth::provider::exchange_code(
        provider_config,
        code,
        &transaction,
        &crate::config::AppConfig::current().public_base_url,
    )
    .await
    .map_err(ApiError::BadRequest)?;
    let mut repo = state
        .try_repo_write()
        .map_err(|_| ApiError::Internal("repository unavailable".into()))?;
    match transaction.operation {
        crate::auth::OAuthOperation::Login => {
            let resolved = crate::services::external_auth_service::resolve_login(
                &mut *repo,
                &external,
                provider_config.auto_register,
            )
            .map_err(map_external_repo_error)?;
            crate::auth::complete_login(&mut *repo, resolved.user, provider, cookies, None, None).map_err(|_| ApiError::Unauthorized("authentication could not be completed".into()))?;
        }
        crate::auth::OAuthOperation::Link => {
            let expected = transaction
                .link_user_id
                .ok_or_else(|| ApiError::BadRequest("invalid linking transaction".into()))?;
            if session.0.as_ref().map(|user| user.id) != Some(expected) {
                return Err(ApiError::Unauthorized(
                    "linking session changed or expired".into(),
                ));
            }
            crate::services::external_auth_service::link_identity(&mut *repo, expected, &external)
                .map_err(map_external_repo_error)?;
        }
    }
    Ok(Redirect::to(transaction.return_to))
}

fn map_external_repo_error(error: RepoError) -> ApiError {
    match error {
        RepoError::Unauthorized => {
            ApiError::Unauthorized("external account is not registered".into())
        }
        RepoError::Duplicate(message) if message == "account_link_required" => ApiError::Conflict(
            "an account already uses this email; sign in and connect this provider".into(),
        ),
        RepoError::Duplicate(message) => ApiError::Conflict(message),
        RepoError::BadInput(message) => ApiError::BadRequest(message),
        _ => ApiError::Internal("external authentication failed".into()),
    }
}

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
    if !crate::auth::AuthConfig::current().password_enabled {
        return Err(ApiError::Gone("password authentication is disabled".into()));
    }
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

#[get("/auth/identities")]
pub fn auth_identities(
    session: OptionalSessionUser,
    state: &State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    use crate::repository::ExternalIdentityRepository;
    let user = session
        .0
        .ok_or_else(|| ApiError::Unauthorized("not authenticated".into()))?;
    let repo = state
        .try_repo_read()
        .map_err(|_| ApiError::Internal("repository unavailable".into()))?;
    let identities = repo.get_identities_for_user(user.id)?.into_iter().map(|identity| json!({
        "id": identity.id, "provider": identity.provider_key, "created_at": identity.created_at, "last_login_at": identity.last_login_at,
    })).collect::<Vec<_>>();
    Ok(Json(
        json!({ "password_configured": user.password_hash.is_some(), "identities": identities }),
    ))
}

#[delete("/auth/identities/<identity_id>")]
pub fn auth_identity_delete(
    identity_id: i32,
    session: OptionalSessionUser,
    state: &State<AppState>,
) -> ApiResult<Status> {
    let user = session
        .0
        .ok_or_else(|| ApiError::Unauthorized("not authenticated".into()))?;
    let mut repo = state
        .try_repo_write()
        .map_err(|_| ApiError::Internal("repository unavailable".into()))?;
    crate::services::external_auth_service::unlink_identity(&mut *repo, user.id, identity_id)
        .map_err(map_external_repo_error)?;
    Ok(Status::NoContent)
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

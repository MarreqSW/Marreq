// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Cloud-only public authentication endpoints: registration, email
//! verification, and password reset. Mounted by [`crate::routes::routes`]
//! into the `marreq-cloud` binary; not present in `marreq-server`.

use rocket::http::Status;
use rocket::serde::json::{json, Json};
use rocket::{get, post};

use crate::services::registration_service::{RegistrationError, RegistrationService, TokenError};
use marreq_core::api::error::{ApiError, ApiResult};
use marreq_core::api::prelude::*;
use marreq_core::models::forms::{
    ForgotPasswordRequest, RegistrationRequest, ResetPasswordRequest,
};

#[post("/auth/register", data = "<body>", format = "json")]
pub fn auth_register(
    body: Json<RegistrationRequest>,
    state: &State<AppState>,
) -> ApiResult<(Status, Json<serde_json::Value>)> {
    let service = RegistrationService::new(state);
    match service.register(body.into_inner()) {
        Ok(_id) => Ok((Status::Created, Json(json!({ "status": "ok" })))),
        Err(RegistrationError::BadInput(msg)) => Err(ApiError::BadRequest(msg)),
        Err(RegistrationError::EmailTaken) => {
            // Avoid disclosing exact reason; surface generic 200 to prevent enumeration.
            Ok((Status::Ok, Json(json!({ "status": "ok" }))))
        }
        Err(RegistrationError::UsernameTaken) => {
            Err(ApiError::BadRequest("username already taken".into()))
        }
        Err(RegistrationError::Repo(e)) => Err(e.into()),
    }
}

#[get("/auth/verify-email?<token>")]
pub fn auth_verify_email(
    token: String,
    state: &State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let service = RegistrationService::new(state);
    match service.verify_email(&token) {
        Ok(_) => Ok(Json(json!({ "status": "ok" }))),
        Err(TokenError::NotFound) | Err(TokenError::Invalid) => {
            Err(ApiError::BadRequest("invalid or already-used token".into()))
        }
        Err(TokenError::Expired) => Err(ApiError::BadRequest("token has expired".into())),
        Err(TokenError::PasswordPolicy(m)) => Err(ApiError::BadRequest(m)),
        Err(TokenError::Repo(e)) => Err(e.into()),
    }
}

#[post("/auth/forgot-password", data = "<body>", format = "json")]
pub fn auth_forgot_password(
    body: Json<ForgotPasswordRequest>,
    state: &State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let service = RegistrationService::new(state);
    // Always report success, regardless of whether the email exists, to avoid
    // user enumeration.
    let _ = service.request_password_reset(&body.email);
    Ok(Json(json!({ "status": "ok" })))
}

#[post("/auth/reset-password", data = "<body>", format = "json")]
pub fn auth_reset_password(
    body: Json<ResetPasswordRequest>,
    state: &State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let payload = body.into_inner();
    let service = RegistrationService::new(state);
    match service.reset_password(&payload.token, &payload.new_password) {
        Ok(_) => Ok(Json(json!({ "status": "ok" }))),
        Err(TokenError::NotFound) | Err(TokenError::Invalid) => {
            Err(ApiError::BadRequest("invalid or already-used token".into()))
        }
        Err(TokenError::Expired) => Err(ApiError::BadRequest("token has expired".into())),
        Err(TokenError::PasswordPolicy(m)) => Err(ApiError::BadRequest(m)),
        Err(TokenError::Repo(e)) => Err(e.into()),
    }
}

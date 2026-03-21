// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! API-specific request guards that avoid triggering HTML 401 catchers.

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::Request;

use crate::app::AppState;
use crate::auth::session::read_session_user_id;
use crate::models::User;
use crate::repository::errors::RepoError;
use crate::repository::UserRepository;

/// Authenticated user for JSON API routes. Missing session yields `Success(None)` so handlers
/// return `ApiError::Unauthorized` (JSON) instead of the global HTML `401` catcher.
pub struct OptionalSessionUser(pub Option<User>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for OptionalSessionUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let cookies = request.cookies();
        let user_id = match read_session_user_id(cookies) {
            Some(id) => id,
            None => return Outcome::Success(OptionalSessionUser(None)),
        };

        let state = match request.rocket().state::<AppState>() {
            Some(s) => s.clone(),
            None => return Outcome::Error((Status::InternalServerError, ())),
        };

        let result = rocket::tokio::task::spawn_blocking(move || {
            let guard = state.try_repo_read()?;
            guard.get_user_by_id(user_id)
        })
        .await;

        match result {
            Ok(Ok(user)) => Outcome::Success(OptionalSessionUser(Some(user))),
            Ok(Err(RepoError::NotFound)) => Outcome::Success(OptionalSessionUser(None)),
            Ok(Err(_)) => Outcome::Error((Status::InternalServerError, ())),
            Err(_) => Outcome::Error((Status::InternalServerError, ())),
        }
    }
}

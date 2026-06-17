// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use std::ops::Deref;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};

use crate::app::AppState;
use crate::auth::{clear_session_cookie_via_state, read_session_user_id_via_state};
use crate::models::User;
use crate::repository::errors::RepoError;
use crate::repository::{GroupMembersRepository, ProjectMembersRepository, UserRepository};

pub(crate) fn session_user_has_project_access(
    state: &AppState,
    user: &User,
    project_id: i32,
) -> Result<bool, RepoError> {
    if user.is_admin {
        return Ok(true);
    }

    let repo = state.try_repo_read()?;
    let memberships = repo.get_projects_for_user(user.id)?;
    Ok(memberships
        .iter()
        .any(|membership| membership.project_id == project_id))
}

pub(crate) fn session_user_has_group_access(
    state: &AppState,
    user: &User,
    group_id: i32,
) -> Result<bool, RepoError> {
    if user.is_admin {
        return Ok(true);
    }

    let repo = state.try_repo_read()?;
    let memberships = repo.get_groups_for_user(user.id)?;
    Ok(memberships
        .iter()
        .any(|membership| membership.group_id == group_id))
}

pub struct SessionUser(pub User);

impl SessionUser {
    pub fn into_inner(self) -> User {
        self.0
    }
}

impl Deref for SessionUser {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for SessionUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let cookies = request.cookies();

        let state = match request.rocket().state::<AppState>() {
            Some(s) => s.clone(),
            None => return Outcome::Error((Status::InternalServerError, ())),
        };

        let user_id = match read_session_user_id_via_state(cookies, &state) {
            Some(user_id) => user_id,
            None => {
                clear_session_cookie_via_state(cookies, &state);
                return Outcome::Error((Status::Unauthorized, ()));
            }
        };

        let state_for_lookup = state.clone();
        let result = rocket::tokio::task::spawn_blocking(move || {
            let guard = state_for_lookup.try_repo_read()?;
            guard.get_user_by_id(user_id)
        })
        .await;

        match result {
            Ok(Ok(user)) => Outcome::Success(SessionUser(user)),
            Ok(Err(RepoError::NotFound)) => {
                clear_session_cookie_via_state(cookies, &state);
                Outcome::Error((Status::Unauthorized, ()))
            }
            Ok(Err(_)) => Outcome::Error((Status::InternalServerError, ())),
            Err(_) => Outcome::Error((Status::InternalServerError, ())),
        }
    }
}

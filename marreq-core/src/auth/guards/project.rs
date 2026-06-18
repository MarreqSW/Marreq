// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use std::ops::Deref;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};

use crate::app::AppState;
use crate::auth::guards::route_params::extract_route_param;
use crate::auth::guards::session::session_user_has_project_access;
use crate::auth::guards::{ApiUserOrBearer, SessionUser};
use crate::models::User;

/// Request guard ensuring the authenticated user may access the requested project.
pub struct ProjectAccess {
    user: User,
    project_id: i32,
}

impl ProjectAccess {
    pub fn user(&self) -> &User {
        &self.user
    }

    pub fn project_id(&self) -> i32 {
        self.project_id
    }

    pub fn into_user(self) -> User {
        self.user
    }

    pub fn into_parts(self) -> (User, i32) {
        (self.user, self.project_id)
    }
}

impl Deref for ProjectAccess {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for ProjectAccess {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let project_id_segment = match extract_route_param(request, "<project_id>") {
            Ok(segment) => segment,
            Err(status) => return Outcome::Error((status, ())),
        };

        let project_id = match project_id_segment.parse::<i32>() {
            Ok(project_id) => project_id,
            Err(_) => {
                return Outcome::Error((Status::BadRequest, ()));
            }
        };

        let state = match request.rocket().state::<AppState>() {
            Some(state) => state,
            None => return Outcome::Error((Status::InternalServerError, ())),
        };

        match request.guard::<SessionUser>().await {
            Outcome::Success(session_user) => {
                let user = session_user.into_inner();
                match session_user_has_project_access(state, &user, project_id) {
                    Ok(true) => Outcome::Success(ProjectAccess { user, project_id }),
                    Ok(false) => Outcome::Error((Status::Forbidden, ())),
                    Err(_) => Outcome::Error((Status::InternalServerError, ())),
                }
            }
            Outcome::Error((status, ())) => Outcome::Error((status, ())),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}

/// Request guard for project-scoped API routes that accept either session or Bearer token.
/// When Bearer token has a project scope, access is restricted to that project only.
pub struct ProjectAccessOrBearer(pub ProjectAccess);

impl Deref for ProjectAccessOrBearer {
    type Target = ProjectAccess;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for ProjectAccessOrBearer {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let project_id_segment = match extract_route_param(request, "<project_id>") {
            Ok(segment) => segment,
            Err(status) => return Outcome::Error((status, ())),
        };

        let project_id = match project_id_segment.parse::<i32>() {
            Ok(id) => id,
            Err(_) => return Outcome::Error((Status::BadRequest, ())),
        };

        let state = match request.rocket().state::<AppState>() {
            Some(s) => s.clone(),
            None => return Outcome::Error((Status::InternalServerError, ())),
        };

        let auth = match request.guard::<ApiUserOrBearer>().await {
            Outcome::Success(a) => a,
            Outcome::Error((s, ())) => return Outcome::Error((s, ())),
            Outcome::Forward(_) => return Outcome::Error((Status::Unauthorized, ())),
        };

        let token_scope = auth.token_project_scope();
        let user = auth.into_api_user().into_parts().0;

        if let Some(scope) = token_scope {
            if scope != project_id {
                return Outcome::Error((Status::Forbidden, ()));
            }
            return Outcome::Success(ProjectAccessOrBearer(ProjectAccess { user, project_id }));
        }

        match session_user_has_project_access(&state, &user, project_id) {
            Ok(true) => Outcome::Success(ProjectAccessOrBearer(ProjectAccess { user, project_id })),
            Ok(false) => Outcome::Error((Status::Forbidden, ())),
            Err(_) => Outcome::Error((Status::InternalServerError, ())),
        }
    }
}

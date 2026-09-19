// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use std::ops::Deref;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};

use crate::app::AppState;
use crate::auth::guards::route_params::extract_route_param;
use crate::auth::guards::session::session_user_has_project_access;
use crate::auth::guards::{
    ApiUserOrBearer, BaselinesRead, BaselinesWrite, RequirementsApprove, RequirementsRead,
    RequirementsWrite, SessionUser, TraceabilityRead, TraceabilityWrite, VerificationsRead,
    VerificationsWrite,
};
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

pub struct ProjectScopedAuth {
    auth: ApiUserOrBearer,
    project_id: i32,
}
impl ProjectScopedAuth {
    pub fn user(&self) -> &User {
        self.auth.user()
    }
    pub fn project_id(&self) -> i32 {
        self.project_id
    }
    pub fn auth(&self) -> &ApiUserOrBearer {
        &self.auth
    }
}
impl Deref for ProjectScopedAuth {
    type Target = ApiUserOrBearer;
    fn deref(&self) -> &Self::Target {
        &self.auth
    }
}

fn authorize_scoped(
    state: &AppState,
    auth: ApiUserOrBearer,
    project_id: i32,
) -> Result<ProjectScopedAuth, Status> {
    if let Some(scope) = auth.token_project_scope() {
        if scope != project_id {
            return Err(Status::Forbidden);
        }
        return Ok(ProjectScopedAuth { auth, project_id });
    }
    match session_user_has_project_access(state, auth.user(), project_id) {
        Ok(true) => Ok(ProjectScopedAuth { auth, project_id }),
        Ok(false) => Err(Status::Forbidden),
        Err(_) => Err(Status::InternalServerError),
    }
}

macro_rules! scoped_project_guard {
    ($name:ident, $auth:ty) => {
        pub struct $name(pub ProjectScopedAuth);
        impl Deref for $name {
            type Target = ProjectScopedAuth;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        #[async_trait]
        impl<'r> FromRequest<'r> for $name {
            type Error = ();
            async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
                let project_id = match extract_route_param(request, "<project_id>")
                    .and_then(|v| v.parse::<i32>().map_err(|_| Status::BadRequest))
                {
                    Ok(v) => v,
                    Err(s) => return Outcome::Error((s, ())),
                };
                let state = match request.rocket().state::<AppState>() {
                    Some(v) => v,
                    None => return Outcome::Error((Status::InternalServerError, ())),
                };
                let auth = match request.guard::<$auth>().await {
                    Outcome::Success(v) => v.into_inner(),
                    Outcome::Error(e) => return Outcome::Error(e),
                    Outcome::Forward(f) => return Outcome::Forward(f),
                };
                match authorize_scoped(state, auth, project_id) {
                    Ok(v) => Outcome::Success(Self(v)),
                    Err(s) => Outcome::Error((s, ())),
                }
            }
        }
    };
}
scoped_project_guard!(ProjectRequirementsRead, RequirementsRead);
scoped_project_guard!(ProjectRequirementsWrite, RequirementsWrite);
scoped_project_guard!(ProjectRequirementsApprove, RequirementsApprove);
scoped_project_guard!(ProjectVerificationsRead, VerificationsRead);
scoped_project_guard!(ProjectVerificationsWrite, VerificationsWrite);
scoped_project_guard!(ProjectTraceabilityRead, TraceabilityRead);
scoped_project_guard!(ProjectTraceabilityWrite, TraceabilityWrite);
scoped_project_guard!(ProjectBaselinesRead, BaselinesRead);
scoped_project_guard!(ProjectBaselinesWrite, BaselinesWrite);

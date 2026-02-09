use std::ops::Deref;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};

use crate::app::AppState;
use crate::auth::{clear_session_cookie, read_session_user_id};
use crate::logger::LogCtx;
use crate::models::User;
use crate::repository::errors::RepoError;
use crate::repository::ProjectMembersRepository;
use crate::repository::UserRepository;

/// Request guard that ensures the user is authenticated and loaded from the database.
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

        let user_id = match read_session_user_id(cookies) {
            Some(user_id) => user_id,
            None => {
                clear_session_cookie(cookies);
                return Outcome::Error((Status::Unauthorized, ()));
            }
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
            Ok(Ok(user)) => Outcome::Success(SessionUser(user)),
            Ok(Err(RepoError::NotFound)) => {
                clear_session_cookie(cookies);
                Outcome::Error((Status::Unauthorized, ()))
            }
            Ok(Err(_)) => Outcome::Error((Status::InternalServerError, ())),
            Err(_) => Outcome::Error((Status::InternalServerError, ())),
        }
    }
}

/// Request guard ensuring the current user has administrator privileges.
pub struct AdminOnly(pub User);

impl AdminOnly {
    pub fn into_inner(self) -> User {
        self.0
    }
}

impl Deref for AdminOnly {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for AdminOnly {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.guard::<SessionUser>().await {
            Outcome::Success(user) => {
                if user.is_admin {
                    Outcome::Success(AdminOnly(user.into_inner()))
                } else {
                    Outcome::Error((Status::Forbidden, ()))
                }
            }
            Outcome::Error((status, ())) => Outcome::Error((status, ())),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}

/// Request guard exposing the authenticated user together with logging context.
pub struct ApiUser {
    user: User,
    log_ctx: LogCtx,
}

impl ApiUser {
    pub fn user(&self) -> &User {
        &self.user
    }

    pub fn log_ctx(&self) -> &LogCtx {
        &self.log_ctx
    }

    pub fn into_parts(self) -> (User, LogCtx) {
        (self.user, self.log_ctx)
    }
}

impl Deref for ApiUser {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.user
    }
}
#[async_trait]
impl<'r> FromRequest<'r> for ApiUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.guard::<SessionUser>().await {
            Outcome::Success(session_user) => {
                let user = session_user.into_inner();
                let log_ctx = LogCtx::from_request(user.id, request);
                Outcome::Success(ApiUser { user, log_ctx })
            }
            Outcome::Error((status, ())) => Outcome::Error((status, ())),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}

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
        let route = match request.route() {
            Some(route) => route,
            None => return Outcome::Error((Status::InternalServerError, ())),
        };

        let route_segments: Vec<_> = route
            .uri
            .path()
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();

        let request_segments: Vec<_> = request
            .uri()
            .path()
            .segments()
            .filter(|segment| !segment.is_empty())
            .collect();

        let project_index = match route_segments
            .iter()
            .position(|segment| *segment == "<project_id>")
        {
            Some(index) => index,
            None => return Outcome::Error((Status::InternalServerError, ())),
        };

        let project_id_segment = match request_segments.get(project_index).copied() {
            Some(segment) => segment,
            None => {
                return Outcome::Error((Status::BadRequest, ()));
            }
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

                if user.is_admin {
                    return Outcome::Success(ProjectAccess { user, project_id });
                }

                let repo = state.repo_read();
                match repo.get_projects_for_user(user.id) {
                    Ok(memberships) => {
                        if memberships
                            .iter()
                            .any(|membership| membership.project_id == project_id)
                        {
                            Outcome::Success(ProjectAccess { user, project_id })
                        } else {
                            Outcome::Error((Status::Forbidden, ()))
                        }
                    }
                    Err(_) => Outcome::Error((Status::InternalServerError, ())),
                }
            }
            Outcome::Error((status, ())) => Outcome::Error((status, ())),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}

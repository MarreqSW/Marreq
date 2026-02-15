use std::ops::Deref;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use sha2::{Digest, Sha256};

use crate::app::AppState;
use crate::auth::{clear_session_cookie, read_session_user_id};
use crate::logger::LogCtx;
use crate::models::User;
use crate::repository::errors::RepoError;
use crate::repository::ApiTokensRepository;
use crate::repository::ProjectMembersRepository;
use crate::repository::UserRepository;

fn hash_api_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    digest
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

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

/// Authenticated user from either session cookie or Bearer API token.
/// Use for API routes that support both browser (session) and headless (Bearer) auth.
/// When auth was via Bearer token with a project scope, `token_project_scope()` returns that project id.
pub struct ApiUserOrBearer {
    api_user: ApiUser,
    /// When Some(p), auth was via Bearer token scoped to project p. When None, session or unscoped token.
    token_project_scope: Option<i32>,
}

impl ApiUserOrBearer {
    pub fn user(&self) -> &User {
        self.api_user.user()
    }

    pub fn log_ctx(&self) -> &LogCtx {
        self.api_user.log_ctx()
    }

    pub fn into_api_user(self) -> ApiUser {
        self.api_user
    }

    /// When auth was via Bearer token with a project scope, returns that project id.
    /// Callers must ensure route's project_id matches this scope when Some.
    pub fn token_project_scope(&self) -> Option<i32> {
        self.token_project_scope
    }
}

impl Deref for ApiUserOrBearer {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        self.api_user.user()
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for ApiUserOrBearer {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Try session first
        match request.guard::<SessionUser>().await {
            Outcome::Success(session_user) => {
                let user = session_user.into_inner();
                let log_ctx = LogCtx::from_request(user.id, request);
                return Outcome::Success(ApiUserOrBearer {
                    api_user: ApiUser { user, log_ctx },
                    token_project_scope: None,
                });
            }
            Outcome::Forward(_) => {}
            Outcome::Error((s, ())) => {
                if s != Status::Unauthorized {
                    return Outcome::Error((s, ()));
                }
            }
        }

        // Try Authorization: Bearer <token>
        let auth_header = match request.headers().get_one("Authorization") {
            Some(h) => h,
            None => return Outcome::Error((Status::Unauthorized, ())),
        };
        let token = match auth_header.strip_prefix("Bearer ") {
            Some(t) => t.trim(),
            None => return Outcome::Error((Status::Unauthorized, ())),
        };
        if token.is_empty() {
            return Outcome::Error((Status::Unauthorized, ()));
        }

        let state = match request.rocket().state::<AppState>() {
            Some(s) => s.clone(),
            None => return Outcome::Error((Status::InternalServerError, ())),
        };

        let token_hash = hash_api_token(token);
        let token_hash_for_lookup = token_hash.clone();

        let result = rocket::tokio::task::spawn_blocking({
            let state = state.clone();
            move || {
                let guard = state.try_repo_read()?;
                guard.get_user_by_token_hash(&token_hash_for_lookup)
            }
        })
        .await;

        let (user, project_scope) = match result {
            Ok(Ok(pair)) => pair,
            Ok(Err(RepoError::NotFound)) => return Outcome::Error((Status::Unauthorized, ())),
            Ok(Err(_)) => return Outcome::Error((Status::InternalServerError, ())),
            Err(_) => return Outcome::Error((Status::InternalServerError, ())),
        };

        // Update last_used_at (best-effort, don't fail request)
        let hash = token_hash;
        let state_update = state.clone();
        rocket::tokio::task::spawn_blocking(move || {
            if let Ok(mut guard) = state_update.try_repo_write() {
                let _ = guard.update_api_token_last_used_at(&hash);
            }
        })
        .await
        .ok();

        let log_ctx = LogCtx::from_optional_request(user.id, Some(request));
        Outcome::Success(ApiUserOrBearer {
            api_user: ApiUser { user, log_ctx },
            token_project_scope: project_scope,
        })
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

        let project_id = match request_segments
            .get(project_index)
            .and_then(|s| s.parse::<i32>().ok())
        {
            Some(id) => id,
            None => return Outcome::Error((Status::BadRequest, ())),
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

        if user.is_admin {
            return Outcome::Success(ProjectAccessOrBearer(ProjectAccess { user, project_id }));
        }

        let repo = state.repo_read();
        match repo.get_projects_for_user(user.id) {
            Ok(memberships) => {
                if memberships.iter().any(|m| m.project_id == project_id) {
                    Outcome::Success(ProjectAccessOrBearer(ProjectAccess { user, project_id }))
                } else {
                    Outcome::Error((Status::Forbidden, ()))
                }
            }
            Err(_) => Outcome::Error((Status::InternalServerError, ())),
        }
    }
}

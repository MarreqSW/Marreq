// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use std::ops::Deref;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use sha2::{Digest, Sha256};

use crate::app::AppState;
use crate::auth::{clear_session_cookie, read_session_user_id};
use crate::logger::LogCtx;
use crate::models::User;
use crate::permissions::{has_group_permission, GroupPermission};
use crate::repository::errors::RepoError;
use crate::repository::ApiTokensRepository;
use crate::repository::ProjectMembersRepository;
use crate::repository::{
    GroupMembersRepository, GroupsRepository, ProjectsRepository, UserRepository,
};

fn hash_api_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    digest
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

fn extract_route_param(request: &Request<'_>, placeholder: &str) -> Result<String, Status> {
    let route = request.route().ok_or(Status::InternalServerError)?;

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

    let project_index = route_segments
        .iter()
        .position(|segment| *segment == placeholder)
        .ok_or(Status::InternalServerError)?;

    request_segments
        .get(project_index)
        .copied()
        .map(|segment| segment.to_string())
        .ok_or(Status::BadRequest)
}

fn session_user_has_project_access(
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

fn session_user_has_group_access(
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

/// Request guard ensuring the authenticated user may access the requested HTML project slug.
pub struct HtmlProjectAccess {
    user: User,
    project_id: i32,
    project_slug: String,
}

impl HtmlProjectAccess {
    pub fn user(&self) -> &User {
        &self.user
    }

    pub fn project_id(&self) -> i32 {
        self.project_id
    }

    pub fn project_slug(&self) -> &str {
        &self.project_slug
    }

    pub fn into_user(self) -> User {
        self.user
    }

    pub fn into_parts(self) -> (User, i32, String) {
        (self.user, self.project_id, self.project_slug)
    }
}

impl Deref for HtmlProjectAccess {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for HtmlProjectAccess {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let project_slug = match extract_route_param(request, "<project_id>") {
            Ok(segment) => segment,
            Err(status) => return Outcome::Error((status, ())),
        };

        let state = match request.rocket().state::<AppState>() {
            Some(state) => state,
            None => return Outcome::Error((Status::InternalServerError, ())),
        };

        let project = {
            let repo = match state.try_repo_read() {
                Ok(repo) => repo,
                Err(_) => return Outcome::Error((Status::InternalServerError, ())),
            };

            match repo.get_project_by_slug(&project_slug) {
                Ok(project) => project,
                Err(RepoError::NotFound) => return Outcome::Error((Status::NotFound, ())),
                Err(_) => return Outcome::Error((Status::InternalServerError, ())),
            }
        };

        match request.guard::<SessionUser>().await {
            Outcome::Success(session_user) => {
                let user = session_user.into_inner();
                match session_user_has_project_access(state, &user, project.id) {
                    Ok(true) => Outcome::Success(HtmlProjectAccess {
                        user,
                        project_id: project.id,
                        project_slug,
                    }),
                    Ok(false) => Outcome::Error((Status::Forbidden, ())),
                    Err(_) => Outcome::Error((Status::InternalServerError, ())),
                }
            }
            Outcome::Error((status, ())) => Outcome::Error((status, ())),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}

/// Request guard that ensures the user can view the addressed group page.
pub struct HtmlGroupAccess {
    user: User,
    group_id: i32,
    group_slug: String,
}

impl HtmlGroupAccess {
    pub fn user(&self) -> &User {
        &self.user
    }

    pub fn group_id(&self) -> i32 {
        self.group_id
    }

    pub fn group_slug(&self) -> &str {
        &self.group_slug
    }

    pub fn into_user(self) -> User {
        self.user
    }

    pub fn into_parts(self) -> (User, i32, String) {
        (self.user, self.group_id, self.group_slug)
    }
}

impl Deref for HtmlGroupAccess {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for HtmlGroupAccess {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let group_slug = match extract_route_param(request, "<group_slug>") {
            Ok(segment) => segment,
            Err(status) => return Outcome::Error((status, ())),
        };

        let state = match request.rocket().state::<AppState>() {
            Some(state) => state,
            None => return Outcome::Error((Status::InternalServerError, ())),
        };

        let group = {
            let repo = match state.try_repo_read() {
                Ok(repo) => repo,
                Err(_) => return Outcome::Error((Status::InternalServerError, ())),
            };

            match repo.get_group_by_slug(&group_slug) {
                Ok(group) => group,
                Err(RepoError::NotFound) => return Outcome::Error((Status::NotFound, ())),
                Err(_) => return Outcome::Error((Status::InternalServerError, ())),
            }
        };

        match request.guard::<SessionUser>().await {
            Outcome::Success(session_user) => {
                let user = session_user.into_inner();
                match session_user_has_group_access(state, &user, group.id) {
                    Ok(true) => Outcome::Success(HtmlGroupAccess {
                        user,
                        group_id: group.id,
                        group_slug,
                    }),
                    Ok(false) => Outcome::Error((Status::Forbidden, ())),
                    Err(_) => Outcome::Error((Status::InternalServerError, ())),
                }
            }
            Outcome::Error((status, ())) => Outcome::Error((status, ())),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}

/// Request guard that ensures the user can manage the addressed group's members/settings.
pub struct HtmlGroupManageAccess(pub HtmlGroupAccess);

impl HtmlGroupManageAccess {
    pub fn group_id(&self) -> i32 {
        self.0.group_id()
    }

    pub fn group_slug(&self) -> &str {
        self.0.group_slug()
    }

    pub fn into_parts(self) -> (User, i32, String) {
        self.0.into_parts()
    }
}

impl Deref for HtmlGroupManageAccess {
    type Target = HtmlGroupAccess;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for HtmlGroupManageAccess {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let access = match request.guard::<HtmlGroupAccess>().await {
            Outcome::Success(access) => access,
            Outcome::Error((status, ())) => return Outcome::Error((status, ())),
            Outcome::Forward(status) => return Outcome::Forward(status),
        };

        let state = match request.rocket().state::<AppState>() {
            Some(state) => state,
            None => return Outcome::Error((Status::InternalServerError, ())),
        };

        let repo = match state.try_repo_read() {
            Ok(repo) => repo,
            Err(_) => return Outcome::Error((Status::InternalServerError, ())),
        };

        if has_group_permission(
            &*repo,
            access.user(),
            access.group_id(),
            GroupPermission::ManageGroupMembers,
        ) {
            Outcome::Success(HtmlGroupManageAccess(access))
        } else {
            Outcome::Error((Status::Forbidden, ()))
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

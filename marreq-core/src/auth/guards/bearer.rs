// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::app::AppState;
use crate::auth::guards::{ApiUser, SessionUser};
use crate::logger::LogCtx;
use crate::models::User;
use crate::repository::errors::RepoError;
use crate::repository::{ApiTokensRepository, DelegatedOAuthRepository};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use sha2::{Digest, Sha256};
use std::ops::Deref;

fn hash_token(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[derive(Debug, Clone)]
pub enum AuthenticationSource {
    Session,
    ApiToken {
        project_scope: Option<i32>,
    },
    DelegatedOAuth {
        client_id: String,
        grant_id: i32,
        scopes: Vec<String>,
    },
}

pub struct ApiUserOrBearer {
    api_user: ApiUser,
    source: AuthenticationSource,
}
impl ApiUserOrBearer {
    pub fn user(&self) -> &User {
        self.api_user.user()
    }
    pub fn log_ctx(&self) -> &LogCtx {
        self.api_user.log_ctx()
    }
    pub fn source(&self) -> &AuthenticationSource {
        &self.source
    }
    pub fn into_api_user(self) -> ApiUser {
        self.api_user
    }
    pub fn token_project_scope(&self) -> Option<i32> {
        match self.source {
            AuthenticationSource::ApiToken { project_scope } => project_scope,
            _ => None,
        }
    }
    pub fn delegated_scopes(&self) -> Option<&[String]> {
        match &self.source {
            AuthenticationSource::DelegatedOAuth { scopes, .. } => Some(scopes),
            _ => None,
        }
    }
    pub fn oauth_client_id(&self) -> Option<&str> {
        match &self.source {
            AuthenticationSource::DelegatedOAuth { client_id, .. } => Some(client_id),
            _ => None,
        }
    }
    pub fn oauth_grant_id(&self) -> Option<i32> {
        match self.source {
            AuthenticationSource::DelegatedOAuth { grant_id, .. } => Some(grant_id),
            _ => None,
        }
    }
}
impl Deref for ApiUserOrBearer {
    type Target = User;
    fn deref(&self) -> &Self::Target {
        self.api_user.user()
    }
}

enum DelegatedPolicy {
    Deny,
    Scope(&'static str),
    Scopes(&'static [&'static str]),
    McpAudit,
}

async fn authenticate(
    request: &Request<'_>,
    policy: DelegatedPolicy,
) -> Outcome<ApiUserOrBearer, ()> {
    match request.guard::<SessionUser>().await {
        Outcome::Success(session) => {
            let user = session.into_inner();
            let log = LogCtx::from_request(user.id, request);
            return Outcome::Success(ApiUserOrBearer {
                api_user: ApiUser::new(user, log),
                source: AuthenticationSource::Session,
            });
        }
        Outcome::Error((status, ())) if status != Status::Unauthorized => {
            return Outcome::Error((status, ()))
        }
        _ => {}
    }
    let token = match request
        .headers()
        .get_one("Authorization")
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        Some(v) => v,
        None => return Outcome::Error((Status::Unauthorized, ())),
    };
    let state = match request.rocket().state::<AppState>() {
        Some(v) => v.clone(),
        None => return Outcome::Error((Status::InternalServerError, ())),
    };
    let token_hash = hash_token(token);
    let lookup = token_hash.clone();
    let result = rocket::tokio::task::spawn_blocking({
        let state = state.clone();
        move || state.try_repo_read()?.get_user_by_token_hash(&lookup)
    })
    .await;
    let (user, source) = match result {
        Ok(Ok((user, project_scope))) => {
            let update_state = state.clone();
            let update_hash = token_hash;
            let _ = rocket::tokio::task::spawn_blocking(move || {
                update_state
                    .try_repo_write()?
                    .update_api_token_last_used_at(&update_hash)
            })
            .await;
            (user, AuthenticationSource::ApiToken { project_scope })
        }
        Ok(Err(RepoError::NotFound)) => {
            let required = match policy {
                DelegatedPolicy::Deny => return Outcome::Error((Status::Forbidden, ())),
                DelegatedPolicy::Scope(s) => Some(s),
                DelegatedPolicy::Scopes(_) => None,
                DelegatedPolicy::McpAudit => None,
            };
            let resource = crate::config::AppConfig::current()
                .mcp_public_url
                .trim_end_matches('/')
                .to_owned();
            let principal = match state.try_repo_read().and_then(|repo| {
                crate::auth::delegated::validate_access(
                    &*repo,
                    token,
                    &resource,
                    required,
                    chrono::Utc::now().naive_utc(),
                )
                .map_err(|_| RepoError::Unauthorized)
            }) {
                Ok(v) => v,
                Err(RepoError::Unauthorized) => return Outcome::Error((Status::Forbidden, ())),
                Err(_) => return Outcome::Error((Status::InternalServerError, ())),
            };
            if let DelegatedPolicy::Scopes(required) = policy {
                if required
                    .iter()
                    .any(|scope| !principal.scopes.iter().any(|held| held == scope))
                {
                    return Outcome::Error((Status::Forbidden, ()));
                }
            }
            let grant_id = principal.grant_id;
            let access_hash = hash_token(token);
            let update_state = state.clone();
            let _ = rocket::tokio::task::spawn_blocking(move || {
                update_state.try_repo_write()?.touch_oauth_access(
                    &access_hash,
                    grant_id,
                    chrono::Utc::now().naive_utc(),
                )
            })
            .await;
            (
                principal.user,
                AuthenticationSource::DelegatedOAuth {
                    client_id: principal.client_id,
                    grant_id,
                    scopes: principal.scopes,
                },
            )
        }
        _ => return Outcome::Error((Status::InternalServerError, ())),
    };
    let log = LogCtx::from_optional_request(user.id, Some(request));
    Outcome::Success(ApiUserOrBearer {
        api_user: ApiUser::new(user, log),
        source,
    })
}

#[async_trait]
impl<'r> FromRequest<'r> for ApiUserOrBearer {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        authenticate(request, DelegatedPolicy::Deny).await
    }
}

macro_rules! scoped_guard {
    ($name:ident, $scope:literal) => {
        pub struct $name(pub ApiUserOrBearer);
        impl Deref for $name {
            type Target = ApiUserOrBearer;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl $name {
            pub fn into_inner(self) -> ApiUserOrBearer {
                self.0
            }
        }
        #[async_trait]
        impl<'r> FromRequest<'r> for $name {
            type Error = ();
            async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
                match authenticate(request, DelegatedPolicy::Scope($scope)).await {
                    Outcome::Success(v) => Outcome::Success(Self(v)),
                    Outcome::Error(e) => Outcome::Error(e),
                    Outcome::Forward(f) => Outcome::Forward(f),
                }
            }
        }
    };
}
scoped_guard!(ProjectsRead, "projects:read");
scoped_guard!(RequirementsRead, "requirements:read");
scoped_guard!(RequirementsWrite, "requirements:write");
scoped_guard!(RequirementsApprove, "requirements:approve");
scoped_guard!(VerificationsRead, "verifications:read");
scoped_guard!(VerificationsWrite, "verifications:write");
scoped_guard!(TraceabilityRead, "traceability:read");
scoped_guard!(TraceabilityWrite, "traceability:write");
scoped_guard!(BaselinesRead, "baselines:read");
scoped_guard!(BaselinesWrite, "baselines:write");

pub struct RequirementsAndBaselinesRead(pub ApiUserOrBearer);
impl Deref for RequirementsAndBaselinesRead {
    type Target = ApiUserOrBearer;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl RequirementsAndBaselinesRead {
    pub fn into_inner(self) -> ApiUserOrBearer {
        self.0
    }
}
#[async_trait]
impl<'r> FromRequest<'r> for RequirementsAndBaselinesRead {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match authenticate(
            request,
            DelegatedPolicy::Scopes(&["requirements:read", "baselines:read"]),
        )
        .await
        {
            Outcome::Success(value) => Outcome::Success(Self(value)),
            Outcome::Error(error) => Outcome::Error(error),
            Outcome::Forward(forward) => Outcome::Forward(forward),
        }
    }
}

pub struct McpAuditAuth(pub ApiUserOrBearer);
impl Deref for McpAuditAuth {
    type Target = ApiUserOrBearer;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[async_trait]
impl<'r> FromRequest<'r> for McpAuditAuth {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match authenticate(request, DelegatedPolicy::McpAudit).await {
            Outcome::Success(v) => Outcome::Success(Self(v)),
            Outcome::Error(e) => Outcome::Error(e),
            Outcome::Forward(f) => Outcome::Forward(f),
        }
    }
}

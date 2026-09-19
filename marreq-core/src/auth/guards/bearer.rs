// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use std::ops::Deref;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};
use sha2::{Digest, Sha256};

use crate::app::AppState;
use crate::auth::guards::{ApiUser, SessionUser};
use crate::logger::LogCtx;
use crate::models::User;
use crate::repository::errors::RepoError;
use crate::repository::ApiTokensRepository;

fn hash_api_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    digest
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

/// Authenticated user from either session cookie or Bearer API token.
/// Use for API routes that support both browser (session) and headless (Bearer) auth.
/// When auth was via Bearer token with a project scope, `token_project_scope()` returns that project id.
pub struct ApiUserOrBearer {
    api_user: ApiUser,
    /// When Some(p), auth was via Bearer token scoped to project p. When None, session or unscoped token.
    token_project_scope: Option<i32>,
    delegated_scopes: Option<Vec<String>>,
    oauth_client_id: Option<String>,
    oauth_grant_id: Option<i32>,
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

    pub fn delegated_scopes(&self) -> Option<&[String]> {
        self.delegated_scopes.as_deref()
    }
    pub fn oauth_client_id(&self) -> Option<&str> {
        self.oauth_client_id.as_deref()
    }
    pub fn oauth_grant_id(&self) -> Option<i32> {
        self.oauth_grant_id
    }
}

fn required_delegated_scope(request: &Request<'_>) -> Option<&'static str> {
    let path = request.uri().path().as_str();
    let write = matches!(
        request.method(),
        rocket::http::Method::Post
            | rocket::http::Method::Put
            | rocket::http::Method::Patch
            | rocket::http::Method::Delete
    );
    if path.contains("/approval") {
        return Some("requirements:approve");
    }
    if path.contains("baseline") {
        return Some(if write {
            "baselines:write"
        } else {
            "baselines:read"
        });
    }
    if path.contains("verification") && !path.contains("verification-method") {
        return Some(if write {
            "verifications:write"
        } else {
            "verifications:read"
        });
    }
    if path.contains("trace") || path.contains("matrix") || path.contains("coverage") {
        return Some(if write {
            "traceability:write"
        } else {
            "traceability:read"
        });
    }
    if path.contains("requirement")
        || path.contains("categories")
        || path.contains("applicability")
        || path.contains("status")
        || path.contains("custom_fields")
        || path.contains("verification-method")
        || path == "/api/mcp/audit"
    {
        return Some(if write && path != "/api/mcp/audit" {
            "requirements:write"
        } else {
            "requirements:read"
        });
    }
    if path == "/api/projects" || path.starts_with("/api/project-from-path") {
        return Some("projects:read");
    }
    None
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
                    api_user: ApiUser::new(user, log_ctx),
                    token_project_scope: None,
                    delegated_scopes: None,
                    oauth_client_id: None,
                    oauth_grant_id: None,
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

        let (user, project_scope, delegated_scopes, oauth_client_id, oauth_grant_id) = match result
        {
            Ok(Ok((user, scope))) => (user, scope, None, None, None),
            Ok(Err(RepoError::NotFound)) => {
                let required = required_delegated_scope(request);
                let expected_resource = format!(
                    "{}/mcp",
                    crate::config::AppConfig::current()
                        .public_base_url
                        .trim_end_matches('/')
                );
                let oauth = match state.try_repo_read().and_then(|repo| {
                    crate::auth::delegated::validate_access(
                        &*repo,
                        token,
                        &expected_resource,
                        required,
                        chrono::Utc::now().naive_utc(),
                    )
                    .map_err(|_| RepoError::Unauthorized)
                }) {
                    Ok(principal) => principal,
                    Err(_) => return Outcome::Error((Status::Unauthorized, ())),
                };
                (
                    oauth.user,
                    None,
                    Some(oauth.scopes),
                    Some(oauth.client_id),
                    Some(oauth.grant_id),
                )
            }
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
            api_user: ApiUser::new(user, log_ctx),
            token_project_scope: project_scope,
            delegated_scopes,
            oauth_client_id,
            oauth_grant_id,
        })
    }
}

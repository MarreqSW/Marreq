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
                    api_user: ApiUser::new(user, log_ctx),
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
            api_user: ApiUser::new(user, log_ctx),
            token_project_scope: project_scope,
        })
    }
}

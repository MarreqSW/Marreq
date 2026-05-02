// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Cloud-only self-service registration, email verification, and password reset.
//!
//! This module lives in the `marreq-cloud` binary crate and is therefore
//! compiled only when building the hosted SaaS deployment. The
//! corresponding routes are mounted by `marreq_cloud::routes::routes()`
//! and are not present in the `marreq-server` binary.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use thiserror::Error;

use marreq_core::app::{AppState, DieselCachedRepo};
use marreq_core::auth::password::hash_password;
use marreq_core::auth::password_policy::{validate_password, PasswordContext};
use marreq_core::models::{
    EmailToken, NewEmailToken, NewUser, NewWorkspace, RegistrationRequest, User,
};
use marreq_core::repository::errors::RepoError;
use marreq_core::repository::{EmailTokensRepository, UserRepository, WorkspacesRepository};
use marreq_core::services::email_sender;
use marreq_core::validation::{sanitize_string, validate_user};

const VERIFY_TOKEN_TTL_HOURS: i64 = 24;
const RESET_TOKEN_TTL_HOURS: i64 = 1;

#[derive(Debug, Error)]
pub enum RegistrationError {
    #[error("invalid input: {0}")]
    BadInput(String),
    #[error("email already registered")]
    EmailTaken,
    #[error("username already taken")]
    UsernameTaken,
    #[error("repository error: {0}")]
    Repo(#[from] RepoError),
}

#[derive(Debug, Error)]
pub enum TokenError {
    #[error("token not found or already used")]
    NotFound,
    #[error("token has expired")]
    Expired,
    #[error("invalid token format")]
    Invalid,
    #[error("repository error: {0}")]
    Repo(#[from] RepoError),
    #[error("password policy violation: {0}")]
    PasswordPolicy(String),
}

pub struct RegistrationService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> RegistrationService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Register a new user (Cloud mode only).
    ///
    /// Steps:
    /// 1. Validate input + password policy.
    /// 2. Insert the user with `email_verified = false` and `is_admin = false`.
    /// 3. Create their personal workspace (slug = username).
    /// 4. Mint a verification token and send the email (best-effort).
    pub fn register(&self, request: RegistrationRequest) -> Result<i32, RegistrationError> {
        validate_password(
            &request.password,
            PasswordContext {
                username: Some(&request.username),
                email: Some(&request.email),
                full_name: Some(&request.name),
            },
        )
        .map_err(|e| RegistrationError::BadInput(e.to_string()))?;

        let password_hash = hash_password(&request.password)
            .map_err(|e| RegistrationError::BadInput(format!("password hashing failed: {e}")))?;

        let mut new_user = NewUser {
            id: None,
            username: request.username,
            name: request.name,
            email: request.email,
            password_hash,
            is_admin: false,
            email_verified: Some(false),
        };
        sanitize_string(&mut new_user.username);
        sanitize_string(&mut new_user.name);
        sanitize_string(&mut new_user.email);
        new_user.username = new_user.username.to_lowercase();
        new_user.email = new_user.email.to_lowercase();

        validate_user(&new_user).map_err(|e| RegistrationError::BadInput(e.to_string()))?;

        let user_id = {
            let mut repo = self.state.repo_write();
            let id = repo.insert_user(&new_user).map_err(map_insert_err)?;

            // Personal workspace named after the user.
            let workspace = NewWorkspace {
                slug: new_user.username.clone(),
                name: new_user.name.clone(),
                owner_user_id: id,
                kind: "personal".into(),
            };
            // If a workspace with that slug already exists (e.g., a stray row),
            // fall back to a numbered variant rather than failing the registration.
            if let Err(RepoError::Duplicate(_)) = repo.insert_workspace(&workspace) {
                let mut suffix = 2u32;
                loop {
                    let candidate = NewWorkspace {
                        slug: format!("{}-{suffix}", new_user.username),
                        name: workspace.name.clone(),
                        owner_user_id: id,
                        kind: "personal".into(),
                    };
                    match repo.insert_workspace(&candidate) {
                        Ok(_) => break,
                        Err(RepoError::Duplicate(_)) if suffix < 100 => suffix += 1,
                        Err(e) => return Err(e.into()),
                    }
                }
            }
            id
        };

        let plain_token = mint_token();
        let token_hash = sha256_hex(&plain_token);
        let expires_at = (Utc::now() + Duration::hours(VERIFY_TOKEN_TTL_HOURS)).naive_utc();
        {
            let mut repo = self.state.repo_write();
            repo.insert_email_token(&NewEmailToken {
                user_id,
                token_hash,
                purpose: EmailToken::PURPOSE_VERIFY_EMAIL.to_string(),
                expires_at,
            })?;
        }

        // Best-effort email; we do not surface SMTP errors to the client.
        let _ = send_verification_email(&new_user.email, &plain_token);
        Ok(user_id)
    }

    /// Consume a verification token, marking the corresponding user as verified.
    pub fn verify_email(&self, token: &str) -> Result<i32, TokenError> {
        consume_token(
            self.state,
            token,
            EmailToken::PURPOSE_VERIFY_EMAIL,
            |repo, user_id| {
                repo.set_user_email_verified(user_id, true)
                    .map_err(TokenError::from)
            },
        )
    }

    /// Issue a password-reset token for the given email. Always reports success
    /// to the caller to avoid leaking which email addresses are registered.
    pub fn request_password_reset(&self, email: &str) -> Result<(), RegistrationError> {
        let normalized = email.trim().to_lowercase();
        let user_opt = {
            let repo = self.state.repo_read();
            repo.get_user_by_email(&normalized)?
        };
        let Some(user) = user_opt else {
            return Ok(());
        };

        let plain_token = mint_token();
        let token_hash = sha256_hex(&plain_token);
        let expires_at = (Utc::now() + Duration::hours(RESET_TOKEN_TTL_HOURS)).naive_utc();
        {
            let mut repo = self.state.repo_write();
            repo.insert_email_token(&NewEmailToken {
                user_id: user.id,
                token_hash,
                purpose: EmailToken::PURPOSE_RESET_PASSWORD.to_string(),
                expires_at,
            })?;
        }
        let _ = send_password_reset_email(&user.email, &plain_token);
        Ok(())
    }

    /// Consume a reset token and update the user's password.
    pub fn reset_password(&self, token: &str, new_password: &str) -> Result<i32, TokenError> {
        let user_id = self.lookup_user_for_token(token, EmailToken::PURPOSE_RESET_PASSWORD)?;
        let user: User = {
            let repo = self.state.repo_read();
            repo.get_user_by_id(user_id).map_err(TokenError::from)?
        };
        validate_password(
            new_password,
            PasswordContext {
                username: Some(&user.username),
                email: Some(&user.email),
                full_name: Some(&user.name),
            },
        )
        .map_err(|e| TokenError::PasswordPolicy(e.to_string()))?;
        let new_hash = hash_password(new_password)
            .map_err(|e| TokenError::PasswordPolicy(format!("hashing failed: {e}")))?;

        consume_token(
            self.state,
            token,
            EmailToken::PURPOSE_RESET_PASSWORD,
            |repo, uid| {
                repo.update_user_password(uid, &new_hash)
                    .map_err(TokenError::from)
            },
        )
    }

    fn lookup_user_for_token(&self, token: &str, purpose: &str) -> Result<i32, TokenError> {
        if token.is_empty() {
            return Err(TokenError::Invalid);
        }
        let token_hash = sha256_hex(token);
        let repo = self.state.repo_read();
        let row = repo
            .find_email_token_by_hash(&token_hash)?
            .ok_or(TokenError::NotFound)?;
        if row.purpose != purpose || row.used_at.is_some() {
            return Err(TokenError::NotFound);
        }
        if row.expires_at < Utc::now().naive_utc() {
            return Err(TokenError::Expired);
        }
        Ok(row.user_id)
    }
}

fn consume_token<F>(
    state: &AppState<DieselCachedRepo>,
    token: &str,
    purpose: &str,
    apply: F,
) -> Result<i32, TokenError>
where
    F: FnOnce(&mut DieselCachedRepo, i32) -> Result<(), TokenError>,
{
    if token.is_empty() {
        return Err(TokenError::Invalid);
    }
    let token_hash = sha256_hex(token);
    let mut repo = state.repo_write();
    let row = repo
        .find_email_token_by_hash(&token_hash)?
        .ok_or(TokenError::NotFound)?;
    if row.purpose != purpose || row.used_at.is_some() {
        return Err(TokenError::NotFound);
    }
    if row.expires_at < Utc::now().naive_utc() {
        return Err(TokenError::Expired);
    }
    apply(&mut repo, row.user_id)?;
    repo.mark_email_token_used(row.id)?;
    Ok(row.user_id)
}

fn map_insert_err(e: RepoError) -> RegistrationError {
    match e {
        RepoError::Duplicate(msg) if msg.to_lowercase().contains("email") => {
            RegistrationError::EmailTaken
        }
        RepoError::Duplicate(_) => RegistrationError::UsernameTaken,
        other => RegistrationError::Repo(other),
    }
}

fn mint_token() -> String {
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn public_base_url() -> String {
    marreq_core::config::AppConfig::try_current()
        .map(|c| c.public_base_url.clone())
        .unwrap_or_else(|| "http://localhost:8000".into())
}

fn send_verification_email(to: &str, token: &str) -> Result<(), email_sender::EmailError> {
    let url = format!(
        "{}/verify-email?token={}",
        public_base_url(),
        urlencoding::encode(token)
    );
    email_sender::send_email(
        to,
        "Verify your Marreq email",
        &format!(
            "Welcome to Marreq!\n\nPlease verify your email address by visiting:\n{url}\n\nThis link expires in {VERIFY_TOKEN_TTL_HOURS} hours."
        ),
    )
}

fn send_password_reset_email(to: &str, token: &str) -> Result<(), email_sender::EmailError> {
    let url = format!(
        "{}/reset-password?token={}",
        public_base_url(),
        urlencoding::encode(token)
    );
    email_sender::send_email(
        to,
        "Reset your Marreq password",
        &format!(
            "A password reset was requested for your Marreq account. If this was you, visit:\n{url}\n\nThis link expires in {RESET_TOKEN_TTL_HOURS} hour(s). If you did not request this, you can ignore this message."
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use marreq_core::repository::diesel_repo_mock::DieselRepoMock;
    use marreq_core::repository::CacheRepository;
    use std::sync::{Arc, RwLock};

    fn state_with_repo(repo: DieselRepoMock) -> AppState<DieselCachedRepo> {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    fn registration_request(username: &str, email: &str) -> RegistrationRequest {
        RegistrationRequest {
            username: username.into(),
            name: "Alice Example".into(),
            email: email.into(),
            password: "CobaltRiver!Vacuum88".into(),
        }
    }

    #[test]
    fn register_creates_unverified_user_and_personal_workspace() {
        let state = state_with_repo(DieselRepoMock::default());
        let service = RegistrationService::new(&state);

        let user_id = service
            .register(registration_request("Alice", "Alice@Example.COM"))
            .unwrap();

        let repo = state.repo_read();
        let user = repo.get_user_by_id(user_id).unwrap();
        assert_eq!(user.username, "alice");
        assert_eq!(user.email, "alice@example.com");
        assert!(!user.email_verified);

        let workspace = repo
            .get_personal_workspace_for_user(user_id)
            .unwrap()
            .unwrap();
        assert_eq!(workspace.slug, "alice");
        assert_eq!(workspace.kind, "personal");
    }

    #[test]
    fn register_uses_numbered_workspace_slug_when_username_slug_is_taken() {
        let mut repo = DieselRepoMock::default();
        repo.insert_workspace(&NewWorkspace {
            slug: "alice".into(),
            name: "Existing Alice".into(),
            owner_user_id: 999,
            kind: "personal".into(),
        })
        .unwrap();
        let state = state_with_repo(repo);
        let service = RegistrationService::new(&state);

        let user_id = service
            .register(registration_request("Alice", "alice@example.com"))
            .unwrap();

        let workspace = state
            .repo_read()
            .get_personal_workspace_for_user(user_id)
            .unwrap()
            .unwrap();
        assert_eq!(workspace.slug, "alice-2");
    }

    #[test]
    fn verify_email_consumes_matching_token() {
        let mut repo = DieselRepoMock::default();
        let user_id = repo
            .insert_user(&NewUser {
                id: None,
                username: "alice".into(),
                name: "Alice Example".into(),
                email: "alice@example.com".into(),
                password_hash: "hash".into(),
                is_admin: false,
                email_verified: Some(false),
            })
            .unwrap();
        let token = "plain-token";
        repo.insert_email_token(&NewEmailToken {
            user_id,
            token_hash: sha256_hex(token),
            purpose: EmailToken::PURPOSE_VERIFY_EMAIL.into(),
            expires_at: (Utc::now() + Duration::hours(1)).naive_utc(),
        })
        .unwrap();
        let state = state_with_repo(repo);
        let service = RegistrationService::new(&state);

        assert_eq!(service.verify_email(token).unwrap(), user_id);

        let repo = state.repo_read();
        assert!(repo.get_user_by_id(user_id).unwrap().email_verified);
        assert!(repo
            .find_email_token_by_hash(&sha256_hex(token))
            .unwrap()
            .unwrap()
            .used_at
            .is_some());
        drop(repo);

        assert!(matches!(
            service.verify_email(token),
            Err(TokenError::NotFound)
        ));
    }

    #[test]
    fn reset_password_rejects_expired_token() {
        let mut repo = DieselRepoMock::default();
        let user_id = repo
            .insert_user(&NewUser {
                id: None,
                username: "alice".into(),
                name: "Alice Example".into(),
                email: "alice@example.com".into(),
                password_hash: "old-hash".into(),
                is_admin: false,
                email_verified: Some(true),
            })
            .unwrap();
        repo.insert_email_token(&NewEmailToken {
            user_id,
            token_hash: sha256_hex("expired-token"),
            purpose: EmailToken::PURPOSE_RESET_PASSWORD.into(),
            expires_at: (Utc::now() - Duration::hours(1)).naive_utc(),
        })
        .unwrap();
        let state = state_with_repo(repo);
        let service = RegistrationService::new(&state);

        assert!(matches!(
            service.reset_password("expired-token", "Another!Strong_2026"),
            Err(TokenError::Expired)
        ));
    }

    #[test]
    fn request_password_reset_for_unknown_email_is_not_an_error() {
        let state = state_with_repo(DieselRepoMock::default());
        let service = RegistrationService::new(&state);

        service
            .request_password_reset("missing@example.com")
            .unwrap();
    }
}

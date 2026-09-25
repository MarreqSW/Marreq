// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::errors::AuthError;
use crate::auth::csrf::{generate_csrf_token, set_csrf_cookie};
use crate::auth::set_session_cookie;
use crate::models::*;
use crate::repository::Repository;
use rocket::http::CookieJar;

// --------------------------------
// API
// --------------------------------

/// Process a login attempt. On success, session + CSRF cookies are set and the
/// authenticated user is returned. On failure an auth error is returned.
pub fn login_user<R: Repository>(
    repo: &mut R,
    login_form: &LoginForm,
    cookies: &CookieJar<'_>,
    user_agent: Option<String>,
    ip_addr: Option<String>,
) -> Result<User, AuthError> {
    let user = authenticate_user(&*repo, &login_form.username, &login_form.password)?;

    complete_login(repo, user, "password", cookies, user_agent, ip_addr)
}

/// Finish every human browser login in one place. Password and federated
/// adapters must call this after authenticating an identity, so session
/// rotation, CSRF rotation and audit semantics cannot drift between methods.
pub fn complete_login<R: Repository>(
    repo: &mut R,
    user: User,
    authentication_method: &str,
    cookies: &CookieJar<'_>,
    user_agent: Option<String>,
    ip_addr: Option<String>,
) -> Result<User, AuthError> {
    // Cloud mode: refuse login until the user has confirmed their email.
    if crate::deployment::current().requires_email_verification() && !user.email_verified {
        return Err(AuthError::EmailNotVerified);
    }

    repo.update_user_last_login(user.id, chrono::Utc::now().naive_utc())?;

    // Rotate any pre-existing authenticated session before issuing a new one.
    crate::auth::clear_session_cookie(cookies, repo);
    let audit_user_agent = user_agent.clone();
    let audit_ip_addr = ip_addr.clone();
    set_session_cookie(cookies, repo, user.id, user_agent, ip_addr)?;

    // Mint a fresh CSRF token on every login so that pre-login tokens are
    // invalidated (CSRF token rotation – mitigates session-fixation variants).
    set_csrf_cookie(cookies, generate_csrf_token());

    // Log the login - don't fail auth if logging fails
    let log = NewLog {
        user_id: user.id,
        action_type: "LOGIN".to_string(),
        entity_type: "User".to_string(),
        project_id: None,
        entity_id: Some(user.id),
        old_values: None,
        new_values: Some(
            serde_json::json!({ "authentication_method": authentication_method }).to_string(),
        ),
        description: Some(format!("User logged in using {authentication_method}")),
        ip_address: audit_ip_addr,
        user_agent: audit_user_agent,
    };
    let _ = repo.insert_log(&log);

    Ok(user)
}

/// Canonical account identifier used by credential lookup and rate limiting.
pub fn canonicalize_username(username: &str) -> String {
    username.trim().to_lowercase()
}

fn authenticate_user<R: Repository>(
    repo: &R,
    username: &str,
    password: &str,
) -> Result<User, AuthError> {
    // Normalise before lookup so "Alice" and "alice" resolve to the same account.
    let username_normalised = canonicalize_username(username);
    let user_opt = repo
        .get_user_by_username(&username_normalised)
        .map_err(|e| AuthError::Db(e.to_string()))?;

    let user = match user_opt {
        Some(u) => u,
        None => return Err(AuthError::InvalidCredentials),
    };

    let Some(password_hash) = user.password_hash.as_deref() else {
        return Err(AuthError::InvalidCredentials);
    };
    match super::verify_password(password, password_hash) {
        Ok(true) => Ok(user),
        Ok(false) => Err(AuthError::InvalidCredentials),
        Err(e) => Err(AuthError::Verify(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::hash_password;
    use crate::repository::diesel_repo_mock::DieselRepoMock;

    // ---------- authenticate_user tests ----------

    #[test]
    fn auth_ok_when_password_matches() {
        let pwd: String = hash_password("secret").unwrap();
        let repo = DieselRepoMock::with_users([DieselRepoMock::make_user(1, "alice", &pwd)]);
        let got = authenticate_user(&repo, "alice", "secret");
        assert!(got.is_ok());
        let user = got.unwrap();
        assert_eq!(user.username, "alice");
    }

    #[test]
    fn password_whitespace_is_preserved() {
        let pwd = hash_password("  secret  ").unwrap();
        let repo = DieselRepoMock::with_users([DieselRepoMock::make_user(1, "alice", &pwd)]);

        assert!(authenticate_user(&repo, "alice", "  secret  ").is_ok());
        assert!(matches!(
            authenticate_user(&repo, "alice", "secret"),
            Err(AuthError::InvalidCredentials)
        ));
    }

    #[test]
    fn username_canonicalization_matches_lookup_identity() {
        assert_eq!(canonicalize_username("  Alice  "), "alice");
    }

    #[test]
    fn auth_err_when_password_mismatch() {
        let pwd = hash_password("secret").unwrap();
        let repo = DieselRepoMock::with_users([DieselRepoMock::make_user(1, "alice", &pwd)]);
        let got = authenticate_user(&repo, "alice", "wrong");
        assert!(got.is_err());
        match got {
            Err(AuthError::InvalidCredentials) => (),
            _ => panic!("Expected InvalidCredentials error"),
        }
    }

    #[test]
    fn auth_err_when_user_not_found() {
        let repo = DieselRepoMock::with_users([]);
        let got = authenticate_user(&repo, "ghost", "anything");
        assert!(got.is_err());
        match got {
            Err(AuthError::InvalidCredentials) => (),
            _ => panic!("Expected InvalidCredentials error"),
        }
    }

    #[test]
    fn external_only_user_cannot_use_password_login() {
        let mut user = DieselRepoMock::make_user(1, "external", "unused");
        user.password_hash = None;
        let repo = DieselRepoMock::with_users([user]);
        assert!(matches!(
            authenticate_user(&repo, "external", "anything"),
            Err(AuthError::InvalidCredentials)
        ));
    }

    #[test]
    fn returns_err_on_repo_error() {
        let repo = DieselRepoMock::with_error();
        let err = authenticate_user(&repo, "alice", "secret");
        assert!(err.is_err());
        match err {
            Err(AuthError::Db(_)) => (),
            _ => panic!("Expected Db error"),
        }
    }

    #[test]
    fn returns_err_when_verifier_fails() {
        // stored "ERR" triggers verifier error in our stub
        let repo = DieselRepoMock::with_users([DieselRepoMock::make_user(1, "alice", "ERR")]);
        let err = authenticate_user(&repo, "alice", "doesnt_matter");
        assert!(err.is_err());
        match err {
            Err(AuthError::Verify(_)) => (),
            _ => panic!("Expected Verify error"),
        }
    }
}

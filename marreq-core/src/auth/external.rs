// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Shared, provider-neutral building blocks for browser federated login.
//!
//! Provider adapters turn a validated provider response into
//! [`ExternalIdentity`].  This module intentionally contains no provider JSON
//! parsing, token persistence, or email-based account matching.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const ENTROPY_BYTES: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalIdentity {
    pub provider_key: String,
    pub issuer: String,
    pub subject: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub username: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicAuthProvider {
    pub id: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderDiscovery {
    pub password_enabled: bool,
    pub external: Vec<PublicAuthProvider>,
}

/// Sensitive, short-lived browser-bound transaction state. It belongs only in
/// server-side storage or a Rocket private cookie, never in a URL or log.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthTransaction {
    pub provider_key: String,
    pub state: String,
    pub nonce: String,
    pub pkce_verifier: String,
    pub return_to: String,
    pub expires_at: chrono::NaiveDateTime,
    pub operation: OAuthOperation,
    pub link_user_id: Option<i32>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OAuthOperation {
    Login,
    Link,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransactionError {
    InvalidState,
    Expired,
    WrongProvider,
    InvalidReturnTo,
}

impl OAuthTransaction {
    pub fn new(
        provider_key: impl Into<String>,
        return_to: &str,
        operation: OAuthOperation,
        now: chrono::NaiveDateTime,
    ) -> Result<Self, TransactionError> {
        if !is_safe_return_to(return_to) {
            return Err(TransactionError::InvalidReturnTo);
        }
        Ok(Self {
            provider_key: provider_key.into(),
            state: random_urlsafe(),
            nonce: random_urlsafe(),
            pkce_verifier: random_urlsafe(),
            return_to: return_to.to_owned(),
            operation,
            link_user_id: None,
            expires_at: now + chrono::Duration::minutes(10),
        })
    }

    pub fn for_link(
        provider_key: impl Into<String>,
        return_to: &str,
        user_id: i32,
        now: chrono::NaiveDateTime,
    ) -> Result<Self, TransactionError> {
        let mut transaction = Self::new(provider_key, return_to, OAuthOperation::Link, now)?;
        transaction.link_user_id = Some(user_id);
        Ok(transaction)
    }

    /// Validate before token exchange. The caller must delete the transaction
    /// regardless of this result, making a callback state single-use.
    pub fn validate_callback(
        &self,
        provider: &str,
        state: Option<&str>,
        now: chrono::NaiveDateTime,
    ) -> Result<(), TransactionError> {
        if now > self.expires_at {
            return Err(TransactionError::Expired);
        }
        if self.provider_key != provider {
            return Err(TransactionError::WrongProvider);
        }
        if !state.is_some_and(|candidate| secret_matches(candidate, &self.state)) {
            return Err(TransactionError::InvalidState);
        }
        Ok(())
    }
}

pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn secret_matches(left: &str, right: &str) -> bool {
    Sha256::digest(left.as_bytes()) == Sha256::digest(right.as_bytes())
}

fn random_urlsafe() -> String {
    let mut bytes = [0u8; ENTROPY_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Only local absolute paths are valid post-login destinations.
pub fn is_safe_return_to(value: &str) -> bool {
    value.starts_with('/')
        && !value.starts_with("//")
        && !value.contains('\\')
        && !value.contains('\r')
        && !value.contains('\n')
}

/// A user may remove a linked identity only when a different usable login
/// method remains. Callers supply the already-authorized user's credential
/// state; this function intentionally does not infer ownership from email.
pub fn may_unlink_identity(has_password: bool, identity_count: usize) -> bool {
    has_password || identity_count > 1
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transaction_rejects_bad_state_expiry_and_provider() {
        let now = chrono::Utc::now().naive_utc();
        let tx = OAuthTransaction::new("oidc", "/projects/2", OAuthOperation::Login, now).unwrap();
        assert!(tx.validate_callback("oidc", Some(&tx.state), now).is_ok());
        assert_eq!(
            tx.validate_callback("oidc", Some("wrong"), now),
            Err(TransactionError::InvalidState)
        );
        assert_eq!(
            tx.validate_callback("github", Some(&tx.state), now),
            Err(TransactionError::WrongProvider)
        );
        assert_eq!(
            tx.validate_callback(
                "oidc",
                Some(&tx.state),
                tx.expires_at + chrono::Duration::seconds(1)
            ),
            Err(TransactionError::Expired)
        );
    }
    #[test]
    fn return_to_does_not_allow_open_redirects() {
        assert!(is_safe_return_to("/projects/1"));
        for bad in [
            "https://evil.example",
            "//evil.example",
            "javascript:alert(1)",
            "projects/1",
        ] {
            assert!(!is_safe_return_to(bad));
        }
    }
    #[test]
    fn cannot_remove_last_login_method() {
        assert!(!may_unlink_identity(false, 1));
        assert!(may_unlink_identity(true, 1));
        assert!(may_unlink_identity(false, 2));
    }

    #[test]
    fn pkce_uses_rfc7636_s256_encoding() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            pkce_challenge(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }
}

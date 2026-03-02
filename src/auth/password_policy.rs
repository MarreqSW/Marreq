// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use lazy_static::lazy_static;
use std::collections::HashSet;
use thiserror::Error;

/// Minimum allowed password length for user-selected passwords (ASVS 6.2.1).
pub const MIN_PASSWORD_LENGTH: usize = 8;
/// Recommended minimum password length (NIST/ASVS guidance).
pub const RECOMMENDED_PASSWORD_LENGTH: usize = 15;
/// Upper bound to prevent pathological input sizes while still permitting long passphrases.
pub const MAX_PASSWORD_LENGTH: usize = 1024;

/// Application-specific context words blocked in user passwords (ASVS 6.2.11).
///
/// This documented list is intentionally small and product-specific. User-specific
/// terms (username/email/name tokens) are added dynamically at validation time.
pub const DOCUMENTED_CONTEXT_WORDS: &[&str] = &[
    "marreq",
    "requirement",
    "requirements",
    "traceability",
    "baseline",
    "verification",
];

const TOP_3000_PASSWORDS: &str = include_str!("data/top3000-policy-passwords.txt");
const BREACHED_PASSWORDS: &str = include_str!("data/breached-passwords.txt");

lazy_static! {
    static ref TOP_3000_SET: HashSet<String> = parse_password_set(TOP_3000_PASSWORDS);
    static ref BREACHED_SET: HashSet<String> = parse_password_set(BREACHED_PASSWORDS);
}

#[derive(Debug, Clone, Default)]
pub struct PasswordContext<'a> {
    pub username: Option<&'a str>,
    pub email: Option<&'a str>,
    pub full_name: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PasswordPolicyError {
    #[error("Password must be at least {min} characters long")]
    TooShort { min: usize },

    #[error("Password must be at most {max} characters long")]
    TooLong { max: usize },

    #[error("Password is too common. Choose a more unique password")]
    Top3000Password,

    #[error("Password appears in known breached-password datasets")]
    BreachedPassword,

    #[error("Password must not contain context-specific terms")]
    ContextSpecific,
}

/// Validate a password against ASVS/NIST-aligned rules.
///
/// This function intentionally does not trim or transform the original password
/// before hashing or verification. Normalization is only used for dictionary
/// comparisons.
pub fn validate_password(
    password: &str,
    context: PasswordContext<'_>,
) -> Result<(), PasswordPolicyError> {
    let char_len = password.chars().count();

    if char_len < MIN_PASSWORD_LENGTH {
        return Err(PasswordPolicyError::TooShort {
            min: MIN_PASSWORD_LENGTH,
        });
    }

    if char_len > MAX_PASSWORD_LENGTH {
        return Err(PasswordPolicyError::TooLong {
            max: MAX_PASSWORD_LENGTH,
        });
    }

    let normalized_password = normalize(password);

    if TOP_3000_SET.contains(&normalized_password) {
        return Err(PasswordPolicyError::Top3000Password);
    }

    if BREACHED_SET.contains(&normalized_password) {
        return Err(PasswordPolicyError::BreachedPassword);
    }

    let context_terms = collect_context_terms(context);
    if context_terms
        .iter()
        .any(|term| normalized_password.contains(term))
    {
        return Err(PasswordPolicyError::ContextSpecific);
    }

    Ok(())
}

fn parse_password_set(raw: &str) -> HashSet<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(normalize)
        .collect()
}

fn normalize(value: &str) -> String {
    value.to_lowercase()
}

fn collect_context_terms(context: PasswordContext<'_>) -> HashSet<String> {
    let mut terms: HashSet<String> = DOCUMENTED_CONTEXT_WORDS
        .iter()
        .map(|w| normalize(w))
        .collect();

    if let Some(username) = context.username {
        terms.extend(extract_terms(username));
    }

    if let Some(email) = context.email {
        terms.extend(extract_terms(email));
        if let Some((local, domain)) = email.split_once('@') {
            terms.extend(extract_terms(local));
            terms.extend(extract_terms(domain));
        }
    }

    if let Some(full_name) = context.full_name {
        terms.extend(extract_terms(full_name));
    }

    terms.retain(|term| term.len() >= 3);
    terms
}

fn extract_terms(source: &str) -> impl Iterator<Item = String> + '_ {
    source
        .split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(normalize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_3000_password_list_has_expected_minimum_size() {
        assert!(TOP_3000_SET.len() >= 3000);
    }

    #[test]
    fn breached_password_list_is_non_trivial() {
        assert!(BREACHED_SET.len() >= 10000);
    }

    #[test]
    fn rejects_password_shorter_than_minimum() {
        let err = validate_password("short", PasswordContext::default()).unwrap_err();
        assert!(matches!(err, PasswordPolicyError::TooShort { min: 8 }));
    }

    #[test]
    fn allows_long_passwords_of_at_least_sixty_four_characters() {
        let candidate = "A".repeat(64);
        assert!(validate_password(&candidate, PasswordContext::default()).is_ok());
    }

    #[test]
    fn rejects_top_3000_password() {
        let err = validate_password("password1", PasswordContext::default()).unwrap_err();
        assert_eq!(err, PasswordPolicyError::Top3000Password);
    }

    #[test]
    fn rejects_breached_password() {
        let err = validate_password("!qaz1qaz", PasswordContext::default()).unwrap_err();
        assert_eq!(err, PasswordPolicyError::BreachedPassword);
    }

    #[test]
    fn rejects_password_with_contextual_username_or_email_term() {
        let ctx = PasswordContext {
            username: Some("alice"),
            email: Some("alice@example.com"),
            full_name: Some("Alice Doe"),
        };

        let err = validate_password("MyAlicePassphrase2026", ctx).unwrap_err();
        assert_eq!(err, PasswordPolicyError::ContextSpecific);
    }

    #[test]
    fn allows_password_without_contextual_terms_or_dictionary_hits() {
        let ctx = PasswordContext {
            username: Some("alice"),
            email: Some("alice@example.com"),
            full_name: Some("Alice Doe"),
        };

        assert!(validate_password("CobaltRiver!Vacuum88", ctx).is_ok());
    }
}

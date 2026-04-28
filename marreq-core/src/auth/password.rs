// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::errors::AuthError;
use super::password_policy::{validate_password, PasswordContext};
use crate::repository::Repository;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::{password_hash::SaltString, Argon2};
use rocket::http::CookieJar;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PasswordCryptoError {
    #[error("argon2 error: {0}")]
    Argon2(String),

    #[error("unsupported password hash format")]
    UnsupportedHashFormat,
}

pub fn hash_password(password: &str) -> Result<String, PasswordCryptoError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| PasswordCryptoError::Argon2(e.to_string()))
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, PasswordCryptoError> {
    let hash = hash.trim();
    if hash.starts_with("$argon2") {
        let parsed =
            PasswordHash::new(hash).map_err(|e| PasswordCryptoError::Argon2(e.to_string()))?;

        match Argon2::default().verify_password(password.as_bytes(), &parsed) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(PasswordCryptoError::Argon2(e.to_string())),
        }
    } else {
        Err(PasswordCryptoError::UnsupportedHashFormat)
    }
}

fn change_user_password_impl<R: Repository>(
    repo: &mut R,
    uid: i32,
    current_password: &str,
    new_password: &str,
) -> Result<(), AuthError> {
    let user = repo.get_user_by_id(uid)?;

    match verify_password(current_password, &user.password_hash) {
        Ok(true) => {
            validate_password(
                new_password,
                PasswordContext {
                    username: Some(&user.username),
                    email: Some(&user.email),
                    full_name: Some(&user.name),
                },
            )
            .map_err(|e| AuthError::PasswordPolicy(e.to_string()))?;

            let new_hash =
                hash_password(new_password).map_err(|e| AuthError::Verify(e.to_string()))?;

            repo.update_user_password(uid, &new_hash)?;
            Ok(())
        }
        Ok(false) => Err(AuthError::InvalidCredentials),
        Err(e) => Err(AuthError::Verify(e.to_string())),
    }
}

/// Handle a change password request, returning success or a domain error.
pub fn change_user_password<R: Repository>(
    repo: &mut R,
    current_password: &str,
    new_password: &str,
    cookies: &CookieJar<'_>,
) -> Result<(), AuthError> {
    // Get user ID from cookie
    let user_id_cookie = cookies
        .get_private(super::session::SESSION_COOKIE)
        .ok_or(AuthError::NotLoggedIn)?;
    let user_id = user_id_cookie
        .value()
        .parse::<i32>()
        .map_err(|_| AuthError::InvalidSession)?;

    change_user_password_impl(repo, user_id, current_password, new_password)
}

/// Set a user's password as admin (no current password required).
/// Caller must ensure the requester is an admin; this function only performs the update.
pub fn admin_set_user_password<R: Repository>(
    repo: &mut R,
    target_user_id: i32,
    new_password: &str,
) -> Result<(), AuthError> {
    let user = repo.get_user_by_id(target_user_id)?;

    validate_password(
        new_password,
        PasswordContext {
            username: Some(&user.username),
            email: Some(&user.email),
            full_name: Some(&user.name),
        },
    )
    .map_err(|e| AuthError::PasswordPolicy(e.to_string()))?;

    let new_hash = hash_password(new_password).map_err(|e| AuthError::Verify(e.to_string()))?;
    repo.update_user_password(target_user_id, &new_hash)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::repository::UserRepository;

    #[test]
    fn argon2_roundtrip_ok() {
        let pw = "s3cr3t!";
        let hash = hash_password(pw).expect("hash should succeed");
        assert!(hash.starts_with("$argon2"));
        assert!(verify_password(pw, &hash).expect("verify should succeed"));
    }

    #[test]
    fn argon2_mismatch_is_false() {
        let hash = hash_password("correct-horse").expect("hash");
        assert!(!verify_password("battery-staple", &hash).expect("verify"));
    }

    #[test]
    fn long_passwords_roundtrip_without_truncation() {
        let long = "X".repeat(128);
        let hash = hash_password(&long).unwrap();
        assert!(verify_password(&long, &hash).unwrap());
    }

    #[test]
    fn change_user_password_updates_hash_and_verifies() {
        // Arrange: user with current hash
        let current = "oldpw-123";
        let newpw = "newpw-123";
        let current_hash = hash_password(current).unwrap();
        let user = DieselRepoMock::make_user(1, "alice", &current_hash);
        let mut repo = DieselRepoMock::with_users([user]);

        // Act
        change_user_password_impl(&mut repo, 1, current, newpw).expect("should succeed");

        // Assert: stored hash changed AND matches the new password
        let updated = repo.get_user_by_id(1).unwrap();
        assert_ne!(updated.password_hash, current_hash);
        assert!(verify_password(newpw, &updated.password_hash).unwrap());
        assert!(!verify_password(current, &updated.password_hash).unwrap());
    }

    #[test]
    fn change_user_password_rejects_wrong_current_password() {
        let current_hash = hash_password("right-now").unwrap();
        let user = DieselRepoMock::make_user(7, "bob", &current_hash);
        let mut repo = DieselRepoMock::with_users([user]);

        let err = change_user_password_impl(&mut repo, 7, "nope", "new-password-1").unwrap_err();
        assert!(matches!(err, AuthError::InvalidCredentials));

        // Ensure nothing was changed
        let stored = repo.get_user_by_id(7).unwrap().password_hash;
        assert_eq!(stored, current_hash);
    }

    #[test]
    fn change_user_password_fails_when_user_not_found() {
        let mut repo = DieselRepoMock::with_users([]);
        let result = change_user_password_impl(&mut repo, 99, "x", "new-password-1");
        assert!(result.is_err());
        assert!(!matches!(
            result.unwrap_err(),
            AuthError::InvalidCredentials | AuthError::Verify(_)
        ));
    }

    #[test]
    fn change_user_password_propagates_update_failure() {
        let current_hash = hash_password("pw-123456").unwrap();
        let user = DieselRepoMock::make_user(3, "carol", &current_hash);
        let mut repo = DieselRepoMock::with_users([user]);
        repo.force_err = true; // make update_user_password error

        let result = change_user_password_impl(&mut repo, 3, "pw-123456", "new-password-123");
        assert!(result.is_err());
    }

    #[test]
    fn change_user_password_rejects_context_specific_password() {
        let current_hash = hash_password("pw-123456").unwrap();
        let user = DieselRepoMock::make_user(5, "dave", &current_hash);
        let mut repo = DieselRepoMock::with_users([user]);

        let err =
            change_user_password_impl(&mut repo, 5, "pw-123456", "dave-password-2026").unwrap_err();

        assert!(matches!(err, AuthError::PasswordPolicy(_)));
    }

    // --- admin_set_user_password ---

    #[test]
    fn admin_set_user_password_updates_hash_and_verifies() {
        let old_hash = hash_password("old-secret").unwrap();
        let user = DieselRepoMock::make_user(1, "alice", &old_hash);
        let mut repo = DieselRepoMock::with_users([user]);

        admin_set_user_password(&mut repo, 1, "new-password-123").expect("should succeed");

        let updated = repo.get_user_by_id(1).unwrap();
        assert_ne!(updated.password_hash, old_hash);
        assert!(verify_password("new-password-123", &updated.password_hash).unwrap());
        assert!(!verify_password("old-secret", &updated.password_hash).unwrap());
    }

    #[test]
    fn admin_set_user_password_fails_when_user_not_found() {
        let mut repo = DieselRepoMock::with_users([]);
        let result = admin_set_user_password(&mut repo, 99, "new-password-123");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::Repo(_)));
    }

    #[test]
    fn admin_set_user_password_rejects_policy_violation() {
        let user = DieselRepoMock::make_user(2, "bob", "hash");
        let mut repo = DieselRepoMock::with_users([user]);

        let err = admin_set_user_password(&mut repo, 2, "short").unwrap_err();
        assert!(matches!(err, AuthError::PasswordPolicy(_)));
    }

    #[test]
    fn admin_set_user_password_rejects_context_specific_password() {
        let user = DieselRepoMock::make_user(3, "carol", "hash");
        let mut repo = DieselRepoMock::with_users([user]);

        let err = admin_set_user_password(&mut repo, 3, "carol-password-2026").unwrap_err();
        assert!(matches!(err, AuthError::PasswordPolicy(_)));
    }

    #[test]
    fn admin_set_user_password_propagates_update_failure() {
        let user = DieselRepoMock::make_user(4, "dave", "hash");
        let mut repo = DieselRepoMock::with_users([user]);
        repo.force_err = true;

        let result = admin_set_user_password(&mut repo, 4, "new-password-123");
        assert!(result.is_err());
    }

    /// Hash used in migrations and scripts/init_complete.sql for seeded users (password: ChangeMe123!).
    #[test]
    fn seeded_demo_password_hash_verifies() {
        const SEEDED_HASH: &str =
            "$argon2id$v=19$m=19456,t=2,p=1$3o6cC/67ksnBxHCCF9rGHA$oWCATKyiKRCdDgWucvrMHinlWvzZNhqoUUvnpyCgOW0";
        assert!(
            verify_password("ChangeMe123!", SEEDED_HASH).unwrap(),
            "Seeded demo password must verify; update migrations/ and scripts/ with a hash from hash_password(\"ChangeMe123!\")"
        );
    }
}

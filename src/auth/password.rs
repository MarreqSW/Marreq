// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

use super::errors::AuthError;
use super::password_policy::{validate_password, PasswordContext};
use crate::repository::Repository;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::{password_hash::SaltString, Argon2};
use rand_core::OsRng;
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

    // Helper for generating static Argon2 hashes when needed.
    // #[test]
    // fn print_argon2_hash() {
    //     println!("{}", hash_password("ChangeMe123!").unwrap());
    // }
}

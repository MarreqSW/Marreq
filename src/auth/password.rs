use super::errors::AuthError;
use crate::repository::Repository;
use bcrypt::{hash, verify, DEFAULT_COST};
use rocket::http::CookieJar;

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
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
    let id = user_id_cookie
        .value()
        .parse::<i32>()
        .map_err(|_| AuthError::InvalidSession)?;

    change_user_password_impl(repo, id, current_password, new_password)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock; // adjust path to your DieselRepoMock
    use crate::repository::UserRepository;

    // --- hash/verify ---------------------------------------------------------

    #[test]
    fn bcrypt_roundtrip_ok() {
        let pw = "s3cr3t!";
        let hash = hash_password(pw).expect("hash should succeed");
        assert!(verify_password(pw, &hash).expect("verify should succeed"));
    }

    #[test]
    fn bcrypt_mismatch_is_false() {
        let hash = hash_password("correct-horse").expect("hash");
        assert!(!verify_password("battery-staple", &hash).expect("verify"));
    }

    // --- change_user_password_impl ------------------------------------------------

    #[test]
    fn change_user_password_updates_hash_and_verifies() {
        // Arrange: user with current bcrypt hash
        let current = "oldpw";
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

        let err = change_user_password_impl(&mut repo, 7, "nope", "new").unwrap_err();
        // Exact variant depends on your AuthError; the function returns InvalidCredentials here.
        assert!(matches!(err, AuthError::InvalidCredentials));

        // Ensure nothing was changed
        let stored = repo.get_user_by_id(7).unwrap().password_hash;
        assert_eq!(stored, current_hash);
    }

    #[test]
    fn change_user_password_fails_when_user_not_found() {
        let mut repo = DieselRepoMock::with_users([]); // empty
        let result = change_user_password_impl(&mut repo, 99, "x", "y");
        assert!(result.is_err());
        // usually mapped from RepoError::NotFound -> AuthError (Db or similar)
        assert!(!matches!(
            result.unwrap_err(),
            AuthError::InvalidCredentials | AuthError::Verify(_)
        ));
    }

    #[test]
    fn change_user_password_propagates_update_failure() {
        let current_hash = hash_password("pw").unwrap();
        let user = DieselRepoMock::make_user(3, "carol", &current_hash);
        let mut repo = DieselRepoMock::with_users([user]);
        repo.force_err = true; // make update_user_password error

        let result = change_user_password_impl(&mut repo, 3, "pw", "newpw");
        assert!(result.is_err());
        // Not checking exact variant because it depends on your From<RepoError> for AuthError
    }
}

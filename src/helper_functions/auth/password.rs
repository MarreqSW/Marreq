use bcrypt::{hash, verify, DEFAULT_COST};
//use crate::models::*;
use super::errors::AuthError;
use crate::{repository::Repository};
use rocket::http::CookieJar;
use rocket_dyn_templates::Template;
use rocket::serde::json::json;

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
}

pub fn change_user_password<R: Repository>(
    repo: &mut R,
    uid: i32,
    current_password: &str,
    new_password: &str
) -> Result<(), AuthError> {

    let user = repo.get_user_by_id(uid)?;

    match verify_password(current_password, &user.user_password) {
        Ok(true) => {
            let new_hash = hash_password(new_password)
                .map_err(|e| AuthError::Verify(e.to_string()))?;

            repo.update_user_password(uid, &new_hash)?;
            Ok(())
        }
        Ok(false) => Err(AuthError::InvalidCredentials),
        Err(e) => Err(AuthError::Verify(e.to_string())),
    }
}

/// Handle a change password request, returning either a success or error template.
pub fn change_password_user(
    //password_form: &ChangePasswordForm,
    current_password: &String,
    new_password: &String,
    cookies: &CookieJar<'_>,
) -> Result<Template, Template> {
    // Get user ID from cookie
    let user_id_cookie = cookies.get_private("user_id");
    let user_id = match user_id_cookie {
        Some(cookie) => match cookie.value().parse::<i32>() {
            Ok(id) => id,
            Err(_) => {
                let ctx = json!({
                    "title": "Change Password",
                    "error": "Invalid session",
                });
                return Err(Template::render("change_password", ctx));
            }
        },
        None => {
            let ctx = json!({
                "title": "Change Password",
                "error": "Not logged in",
            });
            return Err(Template::render("change_password", ctx));
        }
    };

    use crate::repository::diesel_repo::DieselRepo ;
    let mut repo = DieselRepo{};

    match change_user_password(
        &mut repo,
        user_id,
        &current_password,
        &new_password,
    ) {
        Ok(_) => {
            let ctx = json!({
                "title": "Change Password",
                "success": "Password changed successfully",
            });
            Ok(Template::render("change_password", ctx))
        }
        Err(e) => {
            let ctx = json!({
                "title": "Change Password",
                "error": e.to_string(),
            });
            Err(Template::render("change_password", ctx))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::fake_repo::FakeRepo; // adjust path to your FakeRepo
    use crate::repository::Repository;

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

    // --- change_user_password ------------------------------------------------

    #[test]
    fn change_user_password_updates_hash_and_verifies() {
        // Arrange: user with current bcrypt hash
        let current = "oldpw";
        let newpw = "newpw-123";
        let current_hash = hash_password(current).unwrap();
        let user = FakeRepo::make_user(1, "alice", &current_hash);
        let mut repo = FakeRepo::with_users([user]);

        // Act
        change_user_password(&mut repo, 1, current, newpw).expect("should succeed");

        // Assert: stored hash changed AND matches the new password
        let updated = repo.get_user_by_id(1).unwrap();
        assert_ne!(updated.user_password, current_hash);
        assert!(verify_password(newpw, &updated.user_password).unwrap());
        assert!(!verify_password(current, &updated.user_password).unwrap());
    }

    #[test]
    fn change_user_password_rejects_wrong_current_password() {
        let current_hash = hash_password("right-now").unwrap();
        let user = FakeRepo::make_user(7, "bob", &current_hash);
        let mut repo = FakeRepo::with_users([user]);

        let err = change_user_password(&mut repo, 7, "nope", "new").unwrap_err();
        // Exact variant depends on your AuthError; the function returns InvalidCredentials here.
        assert!(matches!(err, AuthError::InvalidCredentials));

        // Ensure nothing was changed
        let stored = repo.get_user_by_id(7).unwrap().user_password;
        assert_eq!(stored, current_hash);
    }

    #[test]
    fn change_user_password_fails_when_user_not_found() {
        let mut repo = FakeRepo::with_users([]); // empty
        let result = change_user_password(&mut repo, 99, "x", "y");
        assert!(result.is_err());
        // usually mapped from RepoError::NotFound -> AuthError (Db or similar)
        assert!(!matches!(result.unwrap_err(), AuthError::InvalidCredentials | AuthError::Verify(_)));
    }

    #[test]
    fn change_user_password_propagates_update_failure() {
        let current_hash = hash_password("pw").unwrap();
        let user = FakeRepo::make_user(3, "carol", &current_hash);
        let mut repo = FakeRepo::with_users([user]);
        repo.force_err = true; // make update_user_password error

        let result = change_user_password(&mut repo, 3, "pw", "newpw");
        assert!(result.is_err());
        // Not checking exact variant because it depends on your From<RepoError> for AuthError
    }
}

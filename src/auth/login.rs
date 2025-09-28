use super::errors::AuthError;
use crate::auth::set_session_cookie;
use crate::logger::{LogCtx, Logger};
use crate::models::*;
use crate::repository::DieselRepo;
use crate::repository::Repository;
use rocket::http::CookieJar;

// --------------------------------
// API
// --------------------------------

/// Process a login attempt. On success, session cookies are set and an empty
/// Ok is returned. On failure a rendered `Template` with the corresponding
/// error is returned.
pub fn login_user<R: Repository>(
    repo: &R,
    login_form: &LoginForm,
    cookies: &CookieJar<'_>,
) -> Result<(), AuthError> {
    let user = authenticate_user(repo, &login_form.username, &login_form.password)?;

    // Store session information
    set_session_cookie(cookies, user.user_id);

    let mut conn = DieselRepo::new()
        .get_conn()
        .map_err(|e| AuthError::Db(e.to_string()))?;
    let ctx = LogCtx::new(user.user_id);
    Logger::log_login(&mut conn, &ctx).map_err(|e| AuthError::Audit(e.to_string()))?;

    Ok(())
}

fn authenticate_user<R: Repository>(
    repo: &R,
    username: &str,
    password: &str,
) -> Result<User, AuthError> {
    let user_opt = repo
        .get_user_by_username(username)
        .map_err(|e| AuthError::Db(e.to_string()))?;

    let user = match user_opt {
        Some(u) => u,
        None => return Err(AuthError::InvalidCredentials),
    };

    match super::verify_password(password, &user.user_password) {
        Ok(true) => Ok(user),
        Ok(false) => Err(AuthError::InvalidCredentials),
        Err(e) => Err(AuthError::Verify(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::hash_password;
    use crate::repository::fake_repo::FakeRepo;

    // ---------- authenticate_user tests ----------

    #[test]
    fn auth_ok_when_password_matches() {
        let pwd: String = hash_password("secret").unwrap();
        let repo = FakeRepo::with_users([FakeRepo::make_user(1, "alice", &pwd)]);
        let got = authenticate_user(&repo, "alice", "secret");
        assert!(got.is_ok());
        let user = got.unwrap();
        assert_eq!(user.user_username, "alice");
    }

    #[test]
    fn auth_err_when_password_mismatch() {
        let pwd = hash_password("secret").unwrap();
        let repo = FakeRepo::with_users([FakeRepo::make_user(1, "alice", &pwd)]);
        let got = authenticate_user(&repo, "alice", "wrong");
        assert!(got.is_err());
        match got {
            Err(AuthError::InvalidCredentials) => (),
            _ => panic!("Expected InvalidCredentials error"),
        }
    }

    #[test]
    fn auth_err_when_user_not_found() {
        let repo = FakeRepo::with_users([]);
        let got = authenticate_user(&repo, "ghost", "anything");
        assert!(got.is_err());
        match got {
            Err(AuthError::InvalidCredentials) => (),
            _ => panic!("Expected InvalidCredentials error"),
        }
    }

    #[test]
    fn returns_err_on_repo_error() {
        let repo = FakeRepo::with_error();
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
        let repo = FakeRepo::with_users([FakeRepo::make_user(1, "alice", "ERR")]);
        let err = authenticate_user(&repo, "alice", "doesnt_matter");
        assert!(err.is_err());
        match err {
            Err(AuthError::Verify(_)) => (),
            _ => panic!("Expected Verify error"),
        }
    }
}

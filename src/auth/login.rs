use rocket::http::{Cookie, CookieJar};
use super::errors::AuthError;
use crate::models::*;
use crate::repository::DieselRepo;
use crate::logger::Logger;
use crate::repository::Repository;

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

    // Set session cookies
    cookies.add_private(Cookie::new("user_id", user.user_id.to_string()));
    cookies.add_private(Cookie::new("username", user.user_username.clone()));
    cookies.add_private(Cookie::new("user_name", user.user_name.clone()));

    let mut conn = DieselRepo::new()
        .get_conn()
        .map_err(|e| AuthError::Db(e.to_string()))?;
    Logger::log_login(&mut conn, user.user_id, None).map_err(|e| AuthError::Audit(e.to_string()))?;

    Ok(())
}

pub fn is_authenticated<R: Repository>(
    repo: &R,
    cookies: &CookieJar<'_>,
) -> Option<User> {
    let uid_cookie   = cookies.get_private("user_id");
    let uname_cookie = cookies.get_private("username");

    is_authenticated_impl(
        repo,
        uid_cookie.as_ref().map(|c| c.value()),
        uname_cookie.as_ref().map(|c| c.value()),
    )
}


// --------------------------------
// Implementation
// --------------------------------

fn is_authenticated_impl<R: Repository>(
    repo: &R,
    user_id: Option<&str>,
    username: Option<&str>,
) -> Option<User> {
    let (uid, uname) = (user_id?, username?);
    let uid = uid.parse::<i32>().ok()?;
    let user = repo.get_user_by_id(uid).ok()?;
    (user.user_username == uname).then_some(user)
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
        Ok(true)  => Ok(user),
        Ok(false) => Err(AuthError::InvalidCredentials),
        Err(e)    => Err(AuthError::Verify(e.to_string())),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::fake_repo::FakeRepo;
    use std::collections::HashMap;
    use crate::auth::hash_password;

    // ---------- is_authenticated_impl tests ----------

    #[test]
    fn returns_user_on_match_core() {
        let mut map = HashMap::new();
        map.insert(42, FakeRepo::make_user(42, "alice", "pwd"));
        let repo = FakeRepo { users: map, ..Default::default() };

        let got = is_authenticated_impl(&repo, Some("42"), Some("alice"));
        assert!(got.is_some());
        assert_eq!(got.unwrap().user_username, "alice");
    }

    #[test]
    fn returns_none_on_username_mismatch_core() {
        let mut map = HashMap::new();
        map.insert(42, FakeRepo::make_user(42, "alice", "pwd"));
        let repo = FakeRepo { users: map, ..Default::default() };

        let got = is_authenticated_impl(&repo, Some("42"), Some("bob"));
        assert!(got.is_none());
    }

    #[test]
    fn returns_none_when_user_missing_core() {
        let repo = FakeRepo::default();
        let got = is_authenticated_impl(&repo, Some("42"), Some("alice"));
        assert!(got.is_none());
    }

    #[test]
    fn returns_none_on_bad_user_id_parse_core() {
        let mut map = HashMap::new();
        map.insert(42, FakeRepo::make_user(42, "alice", "pwd"));
        let repo = FakeRepo { users: map, ..Default::default() };

        let got = is_authenticated_impl(&repo, Some("not-an-int"), Some("alice"));
        assert!(got.is_none());
    }

    #[test]
    fn returns_none_when_any_cookie_missing_core() {
        let mut map = HashMap::new();
        map.insert(42, FakeRepo::make_user(42, "alice", "pwd"));
        let repo = FakeRepo { users: map, ..Default::default() };

        assert!(is_authenticated_impl(&repo, None, Some("alice")).is_none());
        assert!(is_authenticated_impl(&repo, Some("42"), None).is_none());
        assert!(is_authenticated_impl(&repo, None, None).is_none());
    }

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

use rocket::http::{Cookie, CookieJar};
use rocket_dyn_templates::Template;
use rocket::serde::json::json;
use crate::models::*;
use crate::db::get_connection_pooled_safe;
use crate::logger::Logger;
use crate::repository::Repository;

// --------------------------------
// Authentication Route Logic
// --------------------------------

/// Process a login attempt. On success, session cookies are set and an empty
/// Ok is returned. On failure a rendered `Template` with the corresponding
/// error is returned.
pub fn login_user(login_form: &LoginForm, cookies: &CookieJar<'_>) -> Result<(), Template> {
    use crate::repository::diesel_repo::DieselRepo ;
    let repo = DieselRepo{};

    match authenticate_user(&repo, &login_form.username, &login_form.password) {
        Ok(Some(user)) => {
            // Set session cookies
            cookies.add_private(Cookie::new("user_id", user.user_id.to_string()));
            cookies.add_private(Cookie::new("username", user.user_username.clone()));
            cookies.add_private(Cookie::new("user_name", user.user_name.clone()));

            // Log successful login
            let mut conn = get_connection_pooled_safe().map_err(|e| {
                eprintln!("Database connection error: {}", e);
                Template::render("error", json!({"error": "Database connection failed"}))
            })?;
            let _ = Logger::log_login(&mut conn, user.user_id, None);

            Ok(())
        }
        Ok(None) => {
            let ctx = json!({
                "title": "Login",
                "error": "Invalid username or password",
            });
            Err(Template::render("login", ctx))
        }
        Err(_e) => {
            let ctx = json!({
                "title": "Login",
                "error": format!("Authentication error: {}", _e),
            });
            Err(Template::render("login", ctx))
        }
    }
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

pub fn authenticate_user<R: Repository>(
    repo: &R,
    username: &str,
    password: &str,
) -> Result<Option<User>, String> {
    let user = repo
        .get_user_by_username(username)
        .map_err(|e| format!("Database error: {e}"))?;

    match user {
        Some(user) => match super::verify_password(password, &user.user_password) {
            Ok(true)  => Ok(Some(user)),
            Ok(false) => Ok(None),
            Err(e)    => Err(format!("Password verification error: {e}")),
        },
        None => Ok(None),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::errors::RepoError;
    use std::collections::HashMap;
    use chrono::{NaiveDate, NaiveDateTime};
    use crate::helper_functions::hash_password;

    #[derive(Default)]
    struct FakeRepo {
        users: HashMap<i32, User>,
        force_err: bool,
    }

    impl FakeRepo {
        fn with_users(users: impl IntoIterator<Item = User>) -> Self {
            let mut map = HashMap::new();
            for u in users {
                map.insert(u.user_id, u);
            }
            Self { users: map, force_err: false }
        }
        fn with_error() -> Self {
            Self { users: HashMap::new(), force_err: true }
        }
    }

    impl Repository for FakeRepo {
        fn get_user_by_id(&self, id: i32) -> Result<User, RepoError> {
            self.users.get(&id).cloned().ok_or(RepoError::NotFound)
        }
        fn get_user_by_username(&self, uname: &str) -> Result<Option<User>, RepoError> {
            if self.force_err {
                return Err(RepoError::Pool("forced test error".into()));
            }
            let user = self.users.values().find(|u| u.user_username == uname).cloned();
            Ok(user) // Ok(Some(..)) or Ok(None)
        }
    }

    // --- Fixtures ------------------------------------------------------------

    fn epoch() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()
            .and_hms_opt(0, 0, 0).unwrap()
    }

    fn make_user(id: i32, username: &str, stored_pw: &str) -> User {
        User {
            user_id: id,
            user_username: username.to_string(),
            user_name: "name".into(),
            user_email: "email@example.com".into(),
            user_level: 0,
            user_creation_date: epoch(),
            user_last_login: epoch(),
            user_password: stored_pw.into(),
            project_id: None,
            is_admin: false,
        }
    }

    // ---------- is_authenticated_impl tests ----------

    #[test]
    fn returns_user_on_match_core() {
        let mut map = HashMap::new();
        map.insert(42, make_user(42, "alice", "pwd"));
        let repo = FakeRepo { users: map, ..Default::default() };

        let got = is_authenticated_impl(&repo, Some("42"), Some("alice"));
        assert!(got.is_some());
        assert_eq!(got.unwrap().user_username, "alice");
    }

    #[test]
    fn returns_none_on_username_mismatch_core() {
        let mut map = HashMap::new();
        map.insert(42, make_user(42, "alice", "pwd"));
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
        map.insert(42, make_user(42, "alice", "pwd"));
        let repo = FakeRepo { users: map, ..Default::default() };

        let got = is_authenticated_impl(&repo, Some("not-an-int"), Some("alice"));
        assert!(got.is_none());
    }

    #[test]
    fn returns_none_when_any_cookie_missing_core() {
        let mut map = HashMap::new();
        map.insert(42, make_user(42, "alice", "pwd"));
        let repo = FakeRepo { users: map, ..Default::default() };

        assert!(is_authenticated_impl(&repo, None, Some("alice")).is_none());
        assert!(is_authenticated_impl(&repo, Some("42"), None).is_none());
        assert!(is_authenticated_impl(&repo, None, None).is_none());
    }

    // ---------- authenticate_user tests ----------

    #[test]
    fn auth_ok_when_password_matches() {
        let pwd: String = hash_password("secret").unwrap();
        let repo = FakeRepo::with_users([make_user(1, "alice", &pwd)]);
        let got = authenticate_user(&repo, "alice", "secret").expect("no error");
        assert!(got.is_some());
        assert_eq!(got.unwrap().user_username, "alice");
    }

    #[test]
    fn auth_none_when_password_mismatch() {
        let pwd = hash_password("secret").unwrap();
        let repo = FakeRepo::with_users([make_user(1, "alice", &pwd)]);
        let got = authenticate_user(&repo, "alice", "wrong").expect("no error");
        assert!(got.is_none());
    }

    #[test]
    fn auth_none_when_user_not_found() {
        let repo = FakeRepo::with_users([]);
        let got = authenticate_user(&repo, "ghost", "anything").expect("no error");
        assert!(got.is_none());
    }

    #[test]
    fn returns_err_on_repo_error() {
        let repo = FakeRepo::with_error();
        let err = authenticate_user(&repo, "alice", "secret").unwrap_err();
        assert!(err.contains("Database error"));
    }

    #[test]
    fn returns_err_when_verifier_fails() {
        // stored "ERR" triggers verifier error in our stub
        let repo = FakeRepo::with_users([make_user(1, "alice", "ERR")]);
        let err = authenticate_user(&repo, "alice", "doesnt_matter").unwrap_err();
        assert!(err.contains("Password verification error"));
    }
}

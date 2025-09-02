use rocket::http::{Cookie, CookieJar};
use rocket_dyn_templates::Template;
use rocket::serde::json::json;
use super::verify_password;
use crate::models::*;
use crate::db::get_connection_pooled_safe;
use crate::logger::Logger;
use crate::helper_functions::queries::{
    get_user_by_username,
};
use crate::repository::Repository;

// --------------------------------
// Authentication Route Logic
// --------------------------------

/// Process a login attempt. On success, session cookies are set and an empty
/// Ok is returned. On failure a rendered `Template` with the corresponding
/// error is returned.
pub fn login_user(login_form: &LoginForm, cookies: &CookieJar<'_>) -> Result<(), Template> {
    match authenticate_user(&login_form.username, &login_form.password) {
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

pub fn authenticate_user(username: &str, password: &str) -> Result<Option<User>, String> {
    authenticate_user_with(get_user_by_username, username, password)
}

pub fn is_authenticated<R: Repository>(
    repo: &R,
    cookies: &CookieJar<'_>,
) -> Option<User> {
    let (uid_cookie, uname_cookie) = (
        cookies.get_private("user_id")?,
        cookies.get_private("username")?,
    );

    let uid = uid_cookie.value().parse::<i32>().ok()?;
    let uname = uname_cookie.value();

    let user = repo.get_user_by_id(uid).ok()?;
    (user.user_username == uname).then_some(user)
}

/// Core auth logic, takes a fetcher function instead of talking to DB directly
fn authenticate_user_with<F, E>(
    fetch_user: F,
    username: &str,
    password: &str,
) -> Result<Option<User>, String>
where
    F: Fn(&str) -> Result<Option<User>, E>,
    E: std::fmt::Display,
{
    let user = fetch_user(username)
        .map_err(|e| format!("Database error: {e}"))?;

    match user {
        Some(user) => match verify_password(password, &user.user_password) {
            Ok(true)  => Ok(Some(user)),
            Ok(false) => Ok(None),
            Err(e)    => Err(format!("Password verification error: {e}")),
        },
        None => Ok(None),
    }
}




/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::helper_functions::hash_password;
    use chrono::{NaiveDate, NaiveDateTime};

    fn epoch() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    // Helper to create a full User with minimal boilerplate
    fn make_user(id: i32, username: &str) -> User {
        User {
            user_id: id,
            user_username: username.to_string(),
            user_name: "name".into(),
            user_email: "email@example.com".into(),
            user_level: 0,
            user_creation_date: epoch(),
            user_last_login: epoch(),
            user_password: "hashed".into(),
            project_id: None,
            is_admin: false,
        }
    }

    // Fake implementation we control in tests
    fn fake_get_user_by_id(id: i32) -> User {
        match id {
            42 => make_user(42, "alice"),
            _ => make_user(id, "bob"),
        }
    }

    #[test]
    fn returns_some_when_both_cookies_and_match() {
        let out = super::auth_from_cookie_values(fake_get_user_by_id, Some("42"), Some("alice"));
        let user = out.expect("should be Some");
        assert_eq!(user.user_id, 42);
        assert_eq!(user.user_username, "alice");
    }

    #[test]
    fn returns_none_when_username_mismatch() {
        let out = super::auth_from_cookie_values(fake_get_user_by_id, Some("42"), Some("wrong"));
        assert!(out.is_none());
    }

    #[test]
    fn returns_none_when_user_id_not_parseable() {
        let out =
            super::auth_from_cookie_values(fake_get_user_by_id, Some("not-an-int"), Some("alice"));
        assert!(out.is_none());
    }

    #[test]
    fn returns_none_when_any_cookie_missing() {
        assert!(super::auth_from_cookie_values(fake_get_user_by_id, None, Some("alice")).is_none());
        assert!(super::auth_from_cookie_values(fake_get_user_by_id, Some("42"), None).is_none());
        assert!(super::auth_from_cookie_values(fake_get_user_by_id, None, None).is_none());
    }

    // --- authenticate_user_with tests ---

    #[test]
    fn auth_returns_some_when_password_matches() {
        // use your own bcrypt wrapper so no extra imports are needed
        let hashed = hash_password("secret").expect("hash");
        let mut u = make_user(1, "alice");
        u.user_password = hashed;

        // Use an error type that implements Display
        let fetch = move |uname: &str| -> Result<Option<User>, &'static str> {
            assert_eq!(uname, "alice");
            Ok(Some(u.clone()))
        };

        let out = authenticate_user_with(fetch, "alice", "secret")
            .expect("no db error");

        assert!(out.is_some());
        let got = out.unwrap();
        assert_eq!(got.user_id, 1);
        assert_eq!(got.user_username, "alice");
    }

    #[test]
    fn auth_returns_none_when_password_wrong() {
        let hashed = hash_password("secret").expect("hash");
        let mut u = make_user(2, "bob");
        u.user_password = hashed;

        // fix: &'static str instead of ()
        let fetch = move |_uname: &str| -> Result<Option<User>, &'static str> {
            Ok(Some(u.clone()))
        };

        let out = authenticate_user_with(fetch, "bob", "not-the-password")
            .expect("no db error");

        assert!(out.is_none());
    }

    #[test]
    fn auth_returns_none_when_user_not_found() {
        // fix: &'static str instead of ()
        let fetch = |_uname: &str| -> Result<Option<User>, &'static str> { Ok(None) };

        let out = authenticate_user_with(fetch, "ghost", "whatever")
            .expect("no db error");

        assert!(out.is_none());
    }

    #[test]
    fn auth_maps_fetch_error_to_string() {
        let fetch = |_uname: &str| -> Result<Option<User>, &'static str> {
            Err("boom")
        };

        let err = authenticate_user_with(fetch, "alice", "secret")
            .err()
            .expect("should error");

        assert!(err.contains("Database error: boom"));
    }

    #[test]
    fn auth_propagates_password_verification_error() {
        // bcrypt will error on invalid hash format
        let mut u = make_user(3, "carol");
        u.user_password = "not-a-bcrypt-hash".into();

        // fix: &'static str instead of ()
        let fetch = move |_uname: &str| -> Result<Option<User>, &'static str> {
            Ok(Some(u.clone()))
        };

        let err = authenticate_user_with(fetch, "carol", "anything")
            .err()
            .expect("should error");

        assert!(err.starts_with("Password verification error:"));
    }
}
*/
use crate::models::*;
use diesel::prelude::*;
use rocket::http::{Cookie, CookieJar};
use rocket_dyn_templates::Template;
use rocket::serde::json::json;
use bcrypt::{hash, verify, DEFAULT_COST};
use time::Duration;

use super::queries::{
    get_user_by_id,
    get_user_by_username,
};
use crate::db::get_connection_pooled_safe;
use crate::logger::Logger;

// --------------------------------
// API
// --------------------------------

pub fn is_authenticated(cookies: &CookieJar<'_>) -> Option<User> {
    let user_id_cookie = cookies.get_private("user_id");
    let username_cookie = cookies.get_private("username");

    auth_from_cookie_values(
        |id| get_user_by_id(id),
        user_id_cookie.as_ref().map(|c| c.value()),
        username_cookie.as_ref().map(|c| c.value()),
    )
}

pub fn authenticate_user(username: &str, password: &str) -> Result<Option<User>, String> {
    authenticate_user_with(get_user_by_username, username, password)
}

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

// --------------------------------
// Implementation
// --------------------------------

fn auth_from_cookie_values<F>(
    get_user_by_id: F,
    user_id: Option<&str>,
    username: Option<&str>,
) -> Option<User>
where
    F: Fn(i32) -> User,
{
    match (user_id, username) {
        (Some(uid), Some(uname)) => match uid.parse::<i32>() {
            Ok(id) => {
                let user = get_user_by_id(id);
                if user.user_username == uname {
                    Some(user)
                } else {
                    None
                }
            }
            Err(_) => None,
        },
        _ => None,
    }
}

fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
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

pub fn change_user_password(user_id_val: i32, current_password: &str, new_password: &str) -> Result<(), String> {
    use crate::schema::users::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;

    let user_record = users
        .filter(user_id.eq(user_id_val))
        .first::<User>(connection.as_mut())
        .map_err(|e| format!("User not found: {}", e))?;

    match verify_password(current_password, &user_record.user_password) {
        Ok(true) => {
            let new_hash = hash_password(new_password)
                .map_err(|e| format!("Password hashing error: {}", e))?;

            let affected = diesel::update(users.filter(user_id.eq(user_id_val)))
                .set(user_password.eq(new_hash))
                .execute(connection.as_mut())
                .map_err(|e| format!("Database update error: {}", e))?;

            if affected == 1 {
                Ok(())
            } else {
                Err(format!("Unexpected number of rows updated: {}", affected))
            }
        }
        Ok(false) => Err("Current password is incorrect".to_string()),
        Err(e) => Err(format!("Password verification error: {}", e)),
    }
}

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

/// Clear session cookies and log the logout event.
pub fn logout_user(cookies: &CookieJar<'_>) {
    // Get user info before clearing cookies
    let user_id = cookies
        .get_private("user_id")
        .and_then(|cookie| cookie.value().parse::<i32>().ok());
    let username = cookies
        .get_private("username")
        .map(|cookie| cookie.value().to_string());

    // Clear all session cookies
    let mut user_id_cookie = Cookie::new("user_id", "");
    user_id_cookie.set_max_age(Duration::seconds(0));
    user_id_cookie.set_path("/");

    let mut username_cookie = Cookie::new("username", "");
    username_cookie.set_max_age(Duration::seconds(0));
    username_cookie.set_path("/");

    let mut user_name_cookie = Cookie::new("user_name", "");
    user_name_cookie.set_max_age(Duration::seconds(0));
    user_name_cookie.set_path("/");

    cookies.add_private(user_id_cookie);
    cookies.add_private(username_cookie);
    cookies.add_private(user_name_cookie);

    // Log logout if possible
    if let Some(uid) = user_id {
        if let Ok(mut conn) = get_connection_pooled_safe() {
            let _description = username.map(|name| format!("User {} logged out", name));
            let _ = Logger::log_logout(&mut conn, uid, None);
        }
    }
}

/// Handle a change password request, returning either a success or error template.
pub fn change_password_user(
    password_form: &ChangePasswordForm,
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

    // Validate passwords
    if password_form.new_password != password_form.confirm_password {
        let ctx = json!({
            "title": "Change Password",
            "error": "New passwords do not match",
        });
        return Err(Template::render("change_password", ctx));
    }

    if password_form.new_password.len() < 8 {
        let ctx = json!({
            "title": "Change Password",
            "error": "New password must be at least 8 characters long",
        });
        return Err(Template::render("change_password", ctx));
    }

    match change_user_password(
        user_id,
        &password_form.current_password,
        &password_form.new_password,
    ) {
        Ok(_) => {
            let ctx = json!({
                "title": "Change Password",
                "success": "Password changed successfully",
            });
            Ok(Template::render("change_password", ctx))
        }
        Err(_e) => {
            let ctx = json!({
                "title": "Change Password",
                "error": _e,
            });
            Err(Template::render("change_password", ctx))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let hashed = super::hash_password("secret").expect("hash");
        let mut u = make_user(1, "alice");
        u.user_password = hashed;

        // Use an error type that implements Display
        let fetch = move |uname: &str| -> Result<Option<User>, &'static str> {
            assert_eq!(uname, "alice");
            Ok(Some(u.clone()))
        };

        let out = super::authenticate_user_with(fetch, "alice", "secret")
            .expect("no db error");

        assert!(out.is_some());
        let got = out.unwrap();
        assert_eq!(got.user_id, 1);
        assert_eq!(got.user_username, "alice");
    }

    #[test]
    fn auth_returns_none_when_password_wrong() {
        let hashed = super::hash_password("secret").expect("hash");
        let mut u = make_user(2, "bob");
        u.user_password = hashed;

        // fix: &'static str instead of ()
        let fetch = move |_uname: &str| -> Result<Option<User>, &'static str> {
            Ok(Some(u.clone()))
        };

        let out = super::authenticate_user_with(fetch, "bob", "not-the-password")
            .expect("no db error");

        assert!(out.is_none());
    }

    #[test]
    fn auth_returns_none_when_user_not_found() {
        // fix: &'static str instead of ()
        let fetch = |_uname: &str| -> Result<Option<User>, &'static str> { Ok(None) };

        let out = super::authenticate_user_with(fetch, "ghost", "whatever")
            .expect("no db error");

        assert!(out.is_none());
    }

    #[test]
    fn auth_maps_fetch_error_to_string() {
        let fetch = |_uname: &str| -> Result<Option<User>, &'static str> {
            Err("boom")
        };

        let err = super::authenticate_user_with(fetch, "alice", "secret")
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

        let err = super::authenticate_user_with(fetch, "carol", "anything")
            .err()
            .expect("should error");

        assert!(err.starts_with("Password verification error:"));
    }
}

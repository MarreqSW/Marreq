use crate::models::*;
use diesel::prelude::*;
use rocket::http::CookieJar;
use bcrypt::{hash, verify, DEFAULT_COST};
use super::queries::get_user_by_id;


pub fn is_authenticated(cookies: &CookieJar<'_>) -> Option<User> {
    let user_id_cookie = cookies.get_private("user_id");
    let username_cookie = cookies.get_private("username");

    auth_from_cookie_values(
        |id| get_user_by_id(id),
        user_id_cookie.as_ref().map(|c| c.value()),
        username_cookie.as_ref().map(|c| c.value()),
    )
}

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

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
}

pub fn authenticate_user(username: &str, password: &str) -> Result<Option<User>, String> {
    use crate::schema::users::dsl::*;

    let mut connection = crate::db::get_connection_pooled_safe()
        .map_err(|e| format!("Database connection error: {}", e))?;

    let user = users
        .filter(user_username.eq(username))
        .first::<User>(connection.as_mut())
        .optional()
        .map_err(|_e| format!("Database error: {}", _e))?;

    match user {
        Some(user) => {
            match verify_password(password, &user.user_password) {
                Ok(true) => Ok(Some(user)),
                Ok(false) => Ok(None),
                Err(e) => Err(format!("Password verification error: {}", e)),
            }
        }
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
}

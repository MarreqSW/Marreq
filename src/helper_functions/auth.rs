use crate::models::*;
use diesel::prelude::*;
use rocket::http::CookieJar;
use bcrypt::{hash, verify, DEFAULT_COST};
use super::queries::get_user_by_id;

pub fn is_authenticated(cookies: &CookieJar<'_>) -> Option<User> {
    let user_id_cookie = cookies.get_private("user_id");
    let username_cookie = cookies.get_private("username");

    match (user_id_cookie, username_cookie) {
        (Some(user_id_cookie), Some(username_cookie)) => {
            match user_id_cookie.value().parse::<i32>() {
                Ok(user_id) => {
                    let user = get_user_by_id(user_id);
                    if user.user_username == username_cookie.value() {
                        Some(user)
                    } else {
                        None
                    }
                }
                Err(_) => None
            }
        }
        _ => None
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

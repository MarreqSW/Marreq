use bcrypt::{hash, verify, DEFAULT_COST};
use crate::models::*;
use diesel::prelude::*;
use rocket::http::CookieJar;
use rocket_dyn_templates::Template;
use rocket::serde::json::json;

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password, DEFAULT_COST)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password, hash)
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

    match change_user_password(
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
        Err(_e) => {
            let ctx = json!({
                "title": "Change Password",
                "error": _e,
            });
            Err(Template::render("change_password", ctx))
        }
    }
}


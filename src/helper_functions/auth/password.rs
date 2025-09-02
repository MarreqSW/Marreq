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
    repo: &R,
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
    let repo = DieselRepo{};

    match change_user_password(
        &repo,
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


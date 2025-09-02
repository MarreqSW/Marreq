use time::Duration;
use rocket::http::{CookieJar, Cookie};
use crate::db::get_connection_pooled_safe;
use crate::logger::Logger;

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
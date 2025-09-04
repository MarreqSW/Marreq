use rocket::http::{CookieJar, Cookie};
use crate::repository::get_connection_pooled_safe;
use crate::logger::Logger;

/// Clear session cookies and log the logout event.
/// Note: login calls Cookie::new(..). By default, Rocket’s Cookie::new creates
/// a cookie without any explicit path, and Rocket’s add_private will add it to
/// the response as-is. The default path is handled by the browser: if no Path 
/// is given, RFC 6265 says the default is the request’s path up to the rightmost “/”.
pub fn logout_user(cookies: &CookieJar<'_>) {
    // Get user info before clearing cookies
    let user_id = cookies
        .get_private("user_id")
        .and_then(|cookie| cookie.value().parse::<i32>().ok());
    let username = cookies
        .get_private("username")
        .map(|cookie| cookie.value().to_string());

    // Remove all session cookies
    for name in &["user_id", "username", "user_name"] {
        // Important: same path as set at login
        let c = Cookie::build(*name)
            .path("/")
            .build();
        cookies.remove_private(c);
    }

    // Log logout if possible
    if let Some(uid) = user_id {
        if let Ok(mut conn) = get_connection_pooled_safe() {
            let _description = username.map(|name| format!("User {} logged out", name));
            let _ = Logger::log_logout(&mut conn, uid, None);
        }
    }
}
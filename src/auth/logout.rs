use rocket::http::{CookieJar, Cookie};
use crate::repository::DieselRepo;
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
        if let Ok(mut conn) = DieselRepo::new().get_conn() {
            let _description = username.map(|name| format!("User {} logged out", name));
            let _ = Logger::log_logout(&mut conn, uid, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::local::blocking::Client;
    use rocket::{get, routes};

    #[get("/")]
    fn set_basic(cookies: &CookieJar<'_>) {
        cookies.add_private(Cookie::new("user_id", "not-an-int"));
        cookies.add_private(Cookie::new("username", "alice"));
        cookies.add_private(Cookie::new("user_name", "Alice"));
    }

    #[test]
    fn removes_session_cookies() {
        let rocket = rocket::build().mount("/", routes![set_basic]);
        let client = Client::tracked(rocket).expect("valid rocket");
        client.get("/").dispatch();
        let jar = client.cookies();
        assert!(jar.get_private("username").is_some());

        logout_user(&jar);

        assert!(jar.get_pending("user_id").is_none());
        assert!(jar.get_pending("username").is_none());
        assert!(jar.get_pending("user_name").is_none());
    }

    #[get("/")]
    fn set_with_other(cookies: &CookieJar<'_>) {
        cookies.add(Cookie::new("other", "ok"));
        cookies.add_private(Cookie::new("user_id", "bad"));
        cookies.add_private(Cookie::new("username", "bob"));
        cookies.add_private(Cookie::new("user_name", "Bob"));
    }

    #[test]
    fn leaves_other_cookies_intact() {
        let rocket = rocket::build().mount("/", routes![set_with_other]);
        let client = Client::tracked(rocket).expect("valid rocket");
        client.get("/").dispatch();
        let jar = client.cookies();

        logout_user(&jar);

        assert!(jar.get_pending("user_id").is_none());
        assert!(jar.get_pending("username").is_none());
        assert!(jar.get_pending("user_name").is_none());
        assert!(jar.get_pending("other").is_some());
    }
}

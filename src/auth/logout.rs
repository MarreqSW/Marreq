use crate::auth::{clear_session_cookie, read_session_user_id};
use crate::logger::{LogCtx, Logger};
use crate::repository::DieselRepo;
use rocket::http::CookieJar;

/// Clear session cookies and log the logout event.
/// Note: login calls Cookie::new(..). By default, Rocket’s Cookie::new creates
/// a cookie without any explicit path, and Rocket’s add_private will add it to
/// the response as-is. The default path is handled by the browser: if no Path
/// is given, RFC 6265 says the default is the request’s path up to the rightmost “/”.
pub fn logout_user(cookies: &CookieJar<'_>) {
    // Get user info before clearing cookies
    let id = read_session_user_id(cookies);

    // Remove the session cookie
    clear_session_cookie(cookies);

    // Remove legacy cookies from previous versions if they exist
    for legacy in &["username", "name"] {
        let mut cookie = rocket::http::Cookie::new(*legacy, "");
        cookie.set_path("/");
        cookies.remove_private(cookie);
    }

    // Log logout if possible
    if let Some(uid) = id {
        if let Ok(mut conn) = DieselRepo::new().get_conn() {
            let ctx = LogCtx::new(uid);
            let _ = Logger::log_logout(&mut conn, &ctx);
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
        cookies.add_private(rocket::http::Cookie::new("id", "not-an-int"));
        cookies.add_private(rocket::http::Cookie::new("username", "alice"));
        cookies.add_private(rocket::http::Cookie::new("name", "Alice"));
    }

    #[test]
    fn removes_session_cookies() {
        let rocket = rocket::build().mount("/", routes![set_basic]);
        let client = Client::tracked(rocket).expect("valid rocket");
        client.get("/").dispatch();
        let jar = client.cookies();
        assert!(jar.get_private("username").is_some());

        logout_user(&jar);

        assert!(jar.get_pending("id").is_none());
        assert!(jar.get_pending("username").is_none());
        assert!(jar.get_pending("name").is_none());
    }

    #[get("/")]
    fn set_with_other(cookies: &CookieJar<'_>) {
        cookies.add(rocket::http::Cookie::new("other", "ok"));
        cookies.add_private(rocket::http::Cookie::new("id", "bad"));
        cookies.add_private(rocket::http::Cookie::new("username", "bob"));
        cookies.add_private(rocket::http::Cookie::new("name", "Bob"));
    }

    #[test]
    fn leaves_other_cookies_intact() {
        let rocket = rocket::build().mount("/", routes![set_with_other]);
        let client = Client::tracked(rocket).expect("valid rocket");
        client.get("/").dispatch();
        let jar = client.cookies();

        logout_user(&jar);

        assert!(jar.get_pending("id").is_none());
        assert!(jar.get_pending("username").is_none());
        assert!(jar.get_pending("name").is_none());
        assert!(jar.get_pending("other").is_some());
    }
}

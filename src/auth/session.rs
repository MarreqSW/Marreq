use rocket::http::{Cookie, CookieJar};

/// Name of the session cookie used to store the authenticated user id.
pub const SESSION_COOKIE: &str = "id";

/// Store the authenticated user's id in a private cookie.
pub fn set_session_cookie(cookies: &CookieJar<'_>, user_id: i32) {
    let mut cookie = Cookie::new(SESSION_COOKIE, user_id.to_string());
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookies.add_private(cookie);
}

/// Remove the session cookie, effectively logging the user out.
pub fn clear_session_cookie(cookies: &CookieJar<'_>) {
    let mut cookie = Cookie::new(SESSION_COOKIE, "");
    cookie.set_path("/");
    cookies.remove_private(cookie);
}

/// Helper used by request guards to fetch the authenticated user's id.
pub fn read_session_user_id(cookies: &CookieJar<'_>) -> Option<i32> {
    cookies
        .get_private(SESSION_COOKIE)
        .and_then(|cookie| cookie.value().parse::<i32>().ok())
}

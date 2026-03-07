// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::http::{Cookie, CookieJar, SameSite};

/// Name of the session cookie when using HTTPS (OWASP ASVS V3.3.1/V3.3.2).
/// The `__Host-` prefix requires Secure, Path=/, no Domain; browsers reject it on HTTP.
pub const SESSION_COOKIE: &str = "__Host-session";

/// Name used when not using HTTPS (e.g. localhost). Browsers accept it without Secure.
const SESSION_COOKIE_INSECURE: &str = "session";

/// True if we should use the non-__Host- cookie (no Secure). Default true so localhost (HTTP) works;
/// set MARREQ_SECURE_SESSION_COOKIE=1 in production over HTTPS to use __Host-session.
fn use_insecure_session_cookie() -> bool {
    std::env::var("MARREQ_SECURE_SESSION_COOKIE")
        .map(|v| !matches!(v.as_str(), "1" | "true" | "yes"))
        .unwrap_or(true)
}

/// Store the authenticated user's id in a hardened private cookie.
/// On HTTPS uses `__Host-session` with Secure; on HTTP (when MARREQ_INSECURE_SESSION_COOKIE
/// is set) uses `session` without Secure so the cookie is stored on localhost.
pub fn set_session_cookie(cookies: &CookieJar<'_>, user_id: i32) {
    let name = if use_insecure_session_cookie() {
        SESSION_COOKIE_INSECURE
    } else {
        SESSION_COOKIE
    };
    let mut cookie = Cookie::new(name, user_id.to_string());
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    if !use_insecure_session_cookie() {
        cookie.set_secure(true);
    }
    cookies.add_private(cookie);
}

/// Remove the session cookie(s), effectively logging the user out.
pub fn clear_session_cookie(cookies: &CookieJar<'_>) {
    for name in [SESSION_COOKIE, SESSION_COOKIE_INSECURE] {
        let mut cookie = Cookie::new(name, "");
        cookie.set_path("/");
        if name == SESSION_COOKIE {
            cookie.set_secure(true);
        }
        cookie.set_same_site(SameSite::Strict);
        cookies.remove_private(cookie);
    }
}

/// Name of the session cookie used when setting the cookie in the current environment
/// (HTTP/insecure: "session", HTTPS/secure: "__Host-session"). Use this in tests to
/// assert the cookie was set.
pub fn session_cookie_name_for_request() -> &'static str {
    if use_insecure_session_cookie() {
        "session"
    } else {
        SESSION_COOKIE
    }
}

/// Helper used by request guards to fetch the authenticated user's id.
/// Tries both cookie names so sessions set over HTTP or HTTPS are recognized.
pub fn read_session_user_id(cookies: &CookieJar<'_>) -> Option<i32> {
    [SESSION_COOKIE, SESSION_COOKIE_INSECURE]
        .into_iter()
        .find_map(|name| {
            cookies
                .get_private(name)
                .and_then(|cookie| cookie.value().parse::<i32>().ok())
        })
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::http::{Cookie, CookieJar, SameSite};

/// Name of the session cookie.
///
/// The `__Host-` prefix enforces three browser-level constraints automatically:
///   1. The `Secure` attribute MUST be present (cookie only sent over HTTPS).
///   2. The `Path` MUST be `/` (no path-scoping tricks).
///   3. The `Domain` attribute MUST be absent (strictly host-bound).
///
/// Together these satisfy OWASP ASVS V3.3.1 and V3.3.2.
pub const SESSION_COOKIE: &str = "__Host-session";

/// Store the authenticated user's id in a hardened private cookie.
///
/// Attributes set:
/// - `HttpOnly`    – not accessible from JavaScript (XSS mitigation).
/// - `Secure`      – only sent over HTTPS; required for the `__Host-` prefix.
/// - `SameSite=Strict` – never sent on cross-site requests (CSRF mitigation).
/// - `Path=/`      – required for the `__Host-` prefix.
/// - No `Domain`   – required for the `__Host-` prefix (host-bound).
/// Store the authenticated user's id in a private cookie.
///
/// Security attributes (ASVS V3.4):
/// - `HttpOnly=true` – prevents JavaScript from accessing the session token.
/// - `SameSite=Lax` – mitigates CSRF: the cookie is not sent on
///   cross-site non-safe requests initiated by third-party pages (first-line
///   CSRF defence alongside the [`CsrfFairing`](crate::fairings::CsrfFairing)).
pub fn set_session_cookie(cookies: &CookieJar<'_>, user_id: i32) {
    let mut cookie = Cookie::new(SESSION_COOKIE, user_id.to_string());
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookies.add_private(cookie);
}

/// Remove the session cookie, effectively logging the user out.
///
/// The removal cookie must carry the same `Secure`, `Path`, and (absent)
/// `Domain` attributes as the original so that the `__Host-` prefix rules
/// are satisfied and browsers honour the `Max-Age=0` expiry.
pub fn clear_session_cookie(cookies: &CookieJar<'_>) {
    let mut cookie = Cookie::new(SESSION_COOKIE, "");
    cookie.set_path("/");
    cookie.set_secure(true);
    cookie.set_same_site(SameSite::Strict);
    cookies.remove_private(cookie);
}

/// Helper used by request guards to fetch the authenticated user's id.
pub fn read_session_user_id(cookies: &CookieJar<'_>) -> Option<i32> {
    cookies
        .get_private(SESSION_COOKIE)
        .and_then(|cookie| cookie.value().parse::<i32>().ok())
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! CSRF token utilities (ASVS V3.5.1).
//!
//! Implements the **double-submit cookie** pattern:
//!
//! 1. On login (and on anonymous page loads that produce a CSRF seed), a
//!    cryptographically-random token is minted and stored in the `csrf` private
//!    cookie (encrypted + authenticated by Rocket's secret key).
//! 2. The token is surfaced to all authenticated template contexts as
//!    `csrf_token`, so HTML templates can embed it in a `<meta>` tag and
//!    JavaScript can forward it as the `X-CSRF-Token` request header.
//! 3. The [`CsrfFairing`](crate::fairings::CsrfFairing) validates every
//!    unsafe request by checking either:
//!    - the `X-CSRF-Token` header against the `csrf` cookie value, **or**
//!    - the `Origin` / `Referer` header against the application's own origin.

use rand_core::{OsRng, RngCore};
use rocket::http::{Cookie, CookieJar, SameSite};

/// Name of the private CSRF cookie that stores the token.
pub const CSRF_COOKIE: &str = "csrf";

/// HTTP request header that clients (AJAX / fetch) attach to carry the token.
pub const CSRF_HEADER: &str = "X-CSRF-Token";

/// Generate a cryptographically-random CSRF token (32 bytes encoded as 64
/// lowercase hex characters).
pub fn generate_csrf_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Store `token` in a private (encrypted) cookie named [`CSRF_COOKIE`].
///
/// Attributes:
/// - `Path=/` – visible to all routes.
/// - `HttpOnly=true` – protects the cookie from direct JS access; the token
///   is still surfaced to templates via context so AJAX clients can read it
///   from the DOM (e.g., a `<meta name="csrf-token">` tag).
/// - `SameSite=Lax` – first-line defence: browsers do not attach the cookie
///   on cross-site non-safe requests in the common case.
pub fn set_csrf_cookie(cookies: &CookieJar<'_>, token: String) {
    let mut cookie = Cookie::new(CSRF_COOKIE, token);
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookies.add_private(cookie);
}

/// Read the CSRF token stored in the private cookie, or `None` when absent.
pub fn read_csrf_token(cookies: &CookieJar<'_>) -> Option<String> {
    cookies
        .get_private(CSRF_COOKIE)
        .map(|c| c.value().to_string())
}

/// Remove the CSRF cookie, invalidating all outstanding tokens.
///
/// Called on logout to ensure tokens cannot be replayed after the session
/// ends.
pub fn clear_csrf_cookie(cookies: &CookieJar<'_>) {
    let mut cookie = Cookie::new(CSRF_COOKIE, "");
    cookie.set_path("/");
    cookies.remove_private(cookie);
}

/// Return the token already stored in the `csrf` cookie, or generate a new
/// one and set the cookie if none exists.
pub fn get_or_create_csrf_token(cookies: &CookieJar<'_>) -> String {
    if let Some(token) = read_csrf_token(cookies) {
        return token;
    }
    let token = generate_csrf_token();
    set_csrf_cookie(cookies, token.clone());
    token
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_64_hex_chars() {
        let token = generate_csrf_token();
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn tokens_are_unique() {
        let t1 = generate_csrf_token();
        let t2 = generate_csrf_token();
        assert_ne!(t1, t2);
    }
}

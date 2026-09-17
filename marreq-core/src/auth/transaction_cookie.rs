// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::OAuthTransaction;
use rocket::http::{Cookie, CookieJar, SameSite};

const COOKIE_NAME: &str = "oauth_transaction";

pub fn store(
    cookies: &CookieJar<'_>,
    transaction: &OAuthTransaction,
) -> Result<(), serde_json::Error> {
    let value = serde_json::to_string(transaction)?;
    let mut cookie = Cookie::new(COOKIE_NAME, value);
    cookie.set_path("/api/auth/external");
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookie.set_max_age(time::Duration::minutes(10));
    if crate::config::AppConfig::try_current().is_some_and(|config| config.secure_session_cookie) {
        cookie.set_secure(true);
    }
    cookies.add_private(cookie);
    Ok(())
}

/// Read and delete first so every callback attempt consumes the transaction,
/// including malformed, mismatched, and provider-error callbacks.
pub fn take(cookies: &CookieJar<'_>) -> Option<OAuthTransaction> {
    let cookie = cookies.get_private(COOKIE_NAME)?;
    let parsed = serde_json::from_str(cookie.value()).ok();
    let mut removal = Cookie::new(COOKIE_NAME, "");
    removal.set_path("/api/auth/external");
    removal.set_same_site(SameSite::Lax);
    cookies.remove_private(removal);
    parsed
}

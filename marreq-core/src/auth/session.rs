// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Server-side session tokens.
//!
//! Replaces the previous "private cookie carries the user_id" scheme with a
//! random opaque token whose SHA-256 hash is stored in the `sessions` table.
//! The cookie is still a Rocket *private* cookie (AEAD-encrypted at rest in
//! the browser) but the value is now an unguessable token, and revocation
//! works by deleting the row (single-device logout, password change, or
//! "log out everywhere"). Mirrors the existing [`crate::models::EmailToken`]
//! design.

use crate::app::AppState;
use crate::models::entities::NewSession;
use crate::repository::SessionRepository;
use base64::Engine;
use rocket::http::{Cookie, CookieJar, SameSite};
use sha2::{Digest, Sha256};

/// Name of the session cookie when using HTTPS (OWASP ASVS V3.3.1/V3.3.2).
/// The `__Host-` prefix requires Secure, Path=/, no Domain; browsers reject it on HTTP.
pub const SESSION_COOKIE: &str = "__Host-session";

/// Name used when not using HTTPS (e.g. localhost).
const SESSION_COOKIE_INSECURE: &str = "session";

/// Lifetime of a fresh session.
const SESSION_TTL_DAYS: i64 = 30;

/// 256 bits of entropy → base64url ≈ 43 chars.
const TOKEN_BYTES: usize = 32;

fn use_insecure_session_cookie() -> bool {
    !crate::config::AppConfig::try_current()
        .map(|c| c.secure_session_cookie)
        .unwrap_or(false)
}

fn cookie_name() -> &'static str {
    if use_insecure_session_cookie() {
        SESSION_COOKIE_INSECURE
    } else {
        SESSION_COOKIE
    }
}

/// Cookie-name reported for tests/assertions in the current environment.
pub fn session_cookie_name_for_request() -> &'static str {
    cookie_name()
}

fn generate_raw_token() -> String {
    use rand::RngCore;
    let mut buf = [0u8; TOKEN_BYTES];
    rand::thread_rng().fill_bytes(&mut buf);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf)
}

/// Hash the raw cookie value to the form stored in `sessions.token_hash`.
pub fn hash_token(raw: &str) -> String {
    let digest = Sha256::digest(raw.as_bytes());
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        write!(out, "{:02x}", b).expect("write to String");
    }
    out
}

fn build_cookie(name: &'static str, value: String) -> Cookie<'static> {
    let mut cookie = Cookie::new(name, value);
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    if name == SESSION_COOKIE {
        cookie.set_secure(true);
    }
    cookie
}

/// Issue a fresh session for `user_id`: insert a row and set the cookie.
///
/// Caller already holds the repository write lock (e.g. `state.repo_write()`)
/// because this is normally invoked from a write-side handler such as login.
/// On any repository error the cookie is *not* set, so the caller will see
/// the request as unauthenticated rather than a fake authenticated one.
pub fn set_session_cookie<R: SessionRepository>(
    cookies: &CookieJar<'_>,
    repo: &mut R,
    user_id: i32,
    user_agent: Option<String>,
    ip_addr: Option<String>,
) {
    let raw = generate_raw_token();
    let token_hash = hash_token(&raw);
    let expires_at = chrono::Utc::now().naive_utc() + chrono::Duration::days(SESSION_TTL_DAYS);

    let new = NewSession {
        token_hash,
        user_id,
        expires_at,
        user_agent,
        ip_addr,
    };

    if repo.create_session(&new).is_ok() {
        cookies.add_private(build_cookie(cookie_name(), raw));
    }
}

/// Resolve the cookie to a `user_id`, validating the session against the DB.
/// Returns `None` if no cookie, an unknown token, or expiry.
///
/// `last_seen_at` is *not* updated here (would require a write lock from a
/// read-only guard path); a periodic background job can opportunistically
/// refresh it via [`SessionRepository::touch_session`].
pub fn read_session_user_id<R: SessionRepository>(
    cookies: &CookieJar<'_>,
    repo: &R,
) -> Option<i32> {
    let raw = [SESSION_COOKIE, SESSION_COOKIE_INSECURE]
        .into_iter()
        .find_map(|n| cookies.get_private(n).map(|c| c.value().to_owned()))?;
    let token_hash = hash_token(&raw);
    let now = chrono::Utc::now().naive_utc();

    repo.find_active_session(&token_hash, now)
        .ok()
        .flatten()
        .map(|s| s.user_id)
}

/// Variant for callers that only have an [`AppState`] in hand and don't
/// already hold a lock. Acquires a *read* lock for the lookup.
pub fn read_session_user_id_via_state(cookies: &CookieJar<'_>, state: &AppState) -> Option<i32> {
    let repo = state.try_repo_read().ok()?;
    read_session_user_id(cookies, &*repo)
}

/// Revoke the current session (if any) and clear the cookies.
pub fn clear_session_cookie<R: SessionRepository>(cookies: &CookieJar<'_>, repo: &mut R) {
    for name in [SESSION_COOKIE, SESSION_COOKIE_INSECURE] {
        if let Some(c) = cookies.get_private(name) {
            let token_hash = hash_token(c.value());
            let _ = repo.delete_session(&token_hash);
        }
        let mut cookie = Cookie::new(name, "");
        cookie.set_path("/");
        if name == SESSION_COOKIE {
            cookie.set_secure(true);
        }
        cookie.set_same_site(SameSite::Strict);
        cookies.remove_private(cookie);
    }
}

/// Variant of [`clear_session_cookie`] that acquires the write lock itself.
pub fn clear_session_cookie_via_state(cookies: &CookieJar<'_>, state: &AppState) {
    if let Ok(mut repo) = state.try_repo_write() {
        clear_session_cookie(cookies, &mut *repo);
    }
}

/// Revoke every session belonging to `user_id`. Use after password change.
pub fn revoke_all_user_sessions<R: SessionRepository>(repo: &mut R, user_id: i32) {
    let _ = repo.delete_user_sessions(user_id);
}

/// Test helper: insert a session row for `user_id` and return the cookie that
/// authenticates as that user. Tests call this *after* constructing the
/// Rocket [`AppState`] but before launching the [`rocket::local::blocking::Client`].
#[cfg(any(test, feature = "test-helpers"))]
pub fn test_session_cookie_for(state: &AppState, user_id: i32) -> Cookie<'static> {
    let raw = generate_raw_token();
    let token_hash = hash_token(&raw);
    let expires_at = chrono::Utc::now().naive_utc() + chrono::Duration::days(SESSION_TTL_DAYS);
    let new = NewSession {
        token_hash,
        user_id,
        expires_at,
        user_agent: None,
        ip_addr: None,
    };
    state
        .repo_write()
        .create_session(&new)
        .expect("create test session");
    build_cookie(cookie_name(), raw)
}

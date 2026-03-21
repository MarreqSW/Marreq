// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! CSRF protection fairing (ASVS V3.5.1).
//!
//! [`CsrfFairing`] enforces two overlapping defenses for every **unsafe** HTTP
//! method (`POST`, `PUT`, `PATCH`, `DELETE`):
//!
//! ## Defense 1 – Origin / Referer validation
//!
//! When a browser initiates a cross-origin request it always attaches an
//! `Origin` header whose value is the *requesting* origin.  The fairing
//! compares this against the application's own origin (computed from Rocket
//! config + the `CSRF_ALLOWED_ORIGINS` env var).  A mismatch → `403`.
//!
//! If `Origin` is absent the fairing falls back to `Referer`.  If neither
//! header is present for an **authenticated** request (session cookie found),
//! the request is also rejected.
//!
//! ## Defense 2 – `X-CSRF-Token` header (for AJAX / fetch clients)
//!
//! JavaScript clients embed the token from the `<meta name="csrf-token">`
//! page element and send it as `X-CSRF-Token: <value>`.  The fairing
//! validates this header against the `csrf` private cookie.  A valid token
//! allows the request through regardless of the `Origin` header (useful for
//! valid same-origin AJAX that may omit `Origin`).
//!
//! ## Exemptions
//!
//! * Requests that carry `Authorization: Bearer …` are using API-token auth
//!   and are **not** CSRF-vulnerable, so they are unconditionally forwarded.
//! * Safe methods (`GET`, `HEAD`, `OPTIONS`) are never checked.
//!
//! ## Rejection mechanism
//!
//! When a request must be rejected the fairing rewrites it to
//! `GET /_csrf_denied`.  A dedicated route registered in [`app.rs`] responds
//! with `403 Forbidden`.  This technique avoids executing the intended handler
//! while still going through Rocket's normal response pipeline (and therefore
//! the response fairings, including cache-control headers).

use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};

use rocket::fairing::{self, Fairing, Info, Kind};
use rocket::http::{uri::Origin, Method, Status};
use rocket::response::status;
use rocket::{Build, Data, Request, Rocket};

use crate::auth::csrf::{CSRF_COOKIE, CSRF_HEADER};
use crate::auth::session::SESSION_COOKIE;

// ---------------------------------------------------------------------------
// Public helpers for tests
// ---------------------------------------------------------------------------

/// The path the fairing rewrites rejected requests to.
pub const CSRF_DENIED_PATH: &str = "/_csrf_denied";

/// Rocket route that responds to CSRF-rewritten requests.
///
/// Mount this at `/` in `app.rs`:
/// ```rust
/// use marreq::fairings::csrf::csrf_denied;
///
/// let rocket = rocket::build().mount("/", rocket::routes![csrf_denied]);
/// # let _ = rocket;
/// ```
#[get("/_csrf_denied")]
pub fn csrf_denied() -> status::Custom<&'static str> {
    status::Custom(Status::Forbidden, "403 Forbidden – CSRF validation failed")
}

// ---------------------------------------------------------------------------
// CsrfFairing
// ---------------------------------------------------------------------------

/// Rocket fairing that enforces CSRF protection on all unsafe HTTP methods.
///
/// Attach via:
/// ```rust
/// use marreq::fairings::csrf::CsrfFairing;
///
/// let rocket = rocket::build().attach(CsrfFairing::new());
/// # let _ = rocket;
/// ```
pub struct CsrfFairing {
    /// Allowed `Origin` values.  Mutated in `on_ignite`; read in `on_request`.
    allowed_origins: Arc<RwLock<HashSet<String>>>,
}

impl CsrfFairing {
    /// Create the fairing, seeding allowed origins from the
    /// `CSRF_ALLOWED_ORIGINS` environment variable (comma-separated list of
    /// full origins, e.g. `https://app.example.com,http://localhost:8000`).
    /// The Rocket-configured origin is added automatically in `on_ignite`.
    pub fn new() -> Self {
        let mut origins = HashSet::new();
        if let Ok(env_val) = std::env::var("CSRF_ALLOWED_ORIGINS") {
            for part in env_val.split(',') {
                let trimmed = part.trim().to_string();
                if !trimmed.is_empty() {
                    origins.insert(trimmed);
                }
            }
        }
        Self {
            allowed_origins: Arc::new(RwLock::new(origins)),
        }
    }

    /// Add an origin to the runtime allowlist (useful in tests).
    pub fn with_origin(self, origin: impl Into<String>) -> Self {
        if let Ok(mut origins) = self.allowed_origins.write() {
            origins.insert(origin.into());
        }
        self
    }

    /// Check whether `origin` (without trailing slash) is on the allowlist.
    fn is_allowed(&self, origin: &str) -> bool {
        // Normalise: strip trailing slash for comparison.
        let normalised = origin.trim_end_matches('/');
        self.allowed_origins
            .read()
            .map(|set| set.contains(normalised))
            .unwrap_or(false)
    }
}

impl Default for CsrfFairing {
    fn default() -> Self {
        Self::new()
    }
}

#[rocket::async_trait]
impl Fairing for CsrfFairing {
    fn info(&self) -> Info {
        Info {
            name: "CSRF Protection (ASVS V3.5.1)",
            kind: Kind::Ignite | Kind::Request,
        }
    }

    /// Compute the application's own origin from the Rocket configuration and
    /// add it (and `localhost` equivalents for loopback addresses) to the
    /// allowlist.
    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        // `Rocket<Build>` exposes configuration via `figment()`.
        let config: rocket::Config = rocket.figment().extract().unwrap_or_default();

        let scheme = if config.tls_enabled() {
            "https"
        } else {
            "http"
        };
        let addr = config.address;
        let port = config.port;

        // Primary origin from config address
        let computed = format!("{}://{}:{}", scheme, addr, port);

        if let Ok(mut origins) = self.allowed_origins.write() {
            origins.insert(computed);

            // Also add localhost / 127.0.0.1 for loopback or unspecified addresses
            // so that developers using `http://localhost:<port>` are not blocked.
            let is_local = match addr {
                IpAddr::V4(a) => a.is_loopback() || a.is_unspecified(),
                IpAddr::V6(a) => a.is_loopback() || a.is_unspecified(),
            };
            if is_local {
                origins.insert(format!("{}://localhost:{}", scheme, port));
                origins.insert(format!("{}://127.0.0.1:{}", scheme, port));
            }

            // Split-stack SPA: browser `Origin` is the public UI (e.g. nginx :8080), not Rocket’s bind port.
            for extra in [
                "http://127.0.0.1:8080",
                "http://localhost:8080",
                "http://127.0.0.1:8000",
                "http://localhost:8000",
            ] {
                origins.insert(extra.to_string());
            }
        }

        Ok(rocket)
    }

    /// For each unsafe method:
    ///
    /// 1. Skip Bearer-authenticated requests.
    /// 2. Accept if `X-CSRF-Token` header matches the `csrf` private cookie.
    /// 3. Accept if `Origin` (or `Referer` fallback) is on the allowlist.
    /// 4. Otherwise rewrite to `GET /_csrf_denied`.
    async fn on_request(&self, req: &mut Request<'_>, _: &mut Data<'_>) {
        let method = req.method();
        let is_unsafe = matches!(
            method,
            Method::Post | Method::Put | Method::Patch | Method::Delete
        );
        if !is_unsafe {
            return;
        }

        // --- Exemption: Bearer API-token auth is not CSRF-vulnerable ---
        if req
            .headers()
            .get_one("Authorization")
            .map(|h| h.starts_with("Bearer "))
            .unwrap_or(false)
        {
            return;
        }

        // SPA auth endpoints: allow allowlisted `Origin` / `Referer` before double-submit checks.
        // Scoped to login/logout only so other `/api/*` mutating calls still require a matching
        // `X-CSRF-Token` + `csrf` cookie (non-browser clients cannot forge a browser `Origin`).
        let path = req.uri().path().as_str();
        let api_auth_origin_only = matches!(path, "/api/auth/login" | "/api/auth/logout");
        if api_auth_origin_only {
            if let Some(origin) = req.headers().get_one("Origin") {
                if self.is_allowed(origin) {
                    return;
                }
            }
            if let Some(referer) = req.headers().get_one("Referer") {
                if let Some(ro) = extract_origin_from_url(referer) {
                    if self.is_allowed(&ro) {
                        return;
                    }
                }
            }
        }

        // --- Defense 2: X-CSRF-Token header vs csrf cookie ---
        let csrf_header_val = req.headers().get_one(CSRF_HEADER).map(str::to_owned);
        let csrf_cookie_val = req
            .cookies()
            .get_private(CSRF_COOKIE)
            .map(|c| c.value().to_string());

        if let (Some(header_token), Some(cookie_token)) = (&csrf_header_val, &csrf_cookie_val) {
            if header_token == cookie_token {
                return; // Valid double-submit; allow.
            }
            // Token present but mismatched – always reject, regardless of Origin.
            reject_request(req);
            return;
        }

        // --- Defense 1: Origin / Referer header ---
        if let Some(origin) = req.headers().get_one("Origin") {
            if self.is_allowed(origin) {
                return; // Same-site origin; allow.
            }
            // Origin present but not allowed.
            reject_request(req);
            return;
        }

        // Origin absent – try Referer as a weaker fallback.
        if let Some(referer) = req.headers().get_one("Referer") {
            // Extract just the `scheme://host[:port]` prefix of the Referer URL.
            let referer_origin = extract_origin_from_url(referer);
            if let Some(ro) = referer_origin {
                if self.is_allowed(&ro) {
                    return; // Referer matches; allow.
                }
                // Referer present and does not match.
                reject_request(req);
                return;
            }
        }

        // Neither Origin nor Referer present.
        // Only reject when the request is authenticated via session cookie to
        // avoid breaking unauthenticated API / health-check calls that
        // legitimately lack an Origin header.
        let has_session = req.cookies().get_private(SESSION_COOKIE).is_some();
        if has_session {
            reject_request(req);
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Rewrite the request so Rocket routes it to the `csrf_denied` handler.
fn reject_request(req: &mut Request<'_>) {
    req.set_method(Method::Get);
    if let Ok(uri) = Origin::parse(CSRF_DENIED_PATH) {
        req.set_uri(uri);
    }
}

/// Extract `scheme://host[:port]` from a full URL string.
///
/// Returns `None` if the URL cannot be parsed.
fn extract_origin_from_url(url: &str) -> Option<String> {
    // Find end of scheme (e.g. "https://")
    let after_scheme = url.find("://")?;
    let rest = &url[after_scheme + 3..];
    // Everything up to the first '/' (or end of string) is `host[:port]`
    let host_port = rest.split('/').next().unwrap_or(rest);
    let scheme = &url[..after_scheme];
    Some(format!("{}://{}", scheme, host_port))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_origin_parses_full_url() {
        assert_eq!(
            extract_origin_from_url("https://app.example.com/some/path?q=1"),
            Some("https://app.example.com".to_string())
        );
    }

    #[test]
    fn extract_origin_parses_url_with_port() {
        assert_eq!(
            extract_origin_from_url("http://localhost:8000/login"),
            Some("http://localhost:8000".to_string())
        );
    }

    #[test]
    fn extract_origin_returns_none_for_garbage() {
        assert_eq!(extract_origin_from_url("not-a-url"), None);
    }

    #[test]
    fn is_allowed_matches_exact_origin() {
        let fairing = CsrfFairing::new().with_origin("http://localhost:8000");
        assert!(fairing.is_allowed("http://localhost:8000"));
        assert!(!fairing.is_allowed("http://evil.example.com"));
    }

    #[test]
    fn is_allowed_strips_trailing_slash() {
        let fairing = CsrfFairing::new().with_origin("http://localhost:8000");
        assert!(fairing.is_allowed("http://localhost:8000/"));
    }
}

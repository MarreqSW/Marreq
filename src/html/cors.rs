// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! CORS policy fairing (ASVS V3.4.2).
//!
//! Only origins in the configured allowlist receive `Access-Control-Allow-Origin`.
//! `*` is never emitted.  `Vary: Origin` is always added when the header is set
//! to prevent intermediaries from caching a response for the wrong origin.

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{http::Method, http::Status, Request, Response};

/// The set of trusted origins and credential policy for cross-origin requests.
///
/// Build with [`CorsPolicy::from_env`] at application startup, or with
/// [`CorsPolicy::new`] in tests / custom scenarios.
pub struct CorsPolicy {
    /// Exact origins that are permitted to make cross-origin requests.
    /// An empty list means CORS headers are never emitted (safe default for
    /// production deployments where UI and API share the same origin).
    allowed_origins: Vec<String>,
    /// Whether to include `Access-Control-Allow-Credentials: true`.
    /// Must be `false` when using wildcard origins (we never use wildcards, but
    /// keeping this explicit avoids accidental misconfiguration).
    allow_credentials: bool,
}

impl CorsPolicy {
    /// Construct with an explicit origin allowlist.  No `*` wildcard is used.
    pub fn new(allowed_origins: Vec<String>, allow_credentials: bool) -> Self {
        Self {
            allowed_origins,
            allow_credentials,
        }
    }

    /// Read policy from environment variables.
    ///
    /// | Variable | Default | Description |
    /// |---|---|---|
    /// | `CORS_ALLOWED_ORIGINS` | `http://localhost:8000,http://localhost:3000` | Comma-separated list of trusted origins. Set to empty string to disable CORS entirely. |
    /// | `CORS_ALLOW_CREDENTIALS` | `false` | Set to `true`/`1`/`yes` to emit `Access-Control-Allow-Credentials: true`. |
    ///
    /// The defaults permit the Rocket dev server and a typical npm dev server.
    /// In production, set `CORS_ALLOWED_ORIGINS` to the exact front-end origin
    /// (or leave it unset/empty if UI and API share the same origin).
    pub fn from_env() -> Self {
        let allowed_origins: Vec<String> = std::env::var("CORS_ALLOWED_ORIGINS")
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_else(|_| {
                vec![
                    "http://localhost:8000".to_string(),
                    "http://localhost:3000".to_string(),
                    "http://localhost:5173".to_string(),
                ]
            });

        let allow_credentials = std::env::var("CORS_ALLOW_CREDENTIALS")
            .ok()
            .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
            .unwrap_or(false);

        Self {
            allowed_origins,
            allow_credentials,
        }
    }

    /// Returns `true` when `origin` is in the allowlist.
    fn is_allowed(&self, origin: &str) -> bool {
        self.allowed_origins.iter().any(|o| o == origin)
    }
}

/// Rocket fairing that enforces the CORS allowlist policy.
///
/// Attach via `CorsFairing(CorsPolicy::from_env())` in `app.rs`.
pub struct CorsFairing(pub CorsPolicy);

#[rocket::async_trait]
impl Fairing for CorsFairing {
    fn info(&self) -> Info {
        Info {
            name: "CORS Fairing",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        // Only act when the browser sends an `Origin` header (cross-origin request).
        let origin = match request.headers().get_one("Origin") {
            Some(o) => o.to_string(),
            None => return, // Same-origin or non-browser request — no CORS headers needed.
        };

        // Reject origins not in the allowlist silently (no CORS headers emitted).
        if !self.0.is_allowed(&origin) {
            return;
        }

        // Echo the exact origin — never a wildcard.
        response.set_header(rocket::http::Header::new(
            "Access-Control-Allow-Origin",
            origin.clone(),
        ));

        // Prevent shared caches from serving the response to a different origin.
        response.adjoin_header(rocket::http::Header::new("Vary", "Origin"));

        if self.0.allow_credentials {
            response.set_header(rocket::http::Header::new(
                "Access-Control-Allow-Credentials",
                "true",
            ));
        }

        // Handle preflight: OPTIONS requests that have not matched a route arrive
        // as 404.  Convert them to 204 and add the required preflight headers.
        if request.method() == Method::Options && response.status() == Status::NotFound {
            response.set_status(Status::NoContent);
            response.set_header(rocket::http::Header::new(
                "Access-Control-Allow-Methods",
                "GET, POST, PUT, PATCH, DELETE, OPTIONS",
            ));
            response.set_header(rocket::http::Header::new(
                "Access-Control-Allow-Headers",
                "Authorization, Content-Type, Accept, X-Requested-With",
            ));
            // Cache the preflight result for 1 hour to reduce round-trips.
            response.set_header(rocket::http::Header::new("Access-Control-Max-Age", "3600"));
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Baseline security-header fairing (ASVS V3.4.1, V3.4.3, V3.4.4).
//!
//! Injects the following headers on **every** response:
//!
//! | Header                      | Value                                        | ASVS     |
//! |-----------------------------|----------------------------------------------|----------|
//! | `Strict-Transport-Security` | `max-age=31536000; includeSubDomains`        | V3.4.1   |
//! | `Content-Security-Policy`   | see [`CSP_VALUE`]                            | V3.4.3   |
//! | `X-Content-Type-Options`    | `nosniff`                                    | V3.4.4   |
//!
//! ## HSTS note
//!
//! HSTS is emitted unconditionally.  In production the application is
//! typically reached through a TLS-terminating reverse proxy; the header
//! is still meaningful for the user-agent because it instructs browsers to
//! upgrade future requests to HTTPS end-to-end.  If the app is ever served
//! directly over plain HTTP the header is harmless and will simply be ignored
//! by the browser.
//!
//! ## Content Security Policy
//!
//! The policy is deliberately explicit about every source type so that any
//! future relaxation is a conscious decision rather than a quiet fallback to
//! `default-src`.
//!
//! `frame-ancestors 'self'` satisfies the anti-clickjacking requirement
//! (ASVS V3.4.3) and supersedes the legacy `X-Frame-Options` header.
//!
//! Inline scripts and styles are permitted (`'unsafe-inline'`) because the
//! current Handlebars templates rely on inline `<script>` blocks and
//! `style=` attributes.  When a nonce-based approach is adopted in the
//! future this constant should be updated accordingly.

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response};

/// `Content-Security-Policy` header value applied to all responses.
///
/// Directives:
/// * `default-src 'self'` – safe fallback for anything not listed below.
/// * `script-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net` – same-origin
///   scripts, inline script blocks (used by current templates), and
///   the Bootstrap CDN bundle.
/// * `style-src 'self' 'unsafe-inline'` – same-origin stylesheets and inline
///   `style=` attributes (used pervasively in templates).
/// * `img-src 'self' data:` – embedded base64 images and same-origin images.
/// * `font-src 'self'` – local webfonts only.
/// * `object-src 'none'` – block plugins (Flash, PDF, etc.).
/// * `base-uri 'self'` – prevent `<base>` injection attacks.
/// * `form-action 'self'` – restrict form submissions to same origin.
/// * `frame-ancestors 'self'` – prevent embedding in third-party frames
///   (ASVS V3.4.3, anti-clickjacking).
pub const CSP_VALUE: &str = "\
default-src 'self'; \
script-src 'self' 'unsafe-inline' https://cdn.jsdelivr.net; \
style-src 'self' 'unsafe-inline'; \
img-src 'self' data:; \
font-src 'self'; \
object-src 'none'; \
base-uri 'self'; \
form-action 'self'; \
frame-ancestors 'self'";

/// Fairing that injects baseline security headers on every response.
pub struct SecurityHeadersFairing;

#[rocket::async_trait]
impl Fairing for SecurityHeadersFairing {
    fn info(&self) -> Info {
        Info {
            name: "Security Headers (ASVS V3.4.1, V3.4.3, V3.4.4)",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        // HSTS – instructs browsers to require HTTPS (ASVS V3.4.1).
        response.set_header(Header::new(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains",
        ));

        // CSP – controls allowed content sources and prevents framing (ASVS V3.4.3).
        response.set_header(Header::new("Content-Security-Policy", CSP_VALUE));

        // Prevent MIME-type sniffing (ASVS V3.4.4).
        response.set_header(Header::new("X-Content-Type-Options", "nosniff"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::Status;
    use rocket::local::blocking::Client;
    use rocket::{get, routes};

    #[get("/")]
    fn index() -> &'static str {
        "hello"
    }

    fn make_client() -> Client {
        let rocket = rocket::build()
            .mount("/", routes![index])
            .attach(SecurityHeadersFairing);
        Client::tracked(rocket).expect("valid rocket")
    }

    #[test]
    fn hsts_header_present() {
        let client = make_client();
        let response = client.get("/").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(
            response.headers().get_one("Strict-Transport-Security"),
            Some("max-age=31536000; includeSubDomains"),
            "HSTS header must be present (ASVS V3.4.1)"
        );
    }

    #[test]
    fn csp_header_present() {
        let client = make_client();
        let response = client.get("/").dispatch();
        let csp = response
            .headers()
            .get_one("Content-Security-Policy")
            .expect("CSP header must be present (ASVS V3.4.3)");
        assert!(
            csp.contains("frame-ancestors 'self'"),
            "CSP must include frame-ancestors 'self' (ASVS V3.4.3)"
        );
        assert!(
            csp.contains("default-src 'self'"),
            "CSP must include default-src 'self'"
        );
        assert!(
            csp.contains("object-src 'none'"),
            "CSP must block plugins via object-src 'none'"
        );
        assert!(
            csp.contains("form-action 'self'"),
            "CSP must restrict form submissions to same origin"
        );
    }

    #[test]
    fn x_content_type_options_nosniff() {
        let client = make_client();
        let response = client.get("/").dispatch();
        assert_eq!(
            response.headers().get_one("X-Content-Type-Options"),
            Some("nosniff"),
            "X-Content-Type-Options: nosniff must be present (ASVS V3.4.4)"
        );
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Anti-caching fairing for authenticated HTML responses (ASVS V14.3.2).
//!
//! Adds `Cache-Control: no-store`, `Pragma: no-cache`, and `Expires: 0`
//! to every HTML response whose URI is **not** under `/static`.
//! This ensures browsers do not cache authenticated pages, preventing
//! sensitive data from being retrieved via the back button or disk cache.

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response};

pub struct AntiCacheFairing;

#[rocket::async_trait]
impl Fairing for AntiCacheFairing {
    fn info(&self) -> Info {
        Info {
            name: "Anti-Cache Headers (ASVS V14.3.2)",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let path = request.uri().path().as_str();

        // Skip static assets – they should be cacheable.
        if path.starts_with("/static") {
            return;
        }

        // Only target HTML responses (templates, redirects already carry
        // text/html or no content-type). Redirects (3xx) are included
        // because the browser may still cache the redirect itself.
        let dominated_by_html = response
            .content_type()
            .map(|ct| ct.is_html())
            .unwrap_or(false);

        // Also apply to redirects (no body / no content-type) that are not
        // API routes, so back-button behaviour is safe.
        let is_redirect = response.status().code >= 300 && response.status().code < 400;
        let is_api = path.starts_with("/api");

        if dominated_by_html || (is_redirect && !is_api) {
            response.set_header(Header::new("Cache-Control", "no-store"));
            response.set_header(Header::new("Pragma", "no-cache"));
            response.set_header(Header::new("Expires", "0"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::Status;
    use rocket::local::blocking::Client;
    use rocket::{get, routes};

    #[get("/health")]
    fn health() -> &'static str {
        "ok"
    }

    #[get("/static/test.css")]
    fn fake_static() -> (rocket::http::ContentType, &'static str) {
        (rocket::http::ContentType::CSS, "body{}")
    }

    #[test]
    fn static_assets_not_tagged() {
        let rocket = rocket::build()
            .mount("/", routes![fake_static])
            .attach(AntiCacheFairing);
        let client = Client::tracked(rocket).expect("valid rocket");
        let response = client.get("/static/test.css").dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert!(response.headers().get_one("Cache-Control").is_none());
    }

    #[test]
    fn plain_text_not_tagged() {
        let rocket = rocket::build()
            .mount("/", routes![health])
            .attach(AntiCacheFairing);
        let client = Client::tracked(rocket).expect("valid rocket");
        let response = client.get("/health").dispatch();
        assert_eq!(response.status(), Status::Ok);
        // plain text endpoint, not HTML → no anti-cache
        assert!(response.headers().get_one("Cache-Control").is_none());
    }
}

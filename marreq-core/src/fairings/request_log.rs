// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Per-request access log.
//!
//! Emits one line per HTTP request to stderr in a structured format suitable
//! for downstream log shippers:
//!
//! ```text
//! [marreq.req] method=POST path=/api/projects status=201 dur_ms=42 ip=127.0.0.1 ua="curl/8.4.0"
//! ```
//!
//! Lines are intentionally key=value so `grep`, `awk`, `jq -R`, and Loki/ELK
//! parsers all work.  No body or header content is logged so the format is
//! safe to ship to a third party.

use std::time::Instant;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Status;
use rocket::{Data, Request, Response};

/// Wrapper stored in [`Request::local_cache`] so the response phase can read
/// the start instant without a global map.
struct RequestStart(Instant);

/// Fairing that logs every request once Rocket has produced a response.
///
/// Attach in `app::build_with`:
///
/// ```ignore
/// rocket = rocket.attach(RequestLogFairing);
/// ```
pub struct RequestLogFairing;

#[rocket::async_trait]
impl Fairing for RequestLogFairing {
    fn info(&self) -> Info {
        Info {
            name: "Request Log",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        // local_cache deduplicates by type, so storing twice (e.g. on a re-route)
        // is harmless; we just keep the original start time.
        req.local_cache(|| RequestStart(Instant::now()));
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        let start = req.local_cache(|| RequestStart(Instant::now())).0;
        let dur_ms = start.elapsed().as_millis();

        let method = req.method();
        let path = req.uri().path();
        let status = res.status();

        let ip = req
            .client_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| "-".into());

        let ua = req
            .headers()
            .get_one("User-Agent")
            .map(escape_ua)
            .unwrap_or_else(|| "-".into());

        let level = log_level_for(status);

        eprintln!(
            "[marreq.req] level={level} method={method} path={path} status={code} dur_ms={dur_ms} ip={ip} ua=\"{ua}\"",
            code = status.code,
        );
    }
}

/// Coarse severity for downstream filtering.
fn log_level_for(status: Status) -> &'static str {
    match status.code {
        500..=599 => "error",
        400..=499 => "warn",
        _ => "info",
    }
}

/// Strip characters that would break the `ua="..."` field. Keeps the format
/// trivially parseable without bringing in JSON for one field.
fn escape_ua(raw: &str) -> String {
    raw.chars()
        .filter(|c| !matches!(c, '"' | '\n' | '\r'))
        .take(200)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_mapping() {
        assert_eq!(log_level_for(Status::Ok), "info");
        assert_eq!(log_level_for(Status::Created), "info");
        assert_eq!(log_level_for(Status::NotFound), "warn");
        assert_eq!(log_level_for(Status::InternalServerError), "error");
        assert_eq!(log_level_for(Status::BadGateway), "error");
    }

    #[test]
    fn escape_ua_strips_quotes_and_newlines() {
        assert_eq!(escape_ua("curl/\"7.0\""), "curl/7.0");
        assert_eq!(escape_ua("a\nb\rc"), "abc");
    }

    #[test]
    fn escape_ua_caps_length() {
        let huge = "x".repeat(500);
        assert_eq!(escape_ua(&huge).len(), 200);
    }
}

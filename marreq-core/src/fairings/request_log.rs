// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Per-request access log.
//!
//! Emits one line per HTTP request to stderr in a structured format suitable
//! for downstream log shippers:
//!
//! ```text
//! [marreq.req] level=info req_id=4a1f… method=POST path=/api/projects \
//!              status=201 dur_ms=42 user=17 ip=127.0.0.1 ua="curl/8.4.0"
//! ```
//!
//! Lines are intentionally key=value so `grep`, `awk`, `jq -R`, and Loki/ELK
//! parsers all work.  No body or header content is logged so the format is
//! safe to ship to a third party.
//!
//! Each request also receives a `X-Request-Id` response header so clients,
//! reverse proxies, and downstream services can correlate logs end to end.

use std::time::Instant;

use rand::RngCore;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{Header, Status};
use rocket::{Data, Request, Response};

/// Wrapper stored in [`Request::local_cache`] so the response phase can read
/// the start instant without a global map.
struct RequestStart(Instant);

/// 16-hex-char request id stashed at request time and echoed back in the
/// `X-Request-Id` response header.
struct RequestId(String);

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
        // local_cache deduplicates by type, so repeated calls (re-route, etc.)
        // keep the original values.
        req.local_cache(|| RequestStart(Instant::now()));
        req.local_cache(|| RequestId(generate_request_id()));
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        let start = req.local_cache(|| RequestStart(Instant::now())).0;
        let dur_ms = start.elapsed().as_millis();
        let req_id = &req.local_cache(|| RequestId(generate_request_id())).0;

        res.set_header(Header::new("X-Request-Id", req_id.clone()));

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

        let user = crate::auth::session::read_session_user_id(req.cookies())
            .map(|id| id.to_string())
            .unwrap_or_else(|| "-".into());

        let level = log_level_for(status);

        eprintln!(
            "[marreq.req] level={level} req_id={req_id} method={method} path={path} status={code} dur_ms={dur_ms} user={user} ip={ip} ua=\"{ua}\"",
            code = status.code,
        );
    }
}

/// 16 lowercase hex chars (64 bits of randomness). Good enough to correlate
/// requests in logs without pulling in the `uuid` crate.
fn generate_request_id() -> String {
    let mut buf = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut buf);
    let mut s = String::with_capacity(16);
    for b in buf {
        s.push_str(&format!("{b:02x}"));
    }
    s
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

    #[test]
    fn request_id_is_hex_and_correct_length() {
        let id = generate_request_id();
        assert_eq!(id.len(), 16);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn request_id_is_random() {
        let a = generate_request_id();
        let b = generate_request_id();
        assert_ne!(a, b, "two consecutive ids should differ");
    }
}

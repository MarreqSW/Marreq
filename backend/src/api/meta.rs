// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Stable JSON entrypoint at `GET /api` (mount base + `/`).

use rocket::response::content::RawJson;

const API_ROOT_JSON: &str = r#"{"service":"marreq","api":"/api","hint":"Open the SPA on port 8080 when using Docker Compose (see docker/README.md)."}"#;

/// `GET /api` — always available; useful when `GET /` is ambiguous or behind proxies.
#[get("/")]
pub fn api_root() -> RawJson<&'static str> {
    RawJson(API_ROOT_JSON)
}

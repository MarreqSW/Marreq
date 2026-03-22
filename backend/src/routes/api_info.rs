// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Root `GET /` — JSON service descriptor (UI is the SPA).

use rocket::response::content::RawJson;

const ROOT_JSON: &str = r#"{"service":"marreq","api":"/api","message":"JSON API — use the SPA for the web UI (e.g. http://127.0.0.1:8080 with Docker Compose)."}"#;

#[get("/")]
pub fn root_index() -> RawJson<&'static str> {
    RawJson(ROOT_JSON)
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Minimal root response when `MARREQ_UI_MODE=api_only` (no HTML shell).

use rocket::response::content::RawJson;

const API_ONLY_ROOT_JSON: &str = r#"{"service":"marreq","mode":"api_only","api":"/api","message":"JSON API only — use the SPA for the web UI (e.g. http://127.0.0.1:8080 with Docker Compose)."}"#;

#[get("/")]
pub fn api_only_index() -> RawJson<&'static str> {
    RawJson(API_ONLY_ROOT_JSON)
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Stable JSON entrypoint at `GET /api` (mount base + `/`).

use rocket::response::content::RawJson;
use rocket::serde::json::{json, Json};

const API_ROOT_JSON: &str = r#"{"service":"marreq","api":"/api","hint":"Open the SPA on port 8080 when using Docker Compose (see docker/README.md)."}"#;

/// `GET /api` — always available; useful when `GET /` is ambiguous or behind proxies.
#[get("/")]
pub fn api_root() -> RawJson<&'static str> {
    RawJson(API_ROOT_JSON)
}

/// `GET /api/meta/health` — liveness probe.
///
/// Always returns 200 as long as the Rocket process can serve requests.
/// No DB or cache access on purpose: this is a *liveness* signal, not
/// readiness. Used by Docker `HEALTHCHECK`, Kubernetes liveness probes,
/// and external uptime monitors.
#[get("/meta/health")]
pub fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "marreq",
    }))
}

/// `GET /api/meta/deployment` — exposes deployment-mode capabilities so the
/// SPA can render or hide registration/admin UI accordingly.
#[get("/meta/deployment")]
pub fn deployment_info() -> Json<serde_json::Value> {
    let mode = crate::deployment::current();
    Json(json!({
        "mode": mode.name(),
        "allows_self_registration": mode.allows_self_registration(),
        "requires_email_verification": mode.requires_email_verification(),
        "allows_admin_promotion": mode.allows_admin_promotion(),
        "assigns_personal_workspace": mode.assigns_personal_workspace(),
        "allows_self_administered_user_creation": mode.allows_self_administered_user_creation(),
    }))
}

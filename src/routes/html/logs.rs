// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

#![allow(clippy::result_large_err)]

use super::prelude::*;
use crate::services::LogService;
use std::path::Path;

const DEFAULT_EXPORT_LIMIT: i64 = 1000;
const CLEANUP_DAYS: i64 = 90;

#[get("/logs")]
pub fn show_logs(admin: AdminOnly, state: &State<AppState>) -> Result<Template, Redirect> {
    let user = admin.into_inner();
    let service = LogService::new(state.inner());

    let logs = service.recent_logs(DEFAULT_EXPORT_LIMIT).map_err(|err| {
        eprintln!("Failed to load logs: {err}");
        Redirect::to(uri!(crate::routes::html::admin::admin_dashboard))
    })?;

    let ctx = json!({
        "user": user,
        "logs": logs,
        "page_title": "System Logs",
    });

    Ok(Template::render("logs", ctx))
}

#[get("/logs/<entity_type>/<entity_id>")]
pub fn show_entity_logs(
    admin: AdminOnly,
    entity_type: String,
    entity_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = admin.into_inner();
    let service = LogService::new(state.inner());

    let logs = service
        .entity_logs(&entity_type, entity_id)
        .map_err(|err| {
            eprintln!("Failed to load logs for {entity_type} {entity_id}: {err}");
            Redirect::to(uri!(show_logs))
        })?;

    let ctx = json!({
        "user": user,
        "logs": logs,
        "entity_type": entity_type,
        "entity_id": entity_id,
        "page_title": format!("Logs for {} {}", entity_type, entity_id),
    });

    Ok(Template::render("entity_logs", ctx))
}

#[get("/export_logs?<filename>")]
pub async fn export_logs(
    admin: AdminOnly,
    filename: Option<String>,
    state: &State<AppState>,
) -> Result<(ContentType, NamedFile), Redirect> {
    let user = admin.into_inner();
    let service = LogService::new(state.inner());

    let logs = service
        .recent_logs_raw(DEFAULT_EXPORT_LIMIT)
        .map_err(|err| {
            eprintln!("Failed to fetch logs for export: {err}");
            Redirect::to(uri!(show_logs))
        })?;
    let logs_json = service.logs_to_json(&logs).map_err(|err| {
        eprintln!("Failed to serialize logs for export: {err}");
        Redirect::to(uri!(show_logs))
    })?;

    let filename = filename.unwrap_or_else(|| {
        let now = chrono::Utc::now();
        format!("reqman-logs_{}.json", now.format("%Y%m%d_%H%M%S"))
    });
    let filename = ensure_json_extension(&filename);

    let export_dir = "exports";
    if !Path::new(export_dir).exists() {
        std::fs::create_dir(export_dir).map_err(|_| Redirect::to(uri!(show_logs)))?;
    }

    let export_path = format!("{}/{}", export_dir, filename);
    std::fs::write(&export_path, logs_json).map_err(|_| Redirect::to(uri!(show_logs)))?;

    if let Err(err) =
        service.log_export_action(user.id, Some(format!("Exported logs to {filename}")))
    {
        eprintln!("Failed to log export action: {err}");
    }

    let file = NamedFile::open(&export_path)
        .await
        .map_err(|_| Redirect::to(uri!(show_logs)))?;

    Ok((ContentType::JSON, file))
}

#[get("/export_logs/<entity_type>/<entity_id>")]
pub fn export_entity_logs(
    _admin: AdminOnly,
    entity_type: String,
    entity_id: i32,
    state: &State<AppState>,
) -> Result<(rocket::http::ContentType, String), Redirect> {
    let service = LogService::new(state.inner());

    let logs = service
        .entity_logs_raw(&entity_type, entity_id)
        .map_err(|err| {
            eprintln!("Failed to fetch logs for entity export {entity_type} {entity_id}: {err}");
            Redirect::to(uri!(show_logs))
        })?;
    let logs_json = service.logs_to_json(&logs).map_err(|err| {
        eprintln!("Failed to serialize entity logs for export: {err}");
        Redirect::to(uri!(show_logs))
    })?;

    let content_type = rocket::http::ContentType::new("application", "json");
    Ok((content_type, logs_json))
}

#[post("/cleanup_logs")]
pub fn cleanup_logs(admin: AdminOnly, state: &State<AppState>) -> Result<Redirect, Redirect> {
    let user = admin.into_inner();
    let service = LogService::new(state.inner());

    match service.cleanup_old_logs(user.id, CLEANUP_DAYS) {
        Ok(_) => Ok(Redirect::to(uri!(show_logs))),
        Err(err) => {
            eprintln!("Failed to clean up old logs: {err}");
            Err(Redirect::to(uri!(show_logs)))
        }
    }
}

#[get("/log_analytics")]
pub fn log_analytics(admin: AdminOnly, state: &State<AppState>) -> Result<Template, Redirect> {
    let user = admin.into_inner();
    let service = LogService::new(state.inner());

    let analytics = service.analytics().map_err(|err| {
        eprintln!("Failed to load log analytics: {err}");
        Redirect::to(uri!(show_logs))
    })?;

    let ctx = json!({
        "user": user,
        "last_7_days": analytics.last_7_days,
        "last_30_days": analytics.last_30_days,
        "last_90_days": analytics.last_90_days,
        "page_title": "Log Analytics",
    });

    Ok(Template::render("log_analytics", ctx))
}

fn ensure_json_extension(name: &str) -> String {
    if name.ends_with(".json") {
        name.to_owned()
    } else {
        format!("{name}.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_json_extension_adds_when_missing() {
        assert_eq!(ensure_json_extension("logs"), "logs.json");
        assert_eq!(
            ensure_json_extension("reqman-logs_20240101"),
            "reqman-logs_20240101.json"
        );
    }

    #[test]
    fn ensure_json_extension_preserves_when_present() {
        assert_eq!(ensure_json_extension("a.json"), "a.json");
        assert_eq!(ensure_json_extension("file.json"), "file.json");
    }
}

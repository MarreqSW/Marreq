// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Instance-wide audit log browser (admin only).

use chrono::NaiveDateTime;
use rocket::Responder;
use rocket::http::Header;
use rocket::serde::{Deserialize, Serialize};

use crate::api::prelude::*;
use crate::repository::LogListQuery;
use crate::services::log_service::{
    ChangeDetail, LogService, LogStatsResponse, change_summary, log_change_details,
};

const LIST_LIMIT_DEFAULT: i64 = 50;
const LIST_LIMIT_CAP: i64 = 100;
const EXPORT_LIMIT_DEFAULT: i64 = 10_000;
const EXPORT_LIMIT_CAP: i64 = 10_000;
const STATS_DEFAULT_DAYS: i64 = 30;
const STATS_MAX_DAYS: i64 = 366;
const STATS_TOP_DEFAULT: i64 = 10;
const STATS_TOP_CAP: i64 = 50;

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct AdminLogItem {
    pub log_id: i32,
    pub user_id: i32,
    pub username: String,
    pub action_type: String,
    pub summary: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub changes: Vec<ChangeDetail>,
    pub entity_type: String,
    pub entity_id: Option<i32>,
    pub project_id: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct AdminLogListResponse {
    pub items: Vec<AdminLogItem>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(FromForm, Default)]
pub struct LogsQueryParams {
    entity_type: Option<String>,
    entity_id: Option<i32>,
    user_id: Option<i32>,
    action_type: Option<String>,
    project_id: Option<i32>,
    since: Option<String>,
    until: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Filters for `GET /api/admin/logs/stats` (the list filters minus paging, plus `top`).
#[derive(FromForm, Default)]
pub struct LogStatsParams {
    entity_type: Option<String>,
    user_id: Option<i32>,
    action_type: Option<String>,
    project_id: Option<i32>,
    since: Option<String>,
    until: Option<String>,
    top: Option<i64>,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CleanupBody {
    days: i64,
}

#[derive(Responder)]
#[response(status = 200)]
pub struct JsonDownload {
    bytes: Vec<u8>,
    content_type: Header<'static>,
    disposition: Header<'static>,
}

impl JsonDownload {
    fn attachment(bytes: Vec<u8>, filename: &'static str) -> Self {
        Self {
            bytes,
            content_type: Header::new("Content-Type", "application/json"),
            disposition: Header::new(
                "Content-Disposition",
                format!("attachment; filename=\"{filename}\""),
            ),
        }
    }
}

fn blank_to_none(s: Option<String>) -> Option<String> {
    s.and_then(|v| {
        let t = v.trim().to_string();
        if t.is_empty() { None } else { Some(t) }
    })
}

fn parse_datetime(raw: &str) -> Result<NaiveDateTime, ApiError> {
    let s = raw.trim();
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(dt.naive_utc());
    }
    const FORMATS: &[&str] = &[
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d",
    ];
    for fmt in FORMATS {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Ok(dt);
        }
        if *fmt == "%Y-%m-%d"
            && let Ok(d) = chrono::NaiveDate::parse_from_str(s, fmt)
        {
            return Ok(d.and_hms_opt(0, 0, 0).unwrap());
        }
    }
    Err(ApiError::BadRequest(format!(
        "invalid datetime '{s}'; use RFC 3339 or YYYY-MM-DD[THH:MM]"
    )))
}

fn clamp_limit(raw: Option<i64>, default: i64, cap: i64) -> i64 {
    raw.unwrap_or(default).clamp(1, cap)
}

fn to_list_query(q: &LogsQueryParams, default: i64, cap: i64) -> Result<LogListQuery, ApiError> {
    let since = match blank_to_none(q.since.clone()) {
        Some(s) => Some(parse_datetime(&s)?),
        None => None,
    };
    let until = match blank_to_none(q.until.clone()) {
        Some(s) => Some(parse_datetime(&s)?),
        None => None,
    };
    Ok(LogListQuery {
        entity_type: blank_to_none(q.entity_type.clone()),
        entity_id: q.entity_id,
        user_id: q.user_id,
        action_type: blank_to_none(q.action_type.clone()),
        project_id: q.project_id,
        since,
        until,
        limit: clamp_limit(q.limit, default, cap),
        offset: q.offset.unwrap_or(0).max(0),
    })
}

fn map_items(logs: Vec<crate::services::log_service::LogWithUser>) -> Vec<AdminLogItem> {
    logs.into_iter()
        .map(|lw| AdminLogItem {
            log_id: lw.log.log_id,
            user_id: lw.log.user_id,
            username: lw.username,
            action_type: lw.log.action_type.clone(),
            summary: change_summary(&lw.log),
            description: lw.log.description.clone(),
            created_at: lw.log.created_at,
            changes: log_change_details(&lw.log),
            entity_type: lw.log.entity_type.clone(),
            entity_id: lw.log.entity_id,
            project_id: lw.log.project_id,
        })
        .collect()
}

/// `GET /api/admin/logs` — paginated audit log list (administrators only).
#[get("/admin/logs?<q..>")]
pub async fn list(
    _admin: AdminOnly,
    state: &State<AppState>,
    q: LogsQueryParams,
) -> ApiResult<Json<AdminLogListResponse>> {
    let query = to_list_query(&q, LIST_LIMIT_DEFAULT, LIST_LIMIT_CAP)?;
    let limit = query.limit;
    let offset = query.offset;
    let service = LogService::new(state.inner());
    let (logs, total) = service
        .list_filtered(&query)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(AdminLogListResponse {
        items: map_items(logs),
        total,
        limit,
        offset,
    }))
}

/// `GET /api/admin/logs/stats` — activity summary over the audit log (administrators only).
///
/// Defaults to the last 30 UTC days ending now. Returns the total, a zero-filled
/// per-day series, and the top action types and users (`top`, default 10, max 50).
#[get("/admin/logs/stats?<q..>")]
pub async fn stats(
    _admin: AdminOnly,
    state: &State<AppState>,
    q: LogStatsParams,
) -> ApiResult<Json<LogStatsResponse>> {
    let until = match blank_to_none(q.until.clone()) {
        Some(s) => parse_datetime(&s)?,
        None => chrono::Utc::now().naive_utc(),
    };
    let since = match blank_to_none(q.since.clone()) {
        Some(s) => parse_datetime(&s)?,
        None => (until.date() - chrono::Duration::days(STATS_DEFAULT_DAYS - 1))
            .and_hms_opt(0, 0, 0)
            .expect("midnight is valid"),
    };
    if since > until {
        return Err(ApiError::BadRequest("since must not be after until".into()));
    }
    if until - since > chrono::Duration::days(STATS_MAX_DAYS) {
        return Err(ApiError::BadRequest(format!(
            "range too long; at most {STATS_MAX_DAYS} days"
        )));
    }

    let filters = LogListQuery {
        entity_type: blank_to_none(q.entity_type.clone()),
        user_id: q.user_id,
        action_type: blank_to_none(q.action_type.clone()),
        project_id: q.project_id,
        ..Default::default()
    };
    let top = clamp_limit(q.top, STATS_TOP_DEFAULT, STATS_TOP_CAP);
    let body = LogService::new(state.inner())
        .stats(filters, since, until, top)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(body))
}

/// `GET /api/admin/logs/export.json` — JSON download of matching logs (administrators only).
#[get("/admin/logs/export.json?<q..>")]
pub async fn export_json(
    admin: AdminOnly,
    state: &State<AppState>,
    q: LogsQueryParams,
) -> ApiResult<JsonDownload> {
    let query = to_list_query(&q, EXPORT_LIMIT_DEFAULT, EXPORT_LIMIT_CAP)?;
    let limit = query.limit;
    let offset = query.offset;
    let service = LogService::new(state.inner());
    let (logs, total) = service
        .list_filtered(&query)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    let body = AdminLogListResponse {
        items: map_items(logs),
        total,
        limit,
        offset,
    };
    let pretty = serde_json::to_string_pretty(&serde_json::json!({
        "items": body.items.iter().map(|it| serde_json::json!({
            "log_id": it.log_id,
            "user_id": it.user_id,
            "username": it.username,
            "action_type": it.action_type,
            "summary": it.summary,
            "description": it.description,
            "created_at": it.created_at.format("%Y-%m-%dT%H:%M:%S").to_string(),
            "changes": it.changes.iter().map(|c| serde_json::json!({
                "field": c.field,
                "old_value": c.old_value,
                "new_value": c.new_value,
            })).collect::<Vec<_>>(),
            "entity_type": it.entity_type,
            "entity_id": it.entity_id,
            "project_id": it.project_id,
        })).collect::<Vec<_>>(),
        "total": body.total,
        "limit": body.limit,
        "offset": body.offset,
    }))
    .map_err(|e| ApiError::Internal(e.to_string()))?;
    service
        .log_export_action(
            admin.id,
            Some(format!("Exported {total} audit log entries as JSON")),
        )
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(JsonDownload::attachment(
        pretty.into_bytes(),
        "audit-logs.json",
    ))
}

/// `POST /api/admin/logs/cleanup` — delete logs older than `days` (administrators only).
#[post("/admin/logs/cleanup", data = "<payload>")]
pub async fn cleanup(
    admin: AdminOnly,
    state: &State<AppState>,
    payload: Json<CleanupBody>,
) -> ApiResult<Value> {
    let days = payload.days;
    if days < 1 {
        return Err(ApiError::BadRequest("days must be at least 1".into()));
    }
    let service = LogService::new(state.inner());
    let deleted = service
        .cleanup_old_logs(admin.id, days)
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(json!({ "deleted": deleted }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::test_session_cookie_for;
    use crate::models::Log;
    use crate::repository::{CacheRepository, diesel_repo_mock::DieselRepoMock};
    use chrono::NaiveDate;
    use rocket::http::{ContentType, Cookie};
    use rocket::local::asynchronous::Client;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;
    const NON_ADMIN_ID: i32 = 2;

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 6, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    fn sample_log(log_id: i32, user_id: i32, entity_type: &str, action: &str) -> Log {
        Log {
            log_id,
            user_id,
            action_type: action.into(),
            entity_type: entity_type.into(),
            entity_id: Some(10),
            project_id: Some(1),
            old_values: None,
            new_values: None,
            description: Some(format!("{action} {entity_type}")),
            ip_address: None,
            user_agent: None,
            created_at: timestamp(),
        }
    }

    fn repo_with_users_and_logs() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default().with_admin_user();
        let non_admin = DieselRepoMock::make_user(NON_ADMIN_ID, "bob", "hash");
        repo.users.insert(NON_ADMIN_ID, non_admin);
        repo.logs
            .push(sample_log(1, ADMIN_ID, "REQUIREMENT", "CREATE"));
        repo.logs
            .push(sample_log(2, NON_ADMIN_ID, "VERIFICATION", "UPDATE"));
        repo
    }

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(state_from_repo(repo))
            .mount("/api", routes![list, export_json, cleanup, stats]);
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_cookie(client: &Client, user_id: i32) -> Cookie<'static> {
        let state = client.rocket().state::<TestState>().unwrap();
        test_session_cookie_for(state, user_id)
    }

    #[rocket::async_test]
    async fn list_forbidden_for_non_admin() {
        let client = client_with_repo(repo_with_users_and_logs()).await;
        let response = client
            .get("/api/admin/logs")
            .private_cookie(auth_cookie(&client, NON_ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn export_forbidden_for_non_admin() {
        let client = client_with_repo(repo_with_users_and_logs()).await;
        let response = client
            .get("/api/admin/logs/export.json")
            .private_cookie(auth_cookie(&client, NON_ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn cleanup_forbidden_for_non_admin() {
        let client = client_with_repo(repo_with_users_and_logs()).await;
        let response = client
            .post("/api/admin/logs/cleanup")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client, NON_ADMIN_ID))
            .body(r#"{"days":90}"#)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn list_returns_items_and_total_for_admin() {
        let client = client_with_repo(repo_with_users_and_logs()).await;
        let response = client
            .get("/api/admin/logs")
            .private_cookie(auth_cookie(&client, ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: serde_json::Value = response.into_json().await.unwrap();
        assert_eq!(body["total"], 2);
        assert_eq!(body["items"].as_array().unwrap().len(), 2);
        assert!(body["items"][0]["entity_type"].is_string());
        assert!(body["items"][0]["summary"].is_string());
    }

    #[rocket::async_test]
    async fn list_filters_by_entity_type() {
        let client = client_with_repo(repo_with_users_and_logs()).await;
        let response = client
            .get("/api/admin/logs?entity_type=REQUIREMENT")
            .private_cookie(auth_cookie(&client, ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: serde_json::Value = response.into_json().await.unwrap();
        assert_eq!(body["total"], 1);
        assert_eq!(body["items"][0]["entity_type"], "REQUIREMENT");
    }

    #[rocket::async_test]
    async fn cleanup_rejects_days_below_one() {
        let client = client_with_repo(repo_with_users_and_logs()).await;
        let response = client
            .post("/api/admin/logs/cleanup")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client, ADMIN_ID))
            .body(r#"{"days":0}"#)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    async fn cleanup_deletes_old_logs_for_admin() {
        let mut repo = repo_with_users_and_logs();
        for log in &mut repo.logs {
            log.created_at = NaiveDate::from_ymd_opt(1999, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
        }
        let client = client_with_repo(repo).await;
        let response = client
            .post("/api/admin/logs/cleanup")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client, ADMIN_ID))
            .body(r#"{"days":90}"#)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: serde_json::Value = response.into_json().await.unwrap();
        assert_eq!(body["deleted"], 2);
    }

    fn at(day: u32, hour: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 9, day)
            .unwrap()
            .and_hms_opt(hour, 0, 0)
            .unwrap()
    }

    fn repo_with_activity() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default().with_admin_user();
        repo.users.insert(
            NON_ADMIN_ID,
            DieselRepoMock::make_user(NON_ADMIN_ID, "bob", "hash"),
        );
        let rows = [
            (1, 9, ADMIN_ID, "UPDATE", Some(1)),
            (1, 23, ADMIN_ID, "UPDATE", Some(1)),
            (2, 8, NON_ADMIN_ID, "CREATE", Some(2)),
            (4, 10, ADMIN_ID, "LOGIN", None),
            (4, 11, NON_ADMIN_ID, "UPDATE", Some(1)),
        ];
        for (i, (day, hour, user, action, project)) in rows.into_iter().enumerate() {
            let mut log = sample_log(i as i32 + 1, user, "REQUIREMENT", action);
            log.created_at = at(day, hour);
            log.project_id = project;
            repo.logs.push(log);
        }
        repo
    }

    async fn get_stats<'c>(
        client: &'c Client,
        query: &str,
    ) -> rocket::local::asynchronous::LocalResponse<'c> {
        client
            .get(format!("/api/admin/logs/stats?{query}"))
            .private_cookie(auth_cookie(client, ADMIN_ID))
            .dispatch()
            .await
    }

    #[rocket::async_test]
    async fn stats_summarises_activity() {
        let client = client_with_repo(repo_with_activity()).await;
        let response = get_stats(&client, "since=2026-09-01&until=2026-09-04T23:59:59").await;
        assert_eq!(response.status(), Status::Ok);
        let body: serde_json::Value = response.into_json().await.unwrap();

        assert_eq!(body["total"], 5);
        let per_day: Vec<i64> = body["by_day"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["count"].as_i64().unwrap())
            .collect();
        assert_eq!(per_day, vec![2, 1, 0, 2]);
        assert_eq!(body["by_day"][0]["day"], "2026-09-01");
        assert_eq!(body["by_action"][0]["action_type"], "UPDATE");
        assert_eq!(body["by_action"][0]["count"], 3);
        assert_eq!(body["by_user"][0]["username"], "admin");
        assert_eq!(body["by_user"][0]["count"], 3);
        assert_eq!(body["by_user"][1]["username"], "bob");
    }

    #[rocket::async_test]
    async fn stats_applies_filters_and_top() {
        let client = client_with_repo(repo_with_activity()).await;
        let response = get_stats(
            &client,
            "since=2026-09-01&until=2026-09-04T23:59:59&project_id=1&top=1",
        )
        .await;
        let body: serde_json::Value = response.into_json().await.unwrap();
        assert_eq!(body["total"], 3);
        assert_eq!(body["active_users"], 2, "not capped by top");
        assert_eq!(body["by_action"].as_array().unwrap().len(), 1);
        assert_eq!(body["by_user"].as_array().unwrap().len(), 1);

        let response = get_stats(
            &client,
            "since=2026-09-01&until=2026-09-04T23:59:59&action_type=CREATE",
        )
        .await;
        let body: serde_json::Value = response.into_json().await.unwrap();
        assert_eq!(body["total"], 1);
        assert_eq!(body["by_user"][0]["username"], "bob");
    }

    #[rocket::async_test]
    async fn stats_defaults_to_last_30_days() {
        let client = client_with_repo(repo_with_activity()).await;
        let response = get_stats(&client, "").await;
        assert_eq!(response.status(), Status::Ok);
        let body: serde_json::Value = response.into_json().await.unwrap();
        assert_eq!(body["by_day"].as_array().unwrap().len(), 30);
    }

    #[rocket::async_test]
    async fn stats_rejects_bad_ranges() {
        let client = client_with_repo(repo_with_activity()).await;
        let reversed = get_stats(&client, "since=2026-09-04&until=2026-09-01").await;
        assert_eq!(reversed.status(), Status::BadRequest);
        let too_long = get_stats(&client, "since=2024-01-01&until=2026-01-01").await;
        assert_eq!(too_long.status(), Status::BadRequest);
        let garbage = get_stats(&client, "since=yesterday").await;
        assert_eq!(garbage.status(), Status::BadRequest);
    }

    #[rocket::async_test]
    async fn stats_requires_admin() {
        let client = client_with_repo(repo_with_activity()).await;
        let anonymous = client.get("/api/admin/logs/stats").dispatch().await;
        assert_eq!(anonymous.status(), Status::Unauthorized);
        let non_admin = client
            .get("/api/admin/logs/stats")
            .private_cookie(auth_cookie(&client, NON_ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(non_admin.status(), Status::Forbidden);
    }
}

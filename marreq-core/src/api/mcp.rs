// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! MCP (Model Context Protocol) API: audit logging for tool calls.

use rocket::serde::{Deserialize, Serialize};

use crate::api::prelude::*;
use crate::auth::guards::ApiUserOrBearer;
use crate::models::forms::NewLog;
use crate::repository::LogRepository;

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct McpAuditRequest {
    pub project_id: i32,
    pub session_id: Option<String>,
    pub tool_name: String,
    pub params_summary: Option<String>,
    pub result_summary: Option<String>,
    pub is_write: bool,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct McpAuditResponse {
    pub status: &'static str,
}

/// Record an MCP tool call for audit. Requires auth (session or Bearer).
/// Logged to the same logs table with entity_type "MCP", action_type "MCP_TOOL".
#[post("/mcp/audit", data = "<body>")]
pub async fn audit(
    user: ApiUserOrBearer,
    state: &State<AppState>,
    body: Json<McpAuditRequest>,
) -> ApiResult<Json<McpAuditResponse>> {
    let payload = body.into_inner();
    let user_id = user.user().id;
    let description = serde_json::json!({
        "tool": payload.tool_name,
        "params_summary": payload.params_summary,
        "result_summary": payload.result_summary,
        "is_write": payload.is_write,
        "session_id": payload.session_id,
    })
    .to_string();

    let new_log = NewLog {
        user_id,
        action_type: "MCP_TOOL".to_string(),
        entity_type: "MCP".to_string(),
        entity_id: None,
        project_id: Some(payload.project_id),
        old_values: None,
        new_values: None,
        description: Some(description),
        ip_address: user.log_ctx().ip_address().map(str::to_string),
        user_agent: user.log_ctx().user_agent().map(str::to_string),
    };

    state
        .repo_write()
        .insert_log(&new_log)
        .map_err(ApiError::from)?;
    Ok(Json(McpAuditResponse { status: "ok" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::test_session_cookie_for;

    fn auth_cookie_for(
        client: &rocket::local::asynchronous::Client,
        user_id: i32,
    ) -> rocket::http::Cookie<'static> {
        let state = client.rocket().state::<TestState>().unwrap();
        test_session_cookie_for(state, user_id)
    }
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(state_from_repo(repo))
            .mount("/api", routes![audit]);
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_cookie(client: &rocket::local::asynchronous::Client) -> rocket::http::Cookie<'static> {
        auth_cookie_for(client, ADMIN_ID)
    }

    #[rocket::async_test]
    async fn audit_returns_ok_when_authenticated() {
        let client = client_with_repo(DieselRepoMock::default().with_admin_user()).await;
        let response = client
            .post("/api/mcp/audit")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(
                r#"{
                "project_id": 1,
                "session_id": "sess-1",
                "tool_name": "test_tool",
                "params_summary": "a=1",
                "result_summary": "ok",
                "is_write": false
            }"#,
            )
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: serde_json::Value = response.into_json().await.unwrap();
        assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("ok"));
    }

    #[rocket::async_test]
    async fn audit_requires_auth() {
        let client = client_with_repo(DieselRepoMock::default().with_admin_user()).await;
        let response = client
            .post("/api/mcp/audit")
            .header(ContentType::JSON)
            .body(
                r#"{
                "project_id": 1,
                "tool_name": "test_tool",
                "is_write": false
            }"#,
            )
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Unauthorized);
    }

    #[test]
    fn mcp_audit_request_deserialize() {
        let json = r#"{"project_id":1,"tool_name":"x","is_write":true}"#;
        let req: McpAuditRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.project_id, 1);
        assert_eq!(req.tool_name, "x");
        assert!(req.is_write);
        assert!(req.session_id.is_none());
        assert!(req.params_summary.is_none());
        assert!(req.result_summary.is_none());
    }

    #[test]
    fn mcp_audit_response_serialize() {
        let res = McpAuditResponse { status: "ok" };
        let json = serde_json::to_string(&res).unwrap();
        assert_eq!(json, r#"{"status":"ok"}"#);
    }
}

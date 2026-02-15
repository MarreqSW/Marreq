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

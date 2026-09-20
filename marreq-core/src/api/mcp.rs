// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! MCP (Model Context Protocol) API: audit logging for tool calls.

use rocket::request::{FromRequest, Outcome};
use rocket::serde::{Deserialize, Serialize};
use rocket::{async_trait, Request};

use crate::api::prelude::*;
use crate::auth::guards::session::session_user_has_project_access;
use crate::auth::guards::{AuthenticationSource, McpAuditAuth, McpPrincipalAuth};
use crate::models::forms::NewLog;
use crate::repository::LogRepository;

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct McpAuditRequest {
    pub project_id: Option<i32>,
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

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct McpPrincipalResponse {
    pub user_id: i32,
    pub authentication_type: &'static str,
    pub principal_id: String,
    pub client_id: Option<String>,
    pub grant_id: Option<i32>,
}

/// Return the stable identity represented by a credential without requiring a
/// project/domain scope. The opaque principal id lets the MCP transport accept
/// rotated OAuth access tokens only when they belong to the same grant.
#[get("/mcp/principal")]
pub fn principal(user: McpPrincipalAuth) -> Json<McpPrincipalResponse> {
    let (authentication_type, principal_id, client_id, grant_id) = match user.source() {
        AuthenticationSource::Session => {
            ("session", format!("user:{}", user.user().id), None, None)
        }
        AuthenticationSource::ApiToken { token_hash, .. } => {
            ("api_token", format!("token:{token_hash}"), None, None)
        }
        AuthenticationSource::DelegatedOAuth {
            client_id,
            grant_id,
            ..
        } => (
            "delegated_oauth",
            format!("grant:{grant_id}"),
            Some(client_id.clone()),
            Some(*grant_id),
        ),
    };
    Json(McpPrincipalResponse {
        user_id: user.user().id,
        authentication_type,
        principal_id,
        client_id,
        grant_id,
    })
}

const MCP_TOOL_NAMES: &[&str] = &[
    "list_projects",
    "get_requirement",
    "list_requirements",
    "get_versions",
    "semantic_search_requirements",
    "compare_versions",
    "trace_up",
    "trace_down",
    "coverage_report",
    "get_baseline",
    "diff_baselines",
    "list_verifications",
    "get_verification",
    "list_baselines",
    "get_requirement_activity",
    "get_verification_activity",
    "list_requirement_comments",
    "get_verification_matrix",
    "list_project_catalog",
    "diff_baseline_vs_current",
    "create_requirement",
    "patch_requirement",
    "set_approval",
    "create_baseline",
    "create_requirement_comment",
    "create_verification",
    "update_verification",
    "put_verification_matrix",
    "clear_suspect",
];

pub struct InternalMcpAudit;

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

#[async_trait]
impl<'r> FromRequest<'r> for InternalMcpAudit {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let expected = std::env::var("MARREQ_MCP_AUDIT_SECRET").ok();
        let supplied = request.headers().get_one("X-Marreq-MCP-Audit-Secret");
        if expected
            .as_deref()
            .filter(|value| value.len() >= 32)
            .zip(supplied)
            .is_some_and(|(expected, supplied)| {
                constant_time_eq(expected.as_bytes(), supplied.as_bytes())
            })
        {
            Outcome::Success(Self)
        } else {
            Outcome::Error((Status::Forbidden, ()))
        }
    }
}

/// Record an MCP tool call for audit. Requires auth (session or Bearer).
/// Logged to the same logs table with entity_type "MCP", action_type "MCP_TOOL".
#[post("/mcp/audit", data = "<body>")]
pub async fn audit(
    _internal: InternalMcpAudit,
    user: McpAuditAuth,
    state: &State<AppState>,
    body: Json<McpAuditRequest>,
) -> ApiResult<Json<McpAuditResponse>> {
    let payload = body.into_inner();
    if !MCP_TOOL_NAMES.contains(&payload.tool_name.as_str())
        || payload.tool_name.len() > 100
        || payload
            .params_summary
            .as_ref()
            .is_some_and(|v| v.len() > 2_000)
        || payload
            .result_summary
            .as_ref()
            .is_some_and(|v| v.len() > 2_000)
    {
        return Err(ApiError::BadRequest("audit payload is too large".into()));
    }
    if matches!(
        user.source(),
        AuthenticationSource::ApiToken {
            project_scope: Some(scope), ..
        } if payload.project_id.is_some_and(|project_id| *scope != project_id)
    ) {
        return Err(ApiError::Forbidden("project access denied".into()));
    }
    if let Some(project_id) = payload.project_id {
        if !session_user_has_project_access(state, user.user(), project_id)
            .map_err(|_| ApiError::Internal("repository unavailable".into()))?
        {
            return Err(ApiError::Forbidden("project access denied".into()));
        }
    }
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
        project_id: payload.project_id,
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
    const AUDIT_SECRET: &str = "test-only-mcp-audit-secret-32-bytes-long";

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        std::env::set_var("MARREQ_MCP_AUDIT_SECRET", AUDIT_SECRET);
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
            .header(rocket::http::Header::new(
                "X-Marreq-MCP-Audit-Secret",
                AUDIT_SECRET,
            ))
            .private_cookie(auth_cookie(&client))
            .body(
                r#"{
                "project_id": 1,
                "session_id": "sess-1",
                "tool_name": "list_projects",
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
            .header(rocket::http::Header::new(
                "X-Marreq-MCP-Audit-Secret",
                AUDIT_SECRET,
            ))
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

    #[rocket::async_test]
    async fn audit_rejects_cross_project_spoofing() {
        let user = DieselRepoMock::make_user(2, "member", "hash");
        let client = client_with_repo(DieselRepoMock::with_users([user])).await;
        let response = client
            .post("/api/mcp/audit")
            .header(ContentType::JSON)
            .header(rocket::http::Header::new(
                "X-Marreq-MCP-Audit-Secret",
                AUDIT_SECRET,
            ))
            .private_cookie(auth_cookie_for(&client, 2))
            .body(r#"{"project_id":999,"tool_name":"list_projects","is_write":false}"#)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn end_user_auth_alone_cannot_forge_audit() {
        let client = client_with_repo(DieselRepoMock::default().with_admin_user()).await;
        let response = client
            .post("/api/mcp/audit")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(&client))
            .body(r#"{"tool_name":"create_requirement","is_write":true}"#)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
    }

    #[test]
    fn mcp_audit_request_deserialize() {
        let json = r#"{"project_id":1,"tool_name":"x","is_write":true}"#;
        let req: McpAuditRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.project_id, Some(1));
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

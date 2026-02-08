//! Traceability (matrix) API: suspect link management.

use rocket::serde::Deserialize;

use crate::api::prelude::*;
use crate::services::MatrixService;

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct ClearSuspectRequest {
    pub req_id: i32,
    pub test_id: i32,
}

/// Clear the suspect flag for a traceability link. Records current user and timestamp (auditable).
#[post("/traceability/clear_suspect", data = "<body>")]
pub async fn clear_suspect(
    user: ApiUser,
    state: &State<AppState>,
    body: Json<ClearSuspectRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let payload = body.into_inner();
    let service = MatrixService::new(state.inner());
    let updated = service.clear_suspect(user.user(), payload.req_id, payload.test_id)?;
    Ok(Json(serde_json::json!({
        "status": if updated { "ok" } else { "no_change" },
        "cleared": updated
    })))
}

//! API for requirement comments: list and create (immutable).

use rocket::serde::Serialize;

use crate::api::prelude::*;
use crate::config;
use crate::models::{EntityType, NewLog, RequirementComment};
use crate::repository::{LogRepository, ProjectMembersRepository};
use crate::services::{CommentService, RequirementService, UserService};

/// Comment as returned by API (includes author display name).
#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct CommentResponse {
    pub id: i32,
    pub requirement_id: i32,
    pub requirement_version_id: Option<i32>,
    pub author_id: i32,
    pub author_name: String,
    pub body: String,
    pub created_at: chrono::NaiveDateTime,
}

fn comment_to_response(c: &RequirementComment, author_name: Option<String>) -> CommentResponse {
    CommentResponse {
        id: c.id,
        requirement_id: c.requirement_id,
        requirement_version_id: c.requirement_version_id,
        author_id: c.author_id,
        author_name: author_name.unwrap_or_else(|| format!("User#{}", c.author_id)),
        body: c.body.clone(),
        created_at: c.created_at,
    }
}

#[derive(Debug, rocket::serde::Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct CreateCommentRequest {
    pub body: String,
    pub requirement_version_id: Option<i32>,
}

/// List comments for a requirement, optionally filtered by version. Chronological order.
#[get("/requirements/<requirement_id>/comments?<version_id>")]
pub async fn list(
    _user: ApiUser,
    requirement_id: i32,
    version_id: Option<i32>,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<CommentResponse>>> {
    let requirement = RequirementService::new(state.inner()).get_by_id(requirement_id)?;
    let members = state
        .repo_read()
        .get_members_by_project(requirement.project_id)
        .map_err(ApiError::from)?;
    let u = _user.user();
    let can_access = u.is_admin || members.iter().any(|m| m.user_id == u.id);
    if !can_access {
        return Err(ApiError::Forbidden(
            "not a member of this requirement's project".into(),
        ));
    }
    let comments = CommentService::new(state.inner()).list_comments(requirement_id, version_id)?;
    let user_service = UserService::new(state.inner());
    let responses: Vec<CommentResponse> = comments
        .iter()
        .map(|c| {
            let name = user_service.get_by_id(c.author_id).ok().map(|u| u.name);
            comment_to_response(c, name)
        })
        .collect();
    Ok(Json(responses))
}

/// Create a comment. When LOCK_APPROVED_VERSION_COMMENTS is true, returns 403 if
/// requirement_version_id refers to an approved version.
#[post("/requirements/<requirement_id>/comments", data = "<payload>")]
pub async fn create(
    user: ApiUser,
    requirement_id: i32,
    state: &State<AppState>,
    payload: Json<CreateCommentRequest>,
) -> ApiResult<(Status, Json<CommentResponse>)> {
    let requirement = RequirementService::new(state.inner()).get_by_id(requirement_id)?;
    let members = state
        .repo_read()
        .get_members_by_project(requirement.project_id)
        .map_err(ApiError::from)?;
    let u = user.user();
    let can_access = u.is_admin || members.iter().any(|m| m.user_id == u.id);
    if !can_access {
        return Err(ApiError::Forbidden(
            "not a member of this requirement's project".into(),
        ));
    }
    if let Some(version_id) = payload.requirement_version_id {
        let version = RequirementService::new(state.inner())
            .get_version_by_id(version_id)
            .map_err(ApiError::from)?;
        if version.requirement_id != requirement_id {
            return Err(ApiError::NotFound(
                "version does not belong to requirement".into(),
            ));
        }
        if config::lock_approved_version_comments()
            && version.approval_state.eq_ignore_ascii_case("approved")
        {
            return Err(ApiError::Forbidden(
                "approved versions cannot receive new comments when locked".into(),
            ));
        }
    }
    let comment = CommentService::new(state.inner()).create_comment(
        requirement_id,
        payload.requirement_version_id,
        u.id,
        payload.body.clone(),
    )?;
    let author_name = UserService::new(state.inner())
        .get_by_id(comment.author_id)
        .ok()
        .map(|u| u.name);
    let response = comment_to_response(&comment, author_name);

    let new_log = NewLog {
        user_id: u.id,
        action_type: "CREATE".to_string(),
        entity_type: EntityType::Comment.to_string(),
        entity_id: Some(comment.id),
        project_id: Some(requirement.project_id),
        old_values: None,
        new_values: None,
        description: Some(format!(
            "Comment on requirement {} (version {:?})",
            requirement_id, payload.requirement_version_id
        )),
        ip_address: user.log_ctx().ip_address().map(str::to_string),
        user_agent: user.log_ctx().user_agent().map(str::to_string),
    };
    let _ = state.repo_write().insert_log(&new_log);

    Ok((Status::Created, Json(response)))
}

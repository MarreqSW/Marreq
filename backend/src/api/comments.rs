// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! API for requirement comments: list and create (immutable).

use rocket::serde::{Deserialize, Serialize};

use crate::api::prelude::*;
use crate::config;
use crate::models::{EntityType, NewLog, RequirementComment};
use crate::repository::{LogRepository, ProjectMembersRepository};
use crate::services::{CommentService, RequirementService, UserService};

/// Comment as returned by API (includes author display name).
#[derive(Debug, Serialize, Deserialize)]
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
        u,
        requirement_id,
        payload.requirement_version_id,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::models::{Project, ProjectMember, Requirement, RequirementComment};
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use crate::status_enums::ProjectStatus;
    use chrono::NaiveDate;
    use rocket::http::{ContentType, Cookie, SameSite, Status};
    use rocket::local::asynchronous::Client;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;
    const PROJECT_ID: i32 = 1;
    const REQ_ID: i32 = 1;

    fn epoch() -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(state_from_repo(repo))
            .mount("/api", routes![list, create]);
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_cookie(user_id: i32) -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, user_id.to_string());
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(SameSite::Strict);
        cookie
    }

    fn repo_with_requirement_and_member() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default().with_admin_user();
        repo.projects.insert(
            PROJECT_ID,
            Project {
                id: PROJECT_ID,
                name: "Test Project".into(),
                description: Some("Desc".into()),
                creation_date: None,
                update_date: None,
                status: ProjectStatus::Active,
                owner_id: Some(ADMIN_ID),
                slug: "test-project".into(),
                group_id: None,
            },
        );
        repo.project_members.push(ProjectMember {
            project_id: PROJECT_ID,
            user_id: ADMIN_ID,
            role: 1,
            created_at: epoch(),
            updated_at: epoch(),
        });
        repo.requirements.insert(
            REQ_ID,
            Requirement {
                id: REQ_ID,
                current_version_id: None,
                same_as_current: None,
                title: "Req".into(),
                description: "Desc".into(),
                status_id: 1,
                author_id: ADMIN_ID,
                reviewer_id: ADMIN_ID,
                reference_code: "R1".into(),
                category_id: 1,
                parent_id: None,
                creation_date: epoch(),
                update_date: epoch(),
                deadline_date: None,
                applicability_id: 1,
                justification: None,
                project_id: PROJECT_ID,
                approval_state: "draft".into(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            },
        );
        repo
    }

    #[rocket::async_test]
    async fn list_returns_empty_when_no_comments() {
        let client = client_with_repo(repo_with_requirement_and_member()).await;
        let response = client
            .get(format!("/api/requirements/{}/comments", REQ_ID))
            .private_cookie(auth_cookie(ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: Vec<CommentResponse> = response.into_json().await.unwrap();
        assert!(body.is_empty());
    }

    #[rocket::async_test]
    async fn list_returns_comments_when_present() {
        let mut repo = repo_with_requirement_and_member();
        repo.requirement_comments.push(RequirementComment {
            id: 1,
            requirement_id: REQ_ID,
            requirement_version_id: None,
            author_id: ADMIN_ID,
            body: "A comment".into(),
            created_at: epoch(),
        });
        let client = client_with_repo(repo).await;
        let response = client
            .get(format!("/api/requirements/{}/comments", REQ_ID))
            .private_cookie(auth_cookie(ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let body: Vec<CommentResponse> = response.into_json().await.unwrap();
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].body, "A comment");
        assert_eq!(body[0].requirement_id, REQ_ID);
    }

    #[rocket::async_test]
    async fn list_returns_403_when_not_project_member() {
        let mut repo = repo_with_requirement_and_member();
        repo.users.get_mut(&ADMIN_ID).unwrap().is_admin = false;
        repo.project_members.clear();
        let client = client_with_repo(repo).await;
        let response = client
            .get(format!("/api/requirements/{}/comments", REQ_ID))
            .private_cookie(auth_cookie(ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn list_returns_404_when_requirement_missing() {
        let client = client_with_repo(repo_with_requirement_and_member()).await;
        let response = client
            .get("/api/requirements/999/comments")
            .private_cookie(auth_cookie(ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn create_returns_201_and_comment_body() {
        let client = client_with_repo(repo_with_requirement_and_member()).await;
        let response = client
            .post(format!("/api/requirements/{}/comments", REQ_ID))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(ADMIN_ID))
            .body(r#"{"body":"New comment"}"#)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);
        let body: CommentResponse = response.into_json().await.unwrap();
        assert_eq!(body.body, "New comment");
        assert_eq!(body.requirement_id, REQ_ID);
        assert_eq!(body.author_id, ADMIN_ID);
    }

    #[rocket::async_test]
    async fn create_returns_403_when_not_project_member() {
        let mut repo = repo_with_requirement_and_member();
        repo.users.get_mut(&ADMIN_ID).unwrap().is_admin = false;
        repo.project_members.clear();
        let client = client_with_repo(repo).await;
        let response = client
            .post(format!("/api/requirements/{}/comments", REQ_ID))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(ADMIN_ID))
            .body(r#"{"body":"New comment"}"#)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn create_returns_404_when_requirement_missing() {
        let client = client_with_repo(repo_with_requirement_and_member()).await;
        let response = client
            .post("/api/requirements/999/comments")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie(ADMIN_ID))
            .body(r#"{"body":"New comment"}"#)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);
    }

    #[test]
    fn comment_to_response_uses_author_name_when_available() {
        let c = RequirementComment {
            id: 1,
            requirement_id: REQ_ID,
            requirement_version_id: None,
            author_id: ADMIN_ID,
            body: "Hi".into(),
            created_at: epoch(),
        };
        let r = comment_to_response(&c, Some("Alice".into()));
        assert_eq!(r.author_name, "Alice");
        assert_eq!(r.body, "Hi");
    }

    #[test]
    fn comment_to_response_falls_back_to_user_id_when_name_missing() {
        let c = RequirementComment {
            id: 1,
            requirement_id: REQ_ID,
            requirement_version_id: None,
            author_id: 42,
            body: "Hi".into(),
            created_at: epoch(),
        };
        let r = comment_to_response(&c, None);
        assert_eq!(r.author_name, "User#42");
    }
}

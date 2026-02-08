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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::models::MatrixLink;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::ContentType;
    use rocket::local::asynchronous::Client;
    use rocket::serde::json::Value;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(state_from_repo(repo.with_admin_user()))
            .mount("/api", routes![clear_suspect]);
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_cookie() -> rocket::http::Cookie<'static> {
        let mut cookie = rocket::http::Cookie::new(SESSION_COOKIE, "1");
        cookie.set_path("/");
        cookie
    }

    #[rocket::async_test]
    async fn clear_suspect_returns_ok_and_cleared_true_when_link_was_suspect() {
        let mut repo = DieselRepoMock::default();
        repo.matrices.push(MatrixLink {
            req_id: 1,
            test_id: 2,
            creation_date: chrono::Utc::now().naive_utc(),
            project_id: 7,
            suspect: true,
            suspect_at: Some(chrono::Utc::now().naive_utc()),
            suspect_reason: Some("Requirement updated".into()),
            cleared_by: None,
            cleared_at: None,
        });
        let client = client_with_repo(repo).await;
        let response = client
            .post("/api/traceability/clear_suspect")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(r#"{"req_id":1,"test_id":2}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), rocket::http::Status::Ok);
        let body: Value = response.into_json().await.unwrap();
        assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("ok"));
        assert_eq!(body.get("cleared"), Some(&Value::from(true)));
    }

    #[rocket::async_test]
    async fn clear_suspect_returns_ok_and_cleared_false_when_link_missing() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .post("/api/traceability/clear_suspect")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(r#"{"req_id":99,"test_id":99}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), rocket::http::Status::Ok);
        let body: Value = response.into_json().await.unwrap();
        assert_eq!(
            body.get("status").and_then(|v| v.as_str()),
            Some("no_change")
        );
        assert_eq!(body.get("cleared"), Some(&Value::from(false)));
    }

    #[rocket::async_test]
    async fn clear_suspect_requires_auth() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .post("/api/traceability/clear_suspect")
            .header(ContentType::JSON)
            .body(r#"{"req_id":1,"test_id":1}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), rocket::http::Status::Unauthorized);
    }
}

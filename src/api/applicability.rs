use crate::api::prelude::*;
use crate::models::{Applicability, NewApplicability};
use crate::services::ApplicabilityService;

#[get("/applicability")]
pub async fn list(_user: ApiUser, state: &State<AppState>) -> ApiResult<Json<Vec<Applicability>>> {
    let service = ApplicabilityService::new(state.inner());
    let items = service.list_all()?;
    Ok(Json(items))
}

#[get("/applicability/<id>")]
pub async fn get(
    _user: ApiUser,
    state: &State<AppState>,
    id: i32,
) -> ApiResult<Json<Applicability>> {
    let service = ApplicabilityService::new(state.inner());
    let applicability = service.get_by_id(id)?;

    Ok(Json(applicability))
}

#[post("/applicability", data = "<payload>")]
pub async fn create(
    user: ApiUser,
    state: &State<AppState>,
    payload: Json<NewApplicability>,
) -> ApiResult<(Status, Value)> {
    let service = ApplicabilityService::new(state.inner());
    let id = service.create(user.user(), payload.into_inner())?;

    Ok((Status::Created, json!({ "status": "ok", "id": id })))
}

#[put("/applicability/<id>", data = "<payload>")]
pub async fn update(
    user: ApiUser,
    state: &State<AppState>,
    id: i32,
    payload: Json<NewApplicability>,
) -> ApiResult<Value> {
    let service = ApplicabilityService::new(state.inner());
    service.update(user.user(), id, payload.into_inner())?;

    Ok(json!({
        "status": "ok",
        "message": "Applicability updated successfully"
    }))
}

#[delete("/applicability/<id>")]
pub async fn delete(user: ApiUser, state: &State<AppState>, id: i32) -> ApiResult<Status> {
    let service = ApplicabilityService::new(state.inner());
    service.delete(user.user(), id)?;
    Ok(Status::NoContent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::{ContentType, Cookie};
    use rocket::local::asynchronous::{Client, LocalResponse};
    use serde_json::{json, Value};
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;

    fn test_state() -> TestState {
        let repo = CacheRepository::new(DieselRepoMock::default().with_admin_user(), 0);
        AppState {
            repo: Arc::new(RwLock::new(repo)),
        }
    }

    async fn test_client() -> Client {
        let rocket = rocket::build()
            .manage(test_state())
            .mount("/api", routes![list, get, create, update, delete]);
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
        cookie.set_path("/");
        cookie
    }

    async fn post_with_body<'c>(
        client: &'c Client,
        path: &'c str,
        body: Value,
    ) -> LocalResponse<'c> {
        client
            .post(path)
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(body.to_string())
            .dispatch()
            .await
    }

    #[rocket::async_test]
    async fn list_returns_empty_array() {
        let client = test_client().await;
        let response = client
            .get("/api/applicability")
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let items: Vec<Applicability> = response.into_json().await.unwrap();
        assert!(items.is_empty());
    }

    #[rocket::async_test]
    async fn create_returns_created_id() {
        let client = test_client().await;
        let response = post_with_body(
            &client,
            "/api/applicability",
            json!({
                "id": null,
                "title": "Safety",
                "description": "Applies to safety",
                "tag": "safety",
                "project_id": 1
            }),
        )
        .await;

        assert_eq!(response.status(), Status::Created);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("status"), Some(&Value::from("ok")));
        assert_eq!(payload.get("id"), Some(&Value::from(1)));
    }

    #[rocket::async_test]
    async fn update_changes_existing_applicability() {
        let client = test_client().await;
        let create_response = post_with_body(
            &client,
            "/api/applicability",
            json!({
                "id": null,
                "title": "Legacy",
                "description": "Old description",
                "tag": "legacy",
                "project_id": 2
            }),
        )
        .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let response = client
            .put(format!("/api/applicability/{id}"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(
                json!({
                    "id": id,
                    "title": "Modern",
                    "description": "Updated description",
                    "tag": "modern",
                    "project_id": 2
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let message: Value = response.into_json().await.unwrap();
        assert_eq!(
            message.get("message"),
            Some(&Value::from("Applicability updated successfully"))
        );

        let get_response = client
            .get(format!("/api/applicability/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        let fetched: Applicability = get_response.into_json().await.unwrap();
        assert_eq!(fetched.title, "Modern");
        assert_eq!(fetched.description, "Updated description");
    }

    #[rocket::async_test]
    async fn delete_removes_applicability() {
        let client = test_client().await;
        let create_response = post_with_body(
            &client,
            "/api/applicability",
            json!({
                "id": null,
                "title": "To be removed",
                "description": "Temporary",
                "tag": "temp",
                "project_id": 1
            }),
        )
        .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let delete_response = client
            .delete(format!("/api/applicability/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(delete_response.status(), Status::NoContent);

        let not_found = client
            .get(format!("/api/applicability/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(not_found.status(), Status::NotFound);
    }
}

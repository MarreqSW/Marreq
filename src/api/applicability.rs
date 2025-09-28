use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{Applicability, NewApplicability};
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;

#[get("/applicability")]
pub async fn list(_user: ApiUser, state: &State<AppState>) -> ApiResult<Json<Vec<Applicability>>> {
    let items = state
        .repo
        .async_read(|repo| repo.get_applicability_all())
        .await?;
    Ok(Json(items))
}

#[get("/applicability/<id>")]
pub async fn get(
    _user: ApiUser,
    state: &State<AppState>,
    id: i32,
) -> ApiResult<Json<Applicability>> {
    let applicability = state
        .repo
        .async_read(move |repo| repo.get_applicability_by_id(id))
        .await?;

    Ok(Json(applicability))
}

#[post("/applicability", data = "<payload>")]
pub async fn create(
    user: ApiUser,
    state: &State<AppState>,
    payload: Json<NewApplicability>,
) -> ApiResult<(Status, Value)> {
    let app = payload.into_inner();
    let log_ctx = user.log_ctx().clone();

    let id = state
        .repo
        .async_write(move |repo| {
            let id = repo.insert_new_applicability(&app)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let _ = Logger::created(conn.as_mut(), &log_ctx, id, &app);
            }
            Ok::<_, RepoError>(id)
        })
        .await?;

    Ok((Status::Created, json!({ "status": "ok", "id": id })))
}

#[put("/applicability/<id>", data = "<payload>")]
pub async fn update(
    user: ApiUser,
    state: &State<AppState>,
    id: i32,
    payload: Json<NewApplicability>,
) -> ApiResult<Value> {
    let mut app = payload.into_inner();
    app.app_id = Some(id);
    let log_ctx = user.log_ctx().clone();

    state
        .repo
        .async_write(move |repo| {
            let before = match repo.get_applicability_by_id(id) {
                Ok(a) => Some(a),
                Err(RepoError::NotFound) => None,
                Err(e) => return Err(e),
            };

            let updated = repo.edit_applicability(&app)?;
            if !updated {
                return Err(RepoError::NotFound);
            }

            if let Some(prev) = before {
                let after = Applicability {
                    app_id: id,
                    app_title: app.app_title.clone(),
                    app_description: app.app_description.clone(),
                    app_tag: app.app_tag.clone(),
                    project_id: app.project_id,
                };
                if let Ok(mut conn) = repo.inner_repo().get_conn() {
                    let _ = Logger::updated(conn.as_mut(), &log_ctx, &prev, &after);
                }
            }

            Ok::<_, RepoError>(())
        })
        .await?;

    Ok(json!({
        "status": "ok",
        "message": "Applicability updated successfully"
    }))
}

#[delete("/applicability/<id>")]
pub async fn delete(user: ApiUser, state: &State<AppState>, id: i32) -> ApiResult<Status> {
    let log_ctx = user.log_ctx().clone();
    state
        .repo
        .async_write(move |repo| {
            let removed = repo.delete_applicability(id)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let _ = Logger::deleted(conn.as_mut(), &log_ctx, &removed);
            }
            Ok::<_, RepoError>(())
        })
        .await?;
    Ok(Status::NoContent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::repository::{fake_repo::FakeRepo, CacheRepository};
    use rocket::http::{ContentType, Header};
    use rocket::local::asynchronous::{Client, LocalResponse};
    use serde_json::{json, Value};
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<FakeRepo>>;

    fn test_state() -> TestState {
        let repo = CacheRepository::new(FakeRepo::default(), 0);
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

    fn auth_header() -> Header<'static> {
        Header::new("x-test-user", "admin")
    }

    async fn post_with_body<'c>(
        client: &'c Client,
        path: &'c str,
        body: Value,
    ) -> LocalResponse<'c> {
        client
            .post(path)
            .header(ContentType::JSON)
            .header(auth_header())
            .body(body.to_string())
            .dispatch()
            .await
    }

    #[rocket::async_test]
    async fn list_returns_empty_array() {
        let client = test_client().await;
        let response = client
            .get("/api/applicability")
            .header(auth_header())
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
                "app_id": null,
                "app_title": "Safety",
                "app_description": "Applies to safety",
                "app_tag": "safety",
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
                "app_id": null,
                "app_title": "Legacy",
                "app_description": "Old description",
                "app_tag": "legacy",
                "project_id": 2
            }),
        )
        .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let response = client
            .put(format!("/api/applicability/{id}"))
            .header(ContentType::JSON)
            .header(auth_header())
            .body(
                json!({
                    "app_id": id,
                    "app_title": "Modern",
                    "app_description": "Updated description",
                    "app_tag": "modern",
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
            .header(auth_header())
            .dispatch()
            .await;
        let fetched: Applicability = get_response.into_json().await.unwrap();
        assert_eq!(fetched.app_title, "Modern");
        assert_eq!(fetched.app_description, "Updated description");
    }

    #[rocket::async_test]
    async fn delete_removes_applicability() {
        let client = test_client().await;
        let create_response = post_with_body(
            &client,
            "/api/applicability",
            json!({
                "app_id": null,
                "app_title": "To be removed",
                "app_description": "Temporary",
                "app_tag": "temp",
                "project_id": 1
            }),
        )
        .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let delete_response = client
            .delete(format!("/api/applicability/{id}"))
            .header(auth_header())
            .dispatch()
            .await;
        assert_eq!(delete_response.status(), Status::NoContent);

        let not_found = client
            .get(format!("/api/applicability/{id}"))
            .header(auth_header())
            .dispatch()
            .await;
        assert_eq!(not_found.status(), Status::NotFound);
    }
}

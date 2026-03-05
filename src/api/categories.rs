// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::api::prelude::*;
use crate::models::{Category, NewCategory};
use crate::services::CategoryService;

#[get("/categories")]
pub async fn list(_user: ApiUser, state: &State<AppState>) -> ApiResult<Json<Vec<Category>>> {
    let service = CategoryService::new(state.inner());
    let categories = service.list_all()?;
    Ok(Json(categories))
}

#[get("/categories/<id>")]
pub async fn get(_user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Json<Category>> {
    let service = CategoryService::new(state.inner());
    let category = service.get_by_id(id)?;
    Ok(Json(category))
}

#[post("/categories", data = "<payload>")]
pub async fn create(
    user: ApiUser,
    state: &State<AppState>,
    payload: Json<NewCategory>,
) -> ApiResult<Value> {
    let service = CategoryService::new(state.inner());
    let id = service.create(user.user(), payload.into_inner())?;

    Ok(json!({ "status": "ok", "id": id }))
}

#[put("/categories/<id>", data = "<payload>")]
pub async fn update(
    user: ApiUser,
    state: &State<AppState>,
    id: i32,
    payload: Json<NewCategory>,
) -> ApiResult<Value> {
    let service = CategoryService::new(state.inner());
    service.update(user.user(), id, payload.into_inner())?;

    Ok(json!({
        "status": "ok",
        "message": "Category updated successfully"
    }))
}

#[delete("/categories/<id>")]
pub async fn delete(user: ApiUser, state: &State<AppState>, id: i32) -> ApiResult<Status> {
    let service = CategoryService::new(state.inner());
    service.delete(user.user(), id)?;

    Ok(Status::NoContent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::{ContentType, Cookie, SameSite};
    use rocket::local::asynchronous::Client;
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
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(SameSite::Strict);
        cookie
    }

    #[rocket::async_test]
    async fn list_returns_empty_array() {
        let client = test_client().await;
        let response = client
            .get("/api/categories")
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let categories: Vec<Category> = response.into_json().await.unwrap();
        assert!(categories.is_empty());
    }

    #[rocket::async_test]
    async fn create_returns_created_id() {
        let client = test_client().await;
        let response = client
            .post("/api/categories")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(
                json!({
                    "id": null,
                    "title": "Category",
                    "description": "Initial description",
                    "tag": "tag",
                    "project_id": 1
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("status"), Some(&Value::from("ok")));
        assert_eq!(payload.get("id"), Some(&Value::from(1)));
    }

    #[rocket::async_test]
    async fn update_changes_existing_category() {
        let client = test_client().await;
        let create_response = client
            .post("/api/categories")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(
                json!({
                    "id": null,
                    "title": "Legacy",
                    "description": "Old",
                    "tag": "legacy",
                    "project_id": 7
                })
                .to_string(),
            )
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let response = client
            .put(format!("/api/categories/{id}"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(
                json!({
                    "id": id,
                    "title": "Updated",
                    "description": "Refreshed",
                    "tag": "updated",
                    "project_id": 7
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(
            payload.get("message"),
            Some(&Value::from("Category updated successfully"))
        );

        let get_response = client
            .get(format!("/api/categories/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        let category: Category = get_response.into_json().await.unwrap();
        assert_eq!(category.title, "Updated");
        assert_eq!(category.description, "Refreshed");
    }

    #[rocket::async_test]
    async fn delete_removes_category() {
        let client = test_client().await;
        let create_response = client
            .post("/api/categories")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(
                json!({
                    "id": null,
                    "title": "Disposable",
                    "description": "Temporary",
                    "tag": "temp",
                    "project_id": 1
                })
                .to_string(),
            )
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let delete_response = client
            .delete(format!("/api/categories/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(delete_response.status(), Status::NoContent);

        let not_found = client
            .get(format!("/api/categories/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(not_found.status(), Status::NotFound);
    }
}

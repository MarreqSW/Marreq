use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{Category, NewCategory};
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;

#[get("/categories")]
pub async fn list(_user: ApiUser, state: &State<AppState>) -> ApiResult<Json<Vec<Category>>> {
    let categories = state
        .repo
        .async_read(|repo| repo.get_categories_all())
        .await?;
    Ok(Json(categories))
}

#[get("/categories/<id>")]
pub async fn get(_user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Json<Category>> {
    let category = state
        .repo
        .async_read(move |repo| repo.get_category_by_id(id))
        .await?;
    Ok(Json(category))
}

#[post("/categories", data = "<payload>")]
pub async fn create(
    user: ApiUser,
    state: &State<AppState>,
    payload: Json<NewCategory>,
) -> ApiResult<Value> {
    let category = payload.into_inner();
    let log_ctx = user.log_ctx().clone();

    let id = state
        .repo
        .async_write(move |repo| {
            let id = repo.insert_new_category(&category)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let _ = Logger::created(conn.as_mut(), &log_ctx, id, &category);
            }

            Ok::<_, RepoError>(id)
        })
        .await?;

    Ok(json!({ "status": "ok", "id": id }))
}

#[put("/categories/<id>", data = "<payload>")]
pub async fn update(
    user: ApiUser,
    state: &State<AppState>,
    id: i32,
    payload: Json<NewCategory>,
) -> ApiResult<Value> {
    let mut category = payload.into_inner();
    category.cat_id = Some(id);
    let project_id = category.project_id;
    let log_ctx = user.log_ctx().clone();

    state
        .repo
        .async_write(move |repo| {
            let before = match repo.get_category_by_id(id) {
                Ok(c) => Some(c),
                Err(RepoError::NotFound) => None,
                Err(e) => return Err(e),
            };

            let updated = repo.edit_category(&category)?;
            if !updated {
                return Err(RepoError::NotFound);
            }

            if let Some(previous) = before {
                let after = Category {
                    cat_id: id,
                    cat_title: category.cat_title.clone(),
                    cat_description: category.cat_description.clone(),
                    cat_tag: category.cat_tag.clone(),
                    project_id,
                };
                if let Ok(mut conn) = repo.inner_repo().get_conn() {
                    let _ = Logger::updated(conn.as_mut(), &log_ctx, &previous, &after);
                }
            }

            Ok::<_, RepoError>(())
        })
        .await?;

    Ok(json!({
        "status": "ok",
        "message": "Category updated successfully"
    }))
}

#[delete("/categories/<id>")]
pub async fn delete(user: ApiUser, state: &State<AppState>, id: i32) -> ApiResult<Status> {
    let log_ctx = user.log_ctx().clone();
    state
        .repo
        .async_write(move |repo| {
            let removed = repo.delete_category(id)?;
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
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::{ContentType, Header};
    use rocket::local::asynchronous::Client;
    use serde_json::{json, Value};
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    fn test_state() -> TestState {
        let repo = CacheRepository::new(DieselRepoMock::default(), 0);
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

    #[rocket::async_test]
    async fn list_returns_empty_array() {
        let client = test_client().await;
        let response = client
            .get("/api/categories")
            .header(auth_header())
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
            .header(auth_header())
            .body(
                json!({
                    "cat_id": null,
                    "cat_title": "Category",
                    "cat_description": "Initial description",
                    "cat_tag": "tag",
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
            .header(auth_header())
            .body(
                json!({
                    "cat_id": null,
                    "cat_title": "Legacy",
                    "cat_description": "Old",
                    "cat_tag": "legacy",
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
            .header(auth_header())
            .body(
                json!({
                    "cat_id": id,
                    "cat_title": "Updated",
                    "cat_description": "Refreshed",
                    "cat_tag": "updated",
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
            .header(auth_header())
            .dispatch()
            .await;
        let category: Category = get_response.into_json().await.unwrap();
        assert_eq!(category.cat_title, "Updated");
        assert_eq!(category.cat_description, "Refreshed");
    }

    #[rocket::async_test]
    async fn delete_removes_category() {
        let client = test_client().await;
        let create_response = client
            .post("/api/categories")
            .header(ContentType::JSON)
            .header(auth_header())
            .body(
                json!({
                    "cat_id": null,
                    "cat_title": "Disposable",
                    "cat_description": "Temporary",
                    "cat_tag": "temp",
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
            .header(auth_header())
            .dispatch()
            .await;
        assert_eq!(delete_response.status(), Status::NoContent);

        let not_found = client
            .get(format!("/api/categories/{id}"))
            .header(auth_header())
            .dispatch()
            .await;
        assert_eq!(not_found.status(), Status::NotFound);
    }
}

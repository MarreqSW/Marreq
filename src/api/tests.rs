use rocket::serde::{Deserialize, Serialize};

use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{NewTest, Test};
use crate::repository::errors::RepoError;
use crate::repository::TestsRepository;

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FieldUpdateRequest {
    pub field: String,
    pub value: String,
}

#[get("/tests")]
pub async fn list(_user: ApiUser, state: &State<AppState>) -> ApiResult<Json<Vec<Test>>> {
    let tests = state.repo.async_read(|repo| repo.get_tests_all()).await?;
    Ok(Json(tests))
}

#[get("/tests/<id>")]
pub async fn get(_user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Json<Test>> {
    let test = state
        .repo
        .async_read(move |repo| repo.get_test_by_id(id))
        .await?;
    Ok(Json(test))
}

#[post("/tests", data = "<payload>")]
pub async fn create(
    user: ApiUser,
    state: &State<AppState>,
    payload: Json<NewTest>,
) -> ApiResult<Value> {
    let test = payload.into_inner();
    let log_ctx = user.log_ctx().clone();

    let id = state
        .repo
        .async_write(move |repo| {
            let id = repo.insert_test(&test)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let _ = Logger::created(conn.as_mut(), &log_ctx, id, &test);
            }
            Ok::<_, RepoError>(id)
        })
        .await?;

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/tests/<id>")]
pub async fn delete(user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Status> {
    let log_ctx = user.log_ctx().clone();
    state
        .repo
        .async_write(move |repo| {
            let removed = repo.delete_test(id)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let _ = Logger::deleted(conn.as_mut(), &log_ctx, &removed);
            }
            Ok::<_, RepoError>(())
        })
        .await?;
    Ok(Status::NoContent)
}

#[post("/tests/<id>/field", data = "<update>")]
pub async fn update_field(
    user: ApiUser,
    id: i32,
    state: &State<AppState>,
    update: Json<FieldUpdateRequest>,
) -> ApiResult<Value> {
    let update = update.into_inner();
    let log_ctx = user.log_ctx().clone();

    state
        .repo
        .async_write(move |repo| {
            let mut test = repo.get_test_by_id(id)?;
            let original = test.clone();

            match update.field.as_str() {
                "test_name" => test.test_name = update.value,
                "test_description" => test.test_description = update.value,
                "test_source" => test.test_source = update.value,
                "test_status" => {
                    test.test_status = update
                        .value
                        .parse()
                        .map_err(|_| RepoError::BadInput("invalid status id".into()))?;
                }
                "test_reference" => test.test_reference = update.value,
                "test_parent" => {
                    test.test_parent = update
                        .value
                        .parse()
                        .map_err(|_| RepoError::BadInput("invalid parent id".into()))?;
                }
                other => return Err(RepoError::BadInput(format!("unsupported field '{other}'"))),
            }

            let payload = NewTest {
                test_id: Some(test.test_id),
                test_name: test.test_name.clone(),
                test_description: test.test_description.clone(),
                test_source: test.test_source.clone(),
                test_status: test.test_status,
                test_reference: test.test_reference.clone(),
                test_parent: test.test_parent,
                project_id: test.project_id,
            };

            repo.edit_test(&payload)?;

            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let _ = Logger::updated(conn.as_mut(), &log_ctx, &original, &test);
            }

            Ok::<_, RepoError>(())
        })
        .await?;

    Ok(json!({
        "success": true,
        "message": "Field updated successfully"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::repository::{fake_repo::FakeRepo, CacheRepository};
    use rocket::http::{ContentType, Header};
    use rocket::local::asynchronous::Client;
    use serde_json::{json, Value};
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<FakeRepo>>;

    fn state_from_repo(repo: FakeRepo) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: FakeRepo) -> Client {
        let rocket = rocket::build()
            .manage(state_from_repo(repo))
            .mount("/api", routes![list, get, create, delete, update_field]);
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_header() -> Header<'static> {
        Header::new("x-test-user", "admin")
    }

    fn sample_test(name: &str) -> Value {
        json!({
            "test_id": null,
            "test_name": name,
            "test_description": format!("{name} description"),
            "test_source": "manual",
            "test_status": 1,
            "test_reference": "T-1",
            "test_parent": 0,
            "project_id": 1
        })
    }

    #[rocket::async_test]
    async fn list_returns_empty_array() {
        let client = client_with_repo(FakeRepo::default()).await;
        let response = client
            .get("/api/tests")
            .header(auth_header())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let items: Vec<Test> = response.into_json().await.unwrap();
        assert!(items.is_empty());
    }

    #[rocket::async_test]
    async fn create_returns_identifier() {
        let client = client_with_repo(FakeRepo::default()).await;
        let response = client
            .post("/api/tests")
            .header(ContentType::JSON)
            .header(auth_header())
            .body(sample_test("Baseline").to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("status"), Some(&Value::from("ok")));
        assert_eq!(payload.get("id"), Some(&Value::from(1)));
    }

    #[rocket::async_test]
    async fn update_field_changes_name() {
        let client = client_with_repo(FakeRepo::default()).await;
        let create_response = client
            .post("/api/tests")
            .header(ContentType::JSON)
            .header(auth_header())
            .body(sample_test("Scenario").to_string())
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let response = client
            .post(format!("/api/tests/{id}/field"))
            .header(ContentType::JSON)
            .header(auth_header())
            .body(
                json!({
                    "field": "test_name",
                    "value": "Updated"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("success"), Some(&Value::from(true)));

        let get_response = client
            .get(format!("/api/tests/{id}"))
            .header(auth_header())
            .dispatch()
            .await;
        let test: Test = get_response.into_json().await.unwrap();
        assert_eq!(test.test_name, "Updated");
    }

    #[rocket::async_test]
    async fn delete_removes_test() {
        let client = client_with_repo(FakeRepo::default()).await;
        let create_response = client
            .post("/api/tests")
            .header(ContentType::JSON)
            .header(auth_header())
            .body(sample_test("Disposable").to_string())
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let delete_response = client
            .delete(format!("/api/tests/{id}"))
            .header(auth_header())
            .dispatch()
            .await;
        assert_eq!(delete_response.status(), Status::NoContent);

        let not_found = client
            .get(format!("/api/tests/{id}"))
            .header(auth_header())
            .dispatch()
            .await;
        assert_eq!(not_found.status(), Status::NotFound);
    }
}

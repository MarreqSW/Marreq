use rocket::serde::{Deserialize, Serialize};

use crate::api::prelude::*;
use crate::models::{NewTest, Test};
use crate::repository::errors::RepoError;
use crate::services::TestService;

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct FieldUpdateRequest {
    pub field: String,
    pub value: String,
}

#[get("/tests")]
pub async fn list(_user: ApiUser, state: &State<AppState>) -> ApiResult<Json<Vec<Test>>> {
    let service = TestService::new(state.inner());
    let tests = service.list_all()?;
    Ok(Json(tests))
}

#[get("/tests/<id>")]
pub async fn get(_user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Json<Test>> {
    let service = TestService::new(state.inner());
    let test = service.get_by_id(id)?;
    Ok(Json(test))
}

#[post("/tests", data = "<payload>")]
pub async fn create(
    user: ApiUser,
    state: &State<AppState>,
    payload: Json<NewTest>,
) -> ApiResult<Value> {
    let service = TestService::new(state.inner());
    let id = service.create(user.user(), payload.into_inner())?;

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/tests/<id>")]
pub async fn delete(user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Status> {
    let service = TestService::new(state.inner());
    service.delete(user.user(), id)?;
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
    let service = TestService::new(state.inner());
    let mut test = service.get_by_id(id)?;

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
        other => {
            return Err(ApiError::from(RepoError::BadInput(format!(
                "unsupported field '{other}'"
            ))))
        }
    }

    let payload = NewTest {
        test_id: Some(test.test_id),
        test_reference: test.test_reference.clone(),
        test_name: test.test_name.clone(),
        test_description: test.test_description.clone(),
        test_source: test.test_source.clone(),
        test_status: test.test_status,
        test_parent: test.test_parent,
        project_id: test.project_id,
    };

    service.update(user.user(), id, payload)?;

    Ok(json!({
        "success": true,
        "message": "Field updated successfully"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::{ContentType, Cookie};
    use rocket::local::asynchronous::Client;
    use serde_json::{json, Value};
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(state_from_repo(repo.with_admin_user()))
            .mount("/api", routes![list, get, create, delete, update_field]);
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
        cookie.set_path("/");
        cookie
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
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .get("/api/tests")
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let items: Vec<Test> = response.into_json().await.unwrap();
        assert!(items.is_empty());
    }

    #[rocket::async_test]
    async fn create_returns_identifier() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .post("/api/tests")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
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
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_response = client
            .post("/api/tests")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(sample_test("Scenario").to_string())
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let response = client
            .post(format!("/api/tests/{id}/field"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
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
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        let test: Test = get_response.into_json().await.unwrap();
        assert_eq!(test.test_name, "Updated");
    }

    #[rocket::async_test]
    async fn delete_removes_test() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_response = client
            .post("/api/tests")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(sample_test("Disposable").to_string())
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let delete_response = client
            .delete(format!("/api/tests/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(delete_response.status(), Status::NoContent);

        let not_found = client
            .get(format!("/api/tests/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(not_found.status(), Status::NotFound);
    }
}

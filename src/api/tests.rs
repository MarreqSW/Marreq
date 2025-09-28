use rocket::serde::{Deserialize, Serialize};

use crate::api::prelude::*;
use crate::logger::{LogCtx, Logger};
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
pub async fn list(state: &State<AppState>) -> ApiResult<Json<Vec<Test>>> {
    let tests = state.repo.async_read(|repo| repo.get_tests_all()).await?;
    Ok(Json(tests))
}

#[get("/tests/<id>")]
pub async fn get(id: i32, state: &State<AppState>) -> ApiResult<Json<Test>> {
    let test = state
        .repo
        .async_read(move |repo| repo.get_test_by_id(id))
        .await?;
    Ok(Json(test))
}

#[post("/tests", data = "<payload>")]
pub async fn create(state: &State<AppState>, payload: Json<NewTest>) -> ApiResult<Value> {
    let test = payload.into_inner();

    let id = state
        .repo
        .async_write(move |repo| {
            let id = repo.insert_test(&test)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let ctx = LogCtx::new(0);
                let _ = Logger::created(conn.as_mut(), &ctx, id, &test);
            }
            Ok::<_, RepoError>(id)
        })
        .await?;

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/tests/<id>")]
pub async fn delete(id: i32, state: &State<AppState>) -> ApiResult<Status> {
    state
        .repo
        .async_write(move |repo| {
            let removed = repo.delete_test(id)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let ctx = LogCtx::new(0);
                let _ = Logger::deleted(conn.as_mut(), &ctx, &removed);
            }
            Ok::<_, RepoError>(())
        })
        .await?;
    Ok(Status::NoContent)
}

#[post("/tests/<id>/field", data = "<update>")]
pub async fn update_field(
    id: i32,
    state: &State<AppState>,
    update: Json<FieldUpdateRequest>,
) -> ApiResult<Value> {
    let update = update.into_inner();

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
                let ctx = LogCtx::new(0);
                let _ = Logger::updated(conn.as_mut(), &ctx, &original, &test);
            }

            Ok::<_, RepoError>(())
        })
        .await?;

    Ok(json!({
        "success": true,
        "message": "Field updated successfully"
    }))
}

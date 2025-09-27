use rocket::serde::{Deserialize, Serialize};

use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{EntityType, NewTest, Test};
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
    let tests = state
        .repo
        .db_read(|repo| repo.get_tests_all())
        .await
        .map_err(ApiError::from)?;
    Ok(Json(tests))
}

#[get("/tests/<id>")]
pub async fn get(id: i32, state: &State<AppState>) -> ApiResult<Json<Test>> {
    let test = state
        .repo
        .db_read(move |repo| repo.get_test_by_id(id))
        .await
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("test {id} not found")),
            other => other.into(),
        })?;
    Ok(Json(test))
}

#[post("/tests", data = "<payload>")]
pub async fn create(state: &State<AppState>, payload: Json<NewTest>) -> ApiResult<Value> {
    let test = payload.into_inner();
    let name = test.test_name.clone();
    let project_id = test.project_id;
    let new_values = Logger::to_json_string(&test).ok();

    let id = state
        .repo
        .db_write(move |repo| repo.insert_test(&test))
        .await
        .map_err(|err| match err {
            RepoError::Db(e) => ApiError::BadRequest(format!("failed to create test: {e}")),
            other => other.into(),
        })?;

    let _ = state
        .repo
        .db_read(move |repo| {
            let mut conn = repo.inner_repo().get_conn()?;
            if let Some(payload) = new_values {
                let _ = Logger::log_create(
                    conn.as_mut(),
                    0,
                    EntityType::Test,
                    id,
                    Some(project_id),
                    Some(payload),
                    Some(format!("Created test via API: {name}")),
                    None,
                );
            }
            Ok::<(), RepoError>(())
        })
        .await;

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/tests/<id>")]
pub async fn delete(id: i32, state: &State<AppState>) -> ApiResult<Status> {
    let removed = state
        .repo
        .db_write(move |repo| repo.delete_test(id))
        .await
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("test {id} not found")),
            other => other.into(),
        })?;

    let removed_for_log = removed.clone();
    let _ = state
        .repo
        .db_read(move |repo| {
            let mut conn = repo.inner_repo().get_conn()?;
            if let Ok(old_values) = Logger::to_json_string(&removed_for_log) {
                let _ = Logger::log_delete(
                    conn.as_mut(),
                    0,
                    EntityType::Test,
                    id,
                    Some(removed_for_log.project_id),
                    Some(old_values),
                    Some(format!("Deleted test via API: {}", removed_for_log.test_name)),
                    None,
                );
            }
            Ok::<(), RepoError>(())
        })
        .await;

    Ok(Status::NoContent)
}

#[post("/tests/<id>/field", data = "<update>")]
pub async fn update_field(
    id: i32,
    state: &State<AppState>,
    update: Json<FieldUpdateRequest>,
) -> ApiResult<Value> {
    let update = update.into_inner();

    let mut test = state
        .repo
        .db_read(move |repo| repo.get_test_by_id(id))
        .await
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("test {id} not found")),
            other => other.into(),
        })?;
    let original = test.clone();

    match update.field.as_str() {
        "test_name" => test.test_name = update.value,
        "test_description" => test.test_description = update.value,
        "test_source" => test.test_source = update.value,
        "test_status" => {
            test.test_status = update
                .value
                .parse()
                .map_err(|_| ApiError::BadRequest("invalid status id".into()))?;
        }
        "test_reference" => test.test_reference = update.value,
        "test_parent" => {
            test.test_parent = update
                .value
                .parse()
                .map_err(|_| ApiError::BadRequest("invalid parent id".into()))?;
        }
        other => return Err(ApiError::BadRequest(format!("unsupported field '{other}'"))),
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

    state
        .repo
        .db_write(move |repo| repo.edit_test(&payload))
        .await
        .map_err(ApiError::from)?;

    let original_for_log = original.clone();
    let test_for_log = test.clone();
    let _ = state
        .repo
        .db_read(move |repo| {
            let mut conn = repo.inner_repo().get_conn()?;
            if let (Ok(old_values), Ok(new_values)) =
                (Logger::to_json_string(&original_for_log), Logger::to_json_string(&test_for_log))
            {
                let _ = Logger::log_update(
                    conn.as_mut(),
                    0,
                    EntityType::Test,
                    test_for_log.test_id,
                    Some(test_for_log.project_id),
                    Some(old_values),
                    Some(new_values),
                    Some("Updated test via API".into()),
                    None,
                );
            }
            Ok::<(), RepoError>(())
        })
        .await;

    Ok(json!({
        "success": true,
        "message": "Field updated successfully"
    }))
}

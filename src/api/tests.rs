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
pub fn list(state: &State<AppState>) -> ApiResult<Json<Vec<Test>>> {
    state
        .repo_read()
        .get_tests_all()
        .map(Json)
        .map_err(ApiError::from)
}

#[get("/tests/<id>")]
pub fn get(state: &State<AppState>, id: i32) -> ApiResult<Json<Test>> {
    state
        .repo_read()
        .get_test_by_id(id)
        .map(Json)
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("test {id} not found")),
            other => other.into(),
        })
}

#[post("/tests", data = "<payload>")]
pub fn create(state: &State<AppState>, payload: Json<NewTest>) -> ApiResult<Value> {
    let test = payload.into_inner();
    let name = test.test_name.clone();
    let project_id = test.project_id;

    let id = state
        .repo_write()
        .insert_test(&test)
        .map_err(|err| match err {
            RepoError::Db(e) => ApiError::BadRequest(format!("failed to create test: {e}")),
            other => other.into(),
        })?;

    if let Ok(mut conn) = state.repo_read().inner_repo().get_conn() {
        if let Ok(new_values) = Logger::to_json_string(&test) {
            if let Err(err) = Logger::log_create(
                conn.as_mut(),
                0,
                EntityType::Test,
                id,
                Some(project_id),
                Some(new_values),
                Some(format!("Created test via API: {name}")),
                None,
            ) {
                eprintln!("failed to record test creation log: {err}");
            }
        }
    }

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/tests/<id>")]
pub fn delete(state: &State<AppState>, id: i32) -> ApiResult<Status> {
    let test = state
        .repo_write()
        .delete_test(id)
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("test {id} not found")),
            other => other.into(),
        })?;

    if let (Ok(old_values), Ok(mut conn)) = (
        Logger::to_json_string(&test),
        state.repo_read().inner_repo().get_conn(),
    ) {
        if let Err(err) = Logger::log_delete(
            conn.as_mut(),
            0,
            EntityType::Test,
            id,
            Some(test.project_id),
            Some(old_values),
            Some(format!("Deleted test via API: {}", test.test_name)),
            None,
        ) {
            eprintln!("failed to record test deletion log: {err}");
        }
    }

    Ok(Status::NoContent)
}

#[post("/tests/<id>/field", data = "<update>")]
pub fn update_field(
    state: &State<AppState>,
    id: i32,
    update: Json<FieldUpdateRequest>,
) -> ApiResult<Value> {
    let update = update.into_inner();
    let mut test = state
        .repo_read()
        .get_test_by_id(id)
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
        other => {
            return Err(ApiError::BadRequest(format!("unsupported field '{other}'")));
        }
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
        .repo_write()
        .edit_test(&payload)
        .map_err(ApiError::from)?;

    if let (Ok(old_values), Ok(new_values), Ok(mut conn)) = (
        Logger::to_json_string(&original),
        Logger::to_json_string(&test),
        state.repo_read().inner_repo().get_conn(),
    ) {
        if let Err(err) = Logger::log_update(
            conn.as_mut(),
            0,
            EntityType::Test,
            test.test_id,
            Some(test.project_id),
            Some(old_values),
            Some(new_values),
            Some("Updated test via API".into()),
            None,
        ) {
            eprintln!("failed to record test update log: {err}");
        }
    }

    Ok(json!({
        "success": true,
        "message": "Field updated successfully"
    }))
}

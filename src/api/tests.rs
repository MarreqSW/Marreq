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
    let name = test.test_name.clone();
    let project_id = test.project_id;
    let new_values = Logger::to_json_string(&test).ok();

    let id = state
        .repo
        .async_write(move |repo| {
            let id = repo.insert_test(&test)?;
            if let (Some(payload), Ok(mut conn)) = (new_values, repo.inner_repo().get_conn()) {
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
                if let Ok(old_values) = Logger::to_json_string(&removed) {
                    let _ = Logger::log_delete(
                        conn.as_mut(),
                        0,
                        EntityType::Test,
                        id,
                        Some(removed.project_id),
                        Some(old_values),
                        Some(format!("Deleted test via API: {}", removed.test_name)),
                        None,
                    );
                }
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
                        .map_err(|_| RepoError::BadRequest("invalid status id".into()))?;
                }
                "test_reference" => test.test_reference = update.value,
                "test_parent" => {
                    test.test_parent = update
                        .value
                        .parse()
                        .map_err(|_| RepoError::BadRequest("invalid parent id".into()))?;
                }
                other => return Err(RepoError::BadRequest(format!("unsupported field '{other}'"))),
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
                if let (Ok(old_values), Ok(new_values)) =
                    (Logger::to_json_string(&original), Logger::to_json_string(&test))
                {
                    let _ = Logger::log_update(
                        conn.as_mut(),
                        0,
                        EntityType::Test,
                        test.test_id,
                        Some(test.project_id),
                        Some(old_values),
                        Some(new_values),
                        Some("Updated test via API".into()),
                        None,
                    );
                }
            }

            Ok::<_, RepoError>(())
        })
        .await?;

    Ok(json!({
        "success": true,
        "message": "Field updated successfully"
    }))
}

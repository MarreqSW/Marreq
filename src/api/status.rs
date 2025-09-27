use crate::api::prelude::*;
use crate::models::{NewStatus, RequirementStatus, Status as LegacyStatus};
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;

#[get("/status")]
pub async fn list(state: &State<AppState>) -> ApiResult<Json<Vec<LegacyStatus>>> {
    let statuses = state
        .repo
        .db_read(|repo| repo.get_requirement_status_all())
        .await
        .map_err(ApiError::from)?
        .into_iter()
        .map(|status: RequirementStatus| LegacyStatus {
            st_id: status.req_st_id,
            st_title: status.req_st_title,
            st_description: status.req_st_description,
            st_short_name: status.req_st_short_name,
        })
        .collect();
    Ok(Json(statuses))
}

#[get("/status/<id>")]
pub async fn get(id: i32, state: &State<AppState>) -> ApiResult<Json<Value>> {
    let status = state
        .repo
        .db_read(move |repo| repo.get_requirement_status_by_id(id))
        .await
        .map_err(|err| match err {
            RepoError::NotFound => ApiError::NotFound(format!("status {id} not found")),
            other => other.into(),
        })?;

    Ok(Json(json!({
        "id": status.req_st_id,
        "title": status.req_st_title,
        "description": status.req_st_description,
        "short_name": status.req_st_short_name,
    })))
}

#[post("/status", data = "<payload>")]
pub async fn create(
    state: &State<AppState>,
    payload: Json<NewStatus>,
) -> ApiResult<(Status, Value)> {
    let status = payload.into_inner();
    let id = state
        .repo
        .db_write(move |repo| repo.create_status(&status))
        .await
        .map_err(|err| match err {
            RepoError::Db(e) => ApiError::BadRequest(format!("failed to create status: {e}")),
            other => other.into(),
        })?;

    Ok((
        Status::Created,
        json!({ "status": "ok", "id": id }),
    ))
}

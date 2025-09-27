use crate::api::prelude::*;
use crate::models::{NewStatus, RequirementStatus, Status as LegacyStatus};
use crate::repository::errors::RepoError;
use crate::repository::{DieselCachedRepo, LookupRepository};

#[get("/status")]
pub fn list() -> ApiResult<Json<Vec<LegacyStatus>>> {
    let statuses = DieselCachedRepo::read()
        .get_requirement_status_all()
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
pub fn get(id: i32) -> ApiResult<Json<Value>> {
    let status = DieselCachedRepo::read()
        .get_requirement_status_by_id(id)
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
pub fn create(payload: Json<NewStatus>) -> ApiResult<(Status, Value)> {
    let status = payload.into_inner();
    let id = DieselCachedRepo::write()
        .create_status(&status)
        .map_err(|err| match err {
            RepoError::Db(e) => ApiError::BadRequest(format!("failed to create status: {e}")),
            other => other.into(),
        })?;

    Ok((
        Status::Created,
        json!({
            "status": "ok",
            "id": id,
        }),
    ))
}

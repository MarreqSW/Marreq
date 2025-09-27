use diesel::prelude::*;

use crate::api::prelude::*;
use crate::models::Matrix;
use crate::repository::DieselCachedRepo;

#[get("/matrix")]
pub fn list() -> ApiResult<Json<Vec<Matrix>>> {
    use crate::schema::matrix::dsl::matrix;

    let mut conn = DieselCachedRepo::read()
        .inner_repo()
        .get_conn()
        .map_err(ApiError::from)?;

    let entries = matrix
        .load::<Matrix>(conn.as_mut())
        .map_err(|err| ApiError::Internal(format!("failed to load matrix: {err}")))?;

    Ok(Json(entries))
}

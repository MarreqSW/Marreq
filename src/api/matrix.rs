use diesel::prelude::*;

use crate::api::prelude::*;
use crate::models::Matrix;

#[get("/matrix")]
pub async fn list(state: &State<AppState>) -> ApiResult<Json<Vec<Matrix>>> {
    use crate::schema::matrix::dsl::matrix;

    let mut conn = state
        .repo
        .clone()
        .async_read(|repo| repo.inner_repo().get_conn())
        .await?;

    let entries = matrix
        .load::<Matrix>(conn.as_mut())
        .map_err(|err| ApiError::Internal(format!("failed to load matrix: {err}")))?;

    Ok(Json(entries))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::repository::{fake_repo::FakeRepo, CacheRepository};
    use rocket::local::asynchronous::Client;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<FakeRepo>>;

    fn test_state() -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(FakeRepo::default(), 0))),
        }
    }

    async fn test_client() -> Client {
        let rocket = rocket::build()
            .manage(test_state())
            .mount("/api", routes![list]);
        Client::tracked(rocket).await.unwrap()
    }

    #[rocket::async_test]
    async fn list_returns_error_without_database() {
        let client = test_client().await;
        let response = client.get("/api/matrix").dispatch().await;
        assert_eq!(response.status(), Status::InternalServerError);
    }
}

use crate::api::prelude::*;
use crate::models::Matrix;
use crate::services::MatrixService;

#[get("/matrix")]
pub async fn list(state: &State<AppState>) -> ApiResult<Json<Vec<Matrix>>> {
    let service = MatrixService::new(state.inner());
    let entries = service.list_all()?;
    Ok(Json(entries))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::local::asynchronous::Client;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    fn test_state() -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(
                DieselRepoMock::default(),
                0,
            ))),
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

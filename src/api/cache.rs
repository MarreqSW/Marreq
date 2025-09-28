use chrono::Utc;

use crate::api::prelude::*;
use crate::repository::errors::RepoError;

#[get("/cache/stats")]
pub async fn stats(
    state: &State<AppState>,
) -> ApiResult<Json<crate::repository::cache::stats::CacheStats>> {
    let stats = state
        .repo
        .async_read(|repo| Ok::<_, RepoError>(repo.cache().stats()))
        .await?;
    Ok(Json(stats))
}

#[post("/cache/clear")]
pub async fn clear(state: &State<AppState>) -> ApiResult<Json<Value>> {
    state
        .repo
        .async_write(|repo| {
            repo.cache().clear();
            Ok::<_, RepoError>(())
        })
        .await?;
    Ok(Json(json!({
        "message": "Cache cleared successfully",
        "timestamp": Utc::now().to_rfc3339(),
    })))
}

#[post("/cache/cleanup")]
pub async fn cleanup(state: &State<AppState>) -> ApiResult<Json<Value>> {
    let cleaned = state
        .repo
        .async_write(|repo| Ok::<_, RepoError>(repo.cache().cleanup()))
        .await?;
    Ok(Json(json!({
        "message": format!("Cleaned up {} expired entries", cleaned),
        "cleaned_entries": cleaned,
        "timestamp": Utc::now().to_rfc3339(),
    })))
}

#[get("/cache/performance")]
pub async fn performance(state: &State<AppState>) -> ApiResult<Json<Value>> {
    let performance = state
        .repo
        .async_read(|repo| Ok::<_, RepoError>(repo.cache().get_performance()))
        .await?;
    Ok(Json(performance))
}

#[get("/cache/recommendations")]
pub async fn recommendations(state: &State<AppState>) -> ApiResult<Json<Value>> {
    let recommendations = state
        .repo
        .async_read(|repo| Ok::<_, RepoError>(repo.cache().get_recommendations()))
        .await?;
    Ok(Json(recommendations))
}

#[post("/cache/reset-counters")]
pub async fn reset_counters(state: &State<AppState>) -> ApiResult<Json<Value>> {
    state
        .repo
        .async_write(|repo| {
            repo.cache().reset_counters();
            Ok::<_, RepoError>(())
        })
        .await?;
    Ok(Json(json!({
        "message": "Cache performance counters reset successfully",
        "timestamp": Utc::now().to_rfc3339(),
    })))
}

#[get("/cache/health")]
pub async fn health(state: &State<AppState>) -> ApiResult<Json<Value>> {
    let health = state
        .repo
        .async_read(|repo| Ok::<_, RepoError>(repo.cache().get_health()))
        .await?;
    Ok(Json(health))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::ContentType;
    use rocket::local::asynchronous::Client;
    use serde_json::Value;
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
        let rocket = rocket::build().manage(test_state()).mount(
            "/api",
            routes![
                stats,
                clear,
                cleanup,
                performance,
                recommendations,
                reset_counters,
                health
            ],
        );
        Client::tracked(rocket).await.unwrap()
    }

    #[rocket::async_test]
    async fn stats_returns_default_values() {
        let client = test_client().await;
        let response = client.get("/api/cache/stats").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let stats = response.into_json::<Value>().await.unwrap();
        assert!(stats.get("hits").is_some());
    }

    #[rocket::async_test]
    async fn clear_returns_confirmation() {
        let client = test_client().await;
        let response = client
            .post("/api/cache/clear")
            .header(ContentType::JSON)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let value = response.into_json::<Value>().await.unwrap();
        assert_eq!(
            value.get("message"),
            Some(&Value::from("Cache cleared successfully"))
        );
    }

    #[rocket::async_test]
    async fn cleanup_returns_zero_entries() {
        let client = test_client().await;
        let response = client
            .post("/api/cache/cleanup")
            .header(ContentType::JSON)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let value = response.into_json::<Value>().await.unwrap();
        assert_eq!(value.get("cleaned_entries"), Some(&Value::from(0)));
    }

    #[rocket::async_test]
    async fn reset_counters_reports_success() {
        let client = test_client().await;
        let response = client
            .post("/api/cache/reset-counters")
            .header(ContentType::JSON)
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let value = response.into_json::<Value>().await.unwrap();
        assert_eq!(
            value.get("message"),
            Some(&Value::from(
                "Cache performance counters reset successfully"
            ))
        );
    }

    #[rocket::async_test]
    async fn performance_and_health_endpoints_work() {
        let client = test_client().await;
        let performance = client.get("/api/cache/performance").dispatch().await;
        assert_eq!(performance.status(), Status::Ok);
        assert!(performance.into_json::<Value>().await.unwrap().is_object());

        let health = client.get("/api/cache/health").dispatch().await;
        assert_eq!(health.status(), Status::Ok);
        assert!(health.into_json::<Value>().await.unwrap().is_object());
    }
}

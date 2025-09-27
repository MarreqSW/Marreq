use chrono::Utc;

use crate::api::prelude::*;
use crate::repository::errors::RepoError;

#[get("/cache/stats")]
pub async fn stats(state: &State<AppState>) -> ApiResult<Json<crate::repository::cache::stats::CacheStats>> {
    let stats = state
        .repo
        .async_read(|repo| {
            Ok::<_, RepoError>(repo.cache().stats())
        })
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


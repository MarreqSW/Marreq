use chrono::Utc;

use crate::api::prelude::*;

#[get("/cache/stats")]
pub fn stats(state: &State<AppState>) -> ApiResult<Json<Value>> {
    let stats = state.repo_read().cache().stats();
    Ok(Json(json!({
        "total_entries": stats.total_entries,
        "active_entries": stats.active_entries,
        "expired_entries": stats.expired_entries,
    })))
}

#[post("/cache/clear")]
pub fn clear(state: &State<AppState>) -> ApiResult<Json<Value>> {
    state.repo_read().cache().clear();
    Ok(Json(json!({
        "message": "Cache cleared successfully",
        "timestamp": Utc::now().to_rfc3339(),
    })))
}

#[post("/cache/cleanup")]
pub fn cleanup(state: &State<AppState>) -> ApiResult<Json<Value>> {
    let cache = state.repo_read().cache();
    let cleaned = cache.cleanup();

    Ok(Json(json!({
        "message": format!("Cleaned up {} expired entries", cleaned),
        "cleaned_entries": cleaned,
        "timestamp": Utc::now().to_rfc3339(),
    })))
}

#[get("/cache/performance")]
pub fn performance(state: &State<AppState>) -> ApiResult<Json<Value>> {
    let performance = state.repo_read().cache().get_performance();
    Ok(Json(performance))
}

#[get("/cache/recommendations")]
pub fn recommendations(state: &State<AppState>) -> ApiResult<Json<Value>> {
    let recommendations = state.repo_read().cache().get_recommendations();
    Ok(Json(recommendations))
}

#[post("/cache/reset-counters")]
pub fn reset_counters(state: &State<AppState>) -> ApiResult<Json<Value>> {
    state.repo_read().cache().reset_counters();
    Ok(Json(json!({
        "message": "Cache performance counters reset successfully",
        "timestamp": Utc::now().to_rfc3339(),
    })))
}

#[get("/cache/health")]
pub fn health(state: &State<AppState>) -> ApiResult<Json<Value>> {
    let health = state.repo_read().cache().get_health();
    Ok(Json(health))
}

use chrono::Utc;

use crate::api::prelude::*;
use crate::repository::DieselCachedRepo;

#[get("/cache/stats")]
pub fn stats() -> ApiResult<Json<Value>> {
    let stats = DieselCachedRepo::read().cache().stats();
    Ok(Json(json!({
        "total_entries": stats.total_entries,
        "active_entries": stats.active_entries,
        "expired_entries": stats.expired_entries,
    })))
}

#[post("/cache/clear")]
pub fn clear() -> ApiResult<Json<Value>> {
    DieselCachedRepo::read().cache().clear();
    Ok(Json(json!({
        "message": "Cache cleared successfully",
        "timestamp": Utc::now().to_rfc3339(),
    })))
}

#[post("/cache/cleanup")]
pub fn cleanup() -> ApiResult<Json<Value>> {
    let cache = DieselCachedRepo::read().cache();
    let cleaned = cache.cleanup();

    Ok(Json(json!({
        "message": format!("Cleaned up {} expired entries", cleaned),
        "cleaned_entries": cleaned,
        "timestamp": Utc::now().to_rfc3339(),
    })))
}

#[get("/cache/performance")]
pub fn performance() -> ApiResult<Json<Value>> {
    let performance = DieselCachedRepo::read().cache().get_performance();
    Ok(Json(performance))
}

#[get("/cache/recommendations")]
pub fn recommendations() -> ApiResult<Json<Value>> {
    let recommendations = DieselCachedRepo::read().cache().get_recommendations();
    Ok(Json(recommendations))
}

#[post("/cache/reset-counters")]
pub fn reset_counters() -> ApiResult<Json<Value>> {
    DieselCachedRepo::read().cache().reset_counters();
    Ok(Json(json!({
        "message": "Cache performance counters reset successfully",
        "timestamp": Utc::now().to_rfc3339(),
    })))
}

#[get("/cache/health")]
pub fn health() -> ApiResult<Json<Value>> {
    let health = DieselCachedRepo::read().cache().get_health();
    Ok(Json(health))
}

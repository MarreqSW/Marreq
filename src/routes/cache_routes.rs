use crate::repository::DieselCachedRepo;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket::serde::json::json;
use rocket::serde::json::Json;
use rocket_dyn_templates::Template;

use crate::routes::routes_html::require_auth;

/// Show cache statistics page
#[get("/admin/cache")]
pub fn cache_stats_page() -> Template {
    let cache = DieselCachedRepo::read().cache();
    let stats = cache.stats();
    let cleaned = cache.cleanup();

    // Get performance metrics
    let performance = DieselCachedRepo::read().cache().get_performance();
    let recommendations = DieselCachedRepo::read().cache().get_recommendations();

    let ctx = json!({
        "title": "Cache Statistics",
        "stats": stats,
        "cleaned_entries": cleaned,
        "performance": performance,
        "recommendations": recommendations
    });

    Template::render("admin/cache_stats", ctx)
}

/// Clear all cache entries
#[post("/admin/cache/clear")]
pub fn clear_cache() -> Template {
    DieselCachedRepo::read().cache().clear();

    let stats = DieselCachedRepo::read().cache().stats();

    let ctx = json!({
        "title": "Cache Statistics",
        "stats": stats,
        "cleaned_entries": 0,
        "message": "All cache entries have been cleared"
    });

    Template::render("admin/cache_stats", ctx)
}

/// Clean up expired cache entries
#[post("/admin/cache/cleanup")]
pub fn cleanup_cache() -> Template {
    let cache = DieselCachedRepo::read().cache();
    let cleaned = cache.cleanup();
    let stats = cache.stats();

    let ctx = json!({
        "title": "Cache Statistics",
        "stats": stats,
        "cleaned_entries": cleaned,
        "message": format!("Cleaned up {} expired cache entries", cleaned)
    });

    Template::render("admin/cache_stats", ctx)
}

/// API endpoint to get cache statistics
#[get("/api/v1/cache/stats")]
pub fn api_cache_stats() -> rocket::serde::json::Json<serde_json::Value> {
    let stats = DieselCachedRepo::read().cache().stats();

    rocket::serde::json::Json(json!({
        "total_entries": stats.total_entries,
        "active_entries": stats.active_entries,
        "expired_entries": stats.expired_entries
    }))
}

/// API endpoint to clear all cache
#[post("/api/v1/cache/clear")]
pub fn api_clear_cache() -> rocket::serde::json::Json<serde_json::Value> {
    DieselCachedRepo::read().cache().clear();

    rocket::serde::json::Json(json!({
        "message": "Cache cleared successfully",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// API endpoint to cleanup expired entries
#[post("/api/v1/cache/cleanup")]
pub fn api_cleanup_cache() -> rocket::serde::json::Json<serde_json::Value> {
    let cache = DieselCachedRepo::read().cache();
    let cleaned = cache.cleanup();

    rocket::serde::json::Json(json!({
        "message": format!("Cleaned up {} expired entries", cleaned),
        "cleaned_entries": cleaned,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// API endpoint to get cache performance metrics
#[get("/api/v1/cache/performance")]
pub fn api_cache_performance() -> rocket::serde::json::Json<serde_json::Value> {
    let performance = DieselCachedRepo::read().cache().get_performance();
    rocket::serde::json::Json(performance)
}

/// API endpoint to get cache optimization recommendations
#[get("/api/v1/cache/recommendations")]
pub fn api_cache_recommendations() -> rocket::serde::json::Json<serde_json::Value> {
    let recommendations = DieselCachedRepo::read().cache().get_recommendations();
    rocket::serde::json::Json(recommendations)
}

/// API endpoint to reset cache performance counters
#[post("/api/v1/cache/reset-counters")]
pub fn api_reset_cache_counters() -> rocket::serde::json::Json<serde_json::Value> {
    DieselCachedRepo::read().cache().reset_counters();

    rocket::serde::json::Json(json!({
        "message": "Cache performance counters reset successfully",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// API endpoint to get cache health
#[get("/cache/health")]
pub fn cache_health_page(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;

    if !user.is_admin {
        return Err(Redirect::to("/"));
    }

    let health_data = DieselCachedRepo::read().cache().get_health();
    let ctx = json!({
        "user": user,
        "health": health_data
    });

    Ok(Template::render("admin/cache_health", ctx))
}

#[get("/cache/warm")]
pub fn warm_cache_route(cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let user = require_auth(cookies)?;

    if !user.is_admin {
        return Err(Redirect::to("/"));
    }

    // Warm up the cache
    DieselCachedRepo::write().warm_cache();

    Ok(Redirect::to("/admin/cache"))
}

#[get("/cache/health/api")]
pub fn api_cache_health() -> Json<serde_json::Value> {
    let health_data = DieselCachedRepo::read().cache().get_health();
    Json(health_data)
}

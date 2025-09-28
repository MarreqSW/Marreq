use crate::app::AppState;
use crate::auth::AdminOnly;
use rocket::response::Redirect;
use rocket::serde::json::json;
use rocket::State;
use rocket_dyn_templates::Template;

/// Show cache statistics page
#[get("/admin/cache")]
pub fn cache_stats_page(state: &State<AppState>) -> Template {
    let repo = state.repo_read();
    let cache = repo.cache();
    let stats = cache.stats();
    let cleaned = cache.cleanup();
    let performance = cache.get_performance();
    let recommendations = cache.get_recommendations();

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
pub fn clear_cache(state: &State<AppState>) -> Template {
    let repo = state.repo_read();
    let cache = repo.cache();
    cache.clear();

    let stats = cache.stats();

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
pub fn cleanup_cache(state: &State<AppState>) -> Template {
    let repo = state.repo_read();
    let cache = repo.cache();
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

/// Cache health page for administrators
#[get("/cache/health")]
pub fn cache_health_page(admin: AdminOnly, state: &State<AppState>) -> Template {
    let user = admin.into_inner();

    let repo = state.repo_read();
    let health_data = repo.cache().get_health();
    let ctx = json!({
        "user": user,
        "health": health_data
    });

    Template::render("admin/cache_health", ctx)
}

#[get("/cache/warm")]
pub fn warm_cache_route(_admin: AdminOnly, state: &State<AppState>) -> Redirect {
    state.repo_write().warm_cache();

    Redirect::to("/admin/cache")
}

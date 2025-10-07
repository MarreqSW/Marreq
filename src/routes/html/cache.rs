use crate::app::AppState;
use crate::auth::AdminOnly;
use rocket::response::Redirect;
use rocket::serde::json::json;
use rocket::State;
use rocket_dyn_templates::Template;
use crate::services::CacheService;

/// Show cache statistics page
#[get("/admin/cache")]
pub fn cache_stats_page(state: &State<AppState>) -> Template {
    let service = CacheService::new(state);

    let ctx = json!({
        "title": "Cache Statistics",
        "stats": service.stats(),
        "cleaned_entries": service.cleanup(),
        "performance": service.performance(),
        "recommendations": service.recommendations()
    });

    Template::render("admin/cache_stats", ctx)
}

/// Clear all cache entries
#[post("/admin/cache/clear")]
pub fn clear_cache(state: &State<AppState>) -> Template {
    let service = CacheService::new(state);
    service.clear();

    let ctx = json!({
        "title": "Cache Statistics",
        "stats": service.stats(),
        "cleaned_entries": 0,
        "message": "All cache entries have been cleared"
    });

    Template::render("admin/cache_stats", ctx)
}

/// Clean up expired cache entries
#[post("/admin/cache/cleanup")]
pub fn cleanup_cache(state: &State<AppState>) -> Template {
    let service = CacheService::new(state);
    let cleaned = service.cleanup();

    let ctx = json!({
        "title": "Cache Statistics",
        "stats": service.stats(),
        "cleaned_entries": cleaned,
        "message": format!("Cleaned up {} expired cache entries", cleaned)
    });

    Template::render("admin/cache_stats", ctx)
}

/// Cache health page for administrators
#[get("/cache/health")]
pub fn cache_health_page(admin: AdminOnly, state: &State<AppState>) -> Template {
    let service = CacheService::new(state);
    let ctx = json!({
        "user": admin.into_inner(),
        "health": service.health()
    });

    Template::render("admin/cache_health", ctx)
}

#[get("/cache/warm")]
pub fn warm_cache_route(_admin: AdminOnly, state: &State<AppState>) -> Redirect {
    CacheService::new(state).warm_cache();
    Redirect::to("/admin/cache")
}

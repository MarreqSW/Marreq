use crate::app::AppState;
use crate::auth::AdminOnly;
use crate::services::CacheService;
use rocket::response::Redirect;
use rocket::serde::json::json;
use rocket::State;
use rocket_dyn_templates::Template;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::AdminOnly;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::repository::CacheRepository;
    use rocket::State;
    use std::sync::{Arc, RwLock};

    fn admin_user() -> crate::models::User {
        let mut user = DieselRepoMock::make_user(1, "admin", "");
        user.is_admin = true;
        user
    }

    fn app_state() -> AppState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(
                DieselRepoMock::default(),
                60,
            ))),
        }
    }

    fn state_guard(state: &AppState) -> &State<AppState> {
        State::from(state)
    }

    #[test]
    fn cache_stats_page_returns_cache_statistics_template() {
        let state = app_state();
        {
            let cache = state.repo_read().cache();
            cache.set("foo", "bar".into());
        }

        let template = cache_stats_page(state_guard(&state));
        let rendered = format!("{:?}", template);

        assert!(rendered.contains("admin/cache_stats"));

        let stats = state.repo_read().cache().stats();
        assert!(stats.total_entries >= 1);
        assert!(stats.active_entries >= 1);
    }

    #[test]
    fn clear_cache_route_clears_entries_and_sets_message() {
        let state = app_state();
        {
            let cache = state.repo_read().cache();
            cache.set("foo", "bar".into());
        }

        let template = clear_cache(state_guard(&state));
        let rendered = format!("{:?}", template);
        assert!(rendered.contains("All cache entries have been cleared"));

        let stats = state.repo_read().cache().stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.active_entries, 0);
    }

    #[test]
    fn cleanup_cache_route_reports_cleaned_count() {
        let state = app_state();

        let template = cleanup_cache(state_guard(&state));
        let rendered = format!("{:?}", template);
        assert!(rendered.contains("Cleaned up 0 expired cache entries"));
    }

    #[test]
    fn cache_health_page_includes_user_and_health_data() {
        let state = app_state();
        let admin = AdminOnly(admin_user());

        let template = cache_health_page(admin, state_guard(&state));
        let rendered = format!("{:?}", template);
        assert!(rendered.contains("admin/cache_health"));
        assert!(rendered.contains("\"user\""));
        assert!(rendered.contains("\"health\""));
    }

    #[test]
    fn warm_cache_route_redirects_to_cache_stats() {
        let state = app_state();
        let redirect = warm_cache_route(AdminOnly(admin_user()), state_guard(&state));
        let description = format!("{:?}", redirect);

        assert!(description.contains("/admin/cache"));
    }
}

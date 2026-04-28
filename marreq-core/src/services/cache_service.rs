// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::app::{AppState, DieselCachedRepo};
use crate::repository::cache::{Cache, CacheStats};
use serde_json::Value;
use std::sync::Arc;

/// Service encapsulating cache operations for use by route handlers.
pub struct CacheService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> CacheService<'a> {
    /// Create a new cache service bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    fn cache(&self) -> Arc<Cache> {
        self.state.repo_read().cache()
    }

    /// Retrieve cache statistics snapshot.
    pub fn stats(&self) -> CacheStats {
        self.cache().stats()
    }

    /// Clear all cache entries.
    pub fn clear(&self) {
        self.cache().clear();
    }

    /// Clean up expired cache entries, returning the number of removed items.
    pub fn cleanup(&self) -> usize {
        self.cache().cleanup()
    }

    /// Retrieve cache performance metrics.
    pub fn performance(&self) -> Value {
        self.cache().get_performance()
    }

    /// Retrieve cache optimisation recommendations.
    pub fn recommendations(&self) -> Value {
        self.cache().get_recommendations()
    }

    /// Retrieve cache health information.
    pub fn health(&self) -> Value {
        self.cache().get_health()
    }

    /// Warm the cached repository with common queries.
    pub fn warm_cache(&self) {
        self.state.repo_write().warm_cache();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use std::sync::{Arc, RwLock};
    use std::time::Duration;

    fn state_with_repo() -> AppState<DieselCachedRepo> {
        AppState {
            repo: Arc::new(RwLock::new(DieselCachedRepo::new(
                DieselRepoMock::default(),
                1,
            ))),
        }
    }

    #[test]
    fn stats_reflect_cache_entries() {
        let state = state_with_repo();
        let service = CacheService::new(&state);

        {
            let cache = state.repo_read().cache();
            cache.set("foo", "bar".into());
        }

        let stats = service.stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.active_entries, 1);
    }

    #[test]
    fn clear_resets_cache() {
        let state = state_with_repo();
        let service = CacheService::new(&state);

        {
            let cache = state.repo_read().cache();
            cache.set("foo", "bar".into());
        }

        service.clear();

        let stats = service.stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.active_entries, 0);
    }

    #[test]
    fn cleanup_reports_zero_when_no_entries_expired() {
        let state = state_with_repo();
        let service = CacheService::new(&state);

        {
            let cache = state.repo_read().cache();
            cache.set_with_ttl("foo", "bar".into(), Duration::from_secs(60));
        }

        let cleaned = service.cleanup();
        assert_eq!(cleaned, 0);

        let stats = service.stats();
        assert_eq!(stats.total_entries, 1);
    }

    #[test]
    fn exposes_performance_and_health_information() {
        let state = state_with_repo();
        let service = CacheService::new(&state);

        let performance = service.performance();
        assert!(performance.is_object());

        let recommendations = service.recommendations();
        assert!(recommendations.is_object());

        let health = service.health();
        assert!(health.is_object());
    }

    #[test]
    fn warm_cache_delegates_to_repository() {
        let state = state_with_repo();
        let service = CacheService::new(&state);

        service.warm_cache();
    }

    #[test]
    fn stats_reflects_multiple_entries() {
        let state = state_with_repo();
        let service = CacheService::new(&state);

        {
            let cache = state.repo_read().cache();
            cache.set("key1", "value1".into());
            cache.set("key2", "value2".into());
            cache.set("key3", "value3".into());
        }

        let stats = service.stats();
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.active_entries, 3);
    }

    #[test]
    fn clear_removes_all_entries() {
        let state = state_with_repo();
        let service = CacheService::new(&state);

        {
            let cache = state.repo_read().cache();
            cache.set("key1", "value1".into());
            cache.set("key2", "value2".into());
        }

        service.clear();

        let stats = service.stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.active_entries, 0);
    }

    #[test]
    fn cleanup_removes_expired_entries() {
        let state = state_with_repo();
        let service = CacheService::new(&state);

        {
            let cache = state.repo_read().cache();
            // Set entry with long TTL (should remain active)
            cache.set_with_ttl("active", "value".into(), Duration::from_secs(60));
        }

        // Cleanup should return 0 when no entries are expired
        let cleaned = service.cleanup();
        assert_eq!(cleaned, 0);

        let stats = service.stats();
        // Should have the active entry remaining
        assert_eq!(stats.active_entries, 1);
    }

    #[test]
    fn performance_returns_valid_json() {
        let state = state_with_repo();
        let service = CacheService::new(&state);

        let performance = service.performance();
        assert!(performance.is_object());

        // Should have performance-related fields
        let obj = performance.as_object().unwrap();
        assert!(obj.contains_key("hit_rate") || obj.contains_key("miss_rate") || !obj.is_empty());
    }

    #[test]
    fn recommendations_returns_valid_json() {
        let state = state_with_repo();
        let service = CacheService::new(&state);

        let recommendations = service.recommendations();
        assert!(recommendations.is_object());
    }

    #[test]
    fn health_returns_valid_json() {
        let state = state_with_repo();
        let service = CacheService::new(&state);

        let health = service.health();
        assert!(health.is_object());
    }

    #[test]
    fn stats_initial_state_is_zero() {
        let state = state_with_repo();
        let service = CacheService::new(&state);

        let stats = service.stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.active_entries, 0);
    }

    #[test]
    fn clear_on_empty_cache_is_safe() {
        let state = state_with_repo();
        let service = CacheService::new(&state);

        // Should not panic
        service.clear();

        let stats = service.stats();
        assert_eq!(stats.total_entries, 0);
    }

    #[test]
    fn cleanup_on_empty_cache_returns_zero() {
        let state = state_with_repo();
        let service = CacheService::new(&state);

        let cleaned = service.cleanup();
        assert_eq!(cleaned, 0);
    }
}

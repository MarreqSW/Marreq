use super::keys::{self, Keyspace};
use crate::repository::*;
use chrono;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

/// Cache entry with TTL (Time To Live)
#[derive(Clone)]
pub(super) struct CacheEntry<T> {
    pub(super) data: T,
    pub(super) expires_at: Instant,
}

/// In-memory cache with TTL support
pub struct Cache {
    pub(super) data: Arc<RwLock<HashMap<String, CacheEntry<String>>>>,
    pub(super) default_ttl: Duration,
    pub(super) hits: Arc<AtomicU64>,
    pub(super) misses: Arc<AtomicU64>,
    pub(super) total_access_time: Arc<AtomicU64>,
    pub(super) active_entries: Arc<AtomicU64>,
    pub(super) expired_entries: Arc<AtomicU64>,
    pub(super) last_cleanup: Arc<Mutex<chrono::DateTime<chrono::Utc>>>,
}

enum Status {
    Hit(String),
    Miss,
    Expired,
}

impl Cache {
    /// Create a new cache instance with default TTL
    pub fn new(default_ttl_seconds: u64) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: Duration::from_secs(default_ttl_seconds),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            total_access_time: Arc::new(AtomicU64::new(0)),
            active_entries: Arc::new(AtomicU64::new(0)),
            expired_entries: Arc::new(AtomicU64::new(0)),
            last_cleanup: Arc::new(Mutex::new(chrono::Utc::now())),
        }
    }

    fn read(&self, key: &str) -> Status {
        let data = self.data.read().unwrap();
        match data.get(key) {
            Some(cache) if cache.expires_at > Instant::now() => Status::Hit(cache.data.clone()),
            Some(_) => Status::Expired,
            None => Status::Miss,
        }
    }

    /// Get a value from cache
    pub fn get(&self, key: &str) -> Option<String> {
        let start = Instant::now();

        let result: Option<String>;
        match self.read(key) {
            Status::Hit(value) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                result = Some(value);
            }
            Status::Miss => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                result = None;
            }
            Status::Expired => {
                // Update counters: move from active to expired before removal
                self.active_entries.fetch_sub(1, Ordering::Relaxed);
                self.expired_entries.fetch_add(1, Ordering::Relaxed);
                self.misses.fetch_add(1, Ordering::Relaxed);
                self.remove(key);
                result = None;
            }
        }

        let dt = start.elapsed().as_nanos() as u64;
        self.total_access_time.fetch_add(dt, Ordering::Relaxed);

        result
    }

    /// Set a value in cache with default TTL
    pub fn set(&self, key: &str, value: String) {
        self.set_with_ttl(key, value, self.default_ttl);
    }

    /// Set a value in cache with custom TTL
    pub fn set_with_ttl(&self, key: &str, value: String, ttl: Duration) {
        let entry = CacheEntry {
            data: value,
            expires_at: Instant::now() + ttl,
        };

        let mut data = self.data.write().unwrap();
        // Check if this key already exists to update counters correctly
        let existed = data.contains_key(key);
        data.insert(key.to_string(), entry);

        // Update active entries counter
        if !existed {
            self.active_entries.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Remove a key from cache
    pub fn remove(&self, key: &str) {
        let mut data = self.data.write().unwrap();
        if let Some(entry) = data.remove(key) {
            let now = Instant::now();

            // Always decrement active count first
            self.active_entries
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_sub(1))
                .ok();

            // If expired, also decrement expired count
            if entry.expires_at <= now {
                self.expired_entries
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_sub(1))
                    .ok();
            }
        }
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        let mut data = self.data.write().unwrap();
        data.clear();

        // Reset entry counters
        self.active_entries.store(0, Ordering::Relaxed);
        self.expired_entries.store(0, Ordering::Relaxed);
    }

    /// Reset performance counters
    pub fn reset_counters(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.total_access_time.store(0, Ordering::Relaxed);
        self.active_entries.store(0, Ordering::Relaxed);
        self.expired_entries.store(0, Ordering::Relaxed);
    }

    /// Clean up expired entries and return count of cleaned entries
    pub fn cleanup(&self) -> usize {
        let mut data = self.data.write().unwrap();
        let now = Instant::now();
        let initial_count = data.len();

        // Count expired entries before removal
        let expired_count = data.values().filter(|entry| entry.expires_at <= now).count();

        data.retain(|_, entry| entry.expires_at > now);

        let cleaned_count = initial_count - data.len();

        // Update counters for cleaned entries
        if expired_count > 0 {
            self.expired_entries.fetch_sub(expired_count as u64, Ordering::Relaxed);
        }

        // Update last cleanup time
        let mut last_cleanup = self.last_cleanup.lock().unwrap();
        *last_cleanup = chrono::Utc::now();

        cleaned_count
    }

    /// Sync counters with actual data (useful for maintenance and debugging)
    pub fn sync_counters(&self) {
        let data = self.data.read().unwrap();
        let now = Instant::now();

        let actual_active = data.values().filter(|entry| entry.expires_at > now).count() as u64;
        let actual_expired = data.values().filter(|entry| entry.expires_at <= now).count() as u64;

        self.active_entries.store(actual_active, Ordering::Relaxed);
        self.expired_entries.store(actual_expired, Ordering::Relaxed);
    }
}

impl Cache {
    /// Invalidate all project-related cache entries
    pub fn invalidate_project(&self, project_id: i32) {
        self.remove(&keys::Requirements::by_project(project_id));
        self.remove(&keys::Tests::by_project(project_id));
        self.remove(&keys::Matrix::by_project(project_id));
        self.remove(&keys::Verification::by_project(project_id));
        self.remove(&keys::Categories::by_project(project_id));
        self.remove(&keys::Applicability::by_project(project_id));
        self.remove(&keys::Projects::by_id(project_id));
    }

    /// Invalidate all user-related cache entries
    pub fn invalidate_user(&self, user_id: i32) {
        self.remove(&keys::Users::by_id(user_id));
        self.remove(keys::USERS_ALL);
    }

    /// Invalidate all requirement-related cache entries
    pub fn invalidate_requirement(&self, req_id: i32) {
        self.remove(&keys::Requirements::by_id(req_id));
        self.remove(&keys::LinkedTests::for_requirement(req_id));
        self.remove(&keys::RequirementTitle::by_id(req_id));
        // Also invalidate global lists and project-level caches
        self.remove(keys::REQUIREMENTS_ALL);
        // Note: In a real implementation, you'd need to track which project the requirement belongs to
    }

    /// Invalidate all test-related cache entries
    pub fn invalidate_test(&self, test_id: i32) {
        self.remove(&keys::Tests::by_id(test_id));
        self.remove(&keys::LinkedRequirements::for_test(test_id));
        self.remove(&keys::TestStatus::by_id(test_id));
        // Also invalidate global lists and project-level caches
        self.remove(keys::TESTS_ALL);
        // Note: In a real implementation, you'd need to track which project the test belongs to
    }

    /// Invalidate all category-related cache entries
    pub fn invalidate_category(&self, cat_id: i32) {
        self.remove(&keys::Categories::by_id(cat_id));
        self.remove(keys::CATEGORIES_ALL);
    }

    /// Invalidate all status-related cache entries
    pub fn invalidate_status(&self, status_id: i32) {
        self.remove(&keys::Status::by_id(status_id));
        self.remove(keys::STATUS_ALL);
    }

    /// Invalidate all verification-related cache entries
    pub fn invalidate_verification(&self, verification_id: i32) {
        self.remove(&keys::Verification::by_id(verification_id));
        self.remove(keys::VERIFICATION_ALL);
    }

    /// Invalidate all applicability-related cache entries
    pub fn invalidate_applicability(&self, applicability_id: i32) {
        self.remove(&keys::Applicability::by_id(applicability_id));
        self.remove(keys::APPLICABILITY_ALL);
    }
}

/// Warm up the cache with frequently accessed data
pub fn warm_cache() {
    let cache = get_cache();

    let repo = DieselRepo::new();

    // Warm up projects cache
    if let Ok(projects) = repo.get_projects_all() {
        if let Ok(json_data) = serde_json::to_string(&projects) {
            cache.set_with_ttl(keys::PROJECTS_ALL, json_data, Duration::from_secs(600));
        }
    }

    // Warm up status cache
    if let Ok(statuses) = repo.get_status_all() {
        if let Ok(json_data) = serde_json::to_string(&statuses) {
            cache.set_with_ttl(keys::STATUS_ALL, json_data, Duration::from_secs(900));
        }
    }

    // Warm up categories cache
    if let Ok(categories) = repo.get_categories_all() {
        if let Ok(json_data) = serde_json::to_string(&categories) {
            cache.set_with_ttl(keys::CATEGORIES_ALL, json_data, Duration::from_secs(900));
        }
    }

    // Warm up users cache
    if let Ok(users) = repo.get_users_all() {
        if let Ok(json_data) = serde_json::to_string(&users) {
            cache.set_with_ttl(keys::USERS_ALL, json_data, Duration::from_secs(600));
        }
    }

    // Warm up projects navigation cache
    if let Ok(projects) = repo.get_projects_all() {
        if let Ok(json_data) = serde_json::to_string(&projects) {
            cache.set_with_ttl(keys::PROJECTS_NAV, json_data, Duration::from_secs(300));
        }
    }
}

/// Warm up cache for a specific project
/* TODO: never used ??
pub fn warm_project_cache(project_id: i32) {
    let cache = get_cache();

    let repo = DieselRepo::new();

    // Warm up project-specific requirements
    if let Ok(requirements) = repo.get_requirements_by_project(project_id) {
        if let Ok(json_data) = serde_json::to_string(&requirements) {
            cache.set_with_ttl(
                &keys::requirements_by_project(project_id),
                json_data,
                Duration::from_secs(300)
            );
        }
    }

    // Warm up project-specific tests
    if let Ok(tests) = repo.get_tests_by_project(project_id) {
        if let Ok(json_data) = serde_json::to_string(&tests) {
            cache.set_with_ttl(
                &keys::Tests::by_project(project_id),
                json_data,
                Duration::from_secs(300)
            );
        }
    }

    // Warm up project-specific matrix data
    if let Ok(matrix_data) = repo.get_matrix_by_project(project_id) {
        if let Ok(json_data) = serde_json::to_string(&matrix_data) {
            cache.set_with_ttl(
                &keys::Matrix::by_project(project_id),
                json_data,
                Duration::from_secs(180)
            );
        }
    }

    // Warm up project-specific categories
    if let Ok(categories) = repo.get_categories_by_project(project_id) {
        if let Ok(json_data) = serde_json::to_string(&categories) {
            cache.set_with_ttl(
                &keys::Categories::by_project(project_id),
                json_data,
                Duration::from_secs(600)
            );
        }
    }

    // Warm up project-specific verification types
    if let Ok(verifications) = repo.get_verification_by_project(project_id) {
        if let Ok(json_data) = serde_json::to_string(&verifications) {
            cache.set_with_ttl(
                &keys::Verification::by_project(project_id),
                json_data,
                Duration::from_secs(600)
            );
        }
    }
}
*/

/// Clear expired cache entries
pub fn cleanup_expired() {
    let cache = get_cache();
    cache.cleanup();
}

/// Start background cache maintenance tasks
pub fn start_cache_maintenance() {
    thread::spawn(|| {
        loop {
            // Sleep for 5 minutes
            thread::sleep(Duration::from_secs(300));

            // Clean up expired entries
            cleanup_expired();

            // Warm up frequently accessed data if cache is getting empty
            let cache = get_cache();
            let stats = cache.stats();
            if stats.active_entries < 10 {
                warm_cache();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::cache::{
        get_cache_health, get_cache_performance, get_cache_recommendations, get_cache_stats,
        get_memory_usage,
    };
    use std::sync::{atomic::Ordering, Mutex};
    use std::thread;
    use std::time::Duration;
    use std::time::Instant;

    static CACHE_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_cache_basic_operations() {
        let cache = Cache::new(300);

        // Test set and get
        cache.set("key1", "value1".to_string());
        assert_eq!(cache.get("key1"), Some("value1".to_string()));

        // Test TTL
        cache.set_with_ttl("key2", "value2".to_string(), Duration::from_millis(1));
        thread::sleep(Duration::from_millis(10));
        assert_eq!(cache.get("key2"), None);

        // Test remove
        cache.set("key3", "value3".to_string());
        cache.remove("key3");
        assert_eq!(cache.get("key3"), None);
    }

    #[test]
    fn test_cache_cleanup() {
        let cache = Cache::new(300);

        // Add some expired entries
        cache.set_with_ttl("expired1", "value1".to_string(), Duration::from_millis(1));
        cache.set_with_ttl("expired2", "value2".to_string(), Duration::from_millis(1));
        cache.set("valid", "value3".to_string());

        thread::sleep(Duration::from_millis(10));

        let cleaned = cache.cleanup();
        assert_eq!(cleaned, 2);
        assert_eq!(cache.get("expired1"), None);
        assert_eq!(cache.get("expired2"), None);
        assert_eq!(cache.get("valid"), Some("value3".to_string()));
    }

    #[test]
    fn test_concurrent_read_performance() {
        let cache = Arc::new(Cache::new(300));

        // Pre-populate cache with some data
        for i in 0..1000 {
            cache.set(&format!("key{}", i), format!("value{}", i));
        }

        let start = Instant::now();

        // Simulate concurrent reads from multiple threads
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let cache = Arc::clone(&cache);
                thread::spawn(move || {
                    for i in 0..1000 {
                        let _ = cache.get(&format!("key{}", i % 1000));
                    }
                })
            })
            .collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        let duration = start.elapsed();
        println!("Concurrent read performance test completed in {:?}", duration);

        // Verify cache integrity
        assert_eq!(cache.get("key0"), Some("value0".to_string()));
        assert_eq!(cache.get("key999"), Some("value999".to_string()));
    }

    #[test]
    fn test_stats_performance_improvement() {
        let cache = Arc::new(Cache::new(300));

        // Pre-populate cache with many entries
        for i in 0..10000 {
            cache.set(&format!("key{}", i), format!("value{}", i));
        }

        // Test stats performance with large cache
        let start = Instant::now();
        let stats = cache.stats();
        let duration = start.elapsed();

        println!("Stats calculation with 10,000 entries took: {:?}", duration);
        println!("Active entries: {}, Total entries: {}", stats.active_entries, stats.total_entries);

        // Verify the performance improvement - should be very fast now
        assert!(duration.as_micros() < 1000, "Stats should be calculated in under 1ms, took {:?}", duration);

        // Verify counters are working correctly
        assert_eq!(stats.total_entries, 10000);
        assert_eq!(stats.active_entries, 10000);
        assert_eq!(stats.expired_entries, 0);
    }

    #[test]
    fn test_counters_and_reset() {
        let cache = Cache::new(300);

        // Initial stats should be zeroed
        assert_eq!(cache.stats().total_entries, 0);

        // One hit and one miss
        cache.set("a", "1".to_string());
        assert_eq!(cache.get("a"), Some("1".to_string()));
        assert_eq!(cache.get("missing"), None);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.active_entries, 1);

        // Reset all counters
        cache.reset_counters();
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.active_entries, 0);
    }

    #[test]
    fn test_sync_counters_updates_expired() {
        let cache = Cache::new(300);

        cache.set_with_ttl("active", "v".to_string(), Duration::from_secs(1));
        cache.set_with_ttl("expired", "v".to_string(), Duration::from_millis(1));
        thread::sleep(Duration::from_millis(10));

        cache.sync_counters();
        let stats = cache.stats();
        assert_eq!(stats.active_entries, 1);
        assert_eq!(stats.expired_entries, 1);
    }

    #[test]
    fn test_global_cache_helpers() {
        let _guard = CACHE_LOCK.lock().unwrap();
        invalidate_all_cache();
        let cache = get_cache();
        cache.reset_counters();

        cache.set("keep", "v".to_string());
        cache.set_with_ttl("gone", "v".to_string(), Duration::from_millis(1));
        thread::sleep(Duration::from_millis(10));
        cache.sync_counters();

        let stats = get_cache_stats();
        assert_eq!(stats["total_entries"].as_u64(), Some(2));
        assert_eq!(stats["active_entries"].as_u64(), Some(1));
        assert_eq!(stats["expired_entries"].as_u64(), Some(1));
        assert_eq!(stats["cleanup_available"].as_bool(), Some(true));

        cleanup_expired();
        cache.sync_counters();
        let stats = get_cache_stats();
        assert_eq!(stats["total_entries"].as_u64(), Some(1));
        assert_eq!(stats["active_entries"].as_u64(), Some(1));
        assert_eq!(stats["expired_entries"].as_u64(), Some(0));
        assert_eq!(get_memory_usage(), 1);

        let health = get_cache_health();
        assert_eq!(health["status"].as_str(), Some("healthy"));
        assert_eq!(health["cleanup_needed"].as_bool(), Some(false));

        // Trigger performance recommendations
        invalidate_all_cache();
        cache.reset_counters();
        for i in 0..5 {
            let _ = cache.get(&format!("missing{}", i));
        }
        for i in 0..1001 {
            cache.set(&format!("key{}", i), "v".to_string());
        }

        let perf = get_cache_performance();
        assert_eq!(perf["total_requests"].as_u64(), Some(5));

        let recs = get_cache_recommendations();
        let arr = recs["recommendations"].as_array().unwrap();
        assert!(arr.iter().any(|r| r.as_str().unwrap().contains("hit rate")));
    }

    #[test]
    fn test_cache_stats_branches() {
        let _guard = CACHE_LOCK.lock().unwrap();
        invalidate_all_cache();
        let cache = get_cache();
        cache.reset_counters();

        // Empty cache stats and performance
        let stats_empty = cache.stats();
        assert_eq!(stats_empty.total_entries, 0);
        assert_eq!(stats_empty.cache_size_bytes, 0);
        let perf_empty = get_cache_performance();
        assert_eq!(perf_empty["total_requests"].as_u64(), Some(0));
        assert_eq!(perf_empty["cache_efficiency"].as_f64(), Some(0.0));
        assert_eq!(perf_empty["expired_entries_percentage"].as_f64(), Some(0.0));
        invalidate_all_cache();

        // Configure counters to trigger all recommendation branches
        let big_value = String::from_utf8(vec![b'x'; 105 * 1024 * 1024]).unwrap();
        cache.set("huge", big_value);
        cache.hits.store(1, Ordering::Relaxed);
        cache.misses.store(2, Ordering::Relaxed);
        cache.total_access_time.store(6_000_000, Ordering::Relaxed);
        cache.active_entries.store(10, Ordering::Relaxed);
        cache.expired_entries.store(11, Ordering::Relaxed);

        let health_degraded = get_cache_health();
        assert_eq!(health_degraded["status"].as_str(), Some("degraded"));
        assert!(health_degraded["cleanup_needed"].as_bool().unwrap());

        cache.expired_entries.store(25, Ordering::Relaxed);
        let health_warning = get_cache_health();
        assert_eq!(health_warning["status"].as_str(), Some("warning"));

        let recs_bad = get_cache_recommendations();
        let recs_bad_arr = recs_bad["recommendations"].as_array().unwrap();
        assert!(recs_bad_arr
            .iter()
            .any(|r| r.as_str().unwrap().contains("increasing cache TTL")));
        assert!(recs_bad_arr
            .iter()
            .any(|r| r.as_str().unwrap().contains("hit rate is low")));
        assert!(recs_bad_arr
            .iter()
            .any(|r| r.as_str().unwrap().contains("expired entries")));
        assert!(recs_bad_arr
            .iter()
            .any(|r| r.as_str().unwrap().contains("memory usage is high")));
        assert!(recs_bad_arr
            .iter()
            .any(|r| r.as_str().unwrap().contains("access time is slow")));
        assert_eq!(recs_bad["priority"].as_str(), Some("high"));

        // Reset to good state to trigger positive recommendation
        invalidate_all_cache();
        cache.reset_counters();
        cache.hits.store(10, Ordering::Relaxed);
        cache.total_access_time.store(10, Ordering::Relaxed);
        cache.active_entries.store(1, Ordering::Relaxed);
        cache.expired_entries.store(0, Ordering::Relaxed);

        let recs_good = get_cache_recommendations();
        let recs_good_arr = recs_good["recommendations"].as_array().unwrap();
        assert_eq!(recs_good_arr.len(), 1);
        assert!(recs_good_arr[0]
            .as_str()
            .unwrap()
            .contains("performing well"));
        assert_eq!(recs_good["priority"].as_str(), Some("low"));

        let health_healthy = get_cache_health();
        assert_eq!(health_healthy["status"].as_str(), Some("healthy"));

        invalidate_all_cache();
        cache.reset_counters();
    }
}

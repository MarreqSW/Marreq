use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::thread;
use chrono;
use crate::repository::*;
use super::keys;
use crate::cache::keys::Keyspace;

/// Cache entry with TTL (Time To Live)
#[derive(Clone)]
struct CacheEntry<T> {
    data: T,
    expires_at: Instant,
}

/// In-memory cache with TTL support
pub struct Cache {
    data: Arc<RwLock<HashMap<String, CacheEntry<String>>>>,
    default_ttl: Duration,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    total_access_time: Arc<AtomicU64>,
    active_entries: Arc<AtomicU64>,
    expired_entries: Arc<AtomicU64>,
    last_cleanup: Arc<Mutex<chrono::DateTime<chrono::Utc>>>,
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
            Some(cache) if cache.expires_at > Instant::now() => { Status::Hit(cache.data.clone()) }
            Some(_) => { Status::Expired }
            None => { Status::Miss }
        }
    }

    fn remove_(&self, key: &str) -> Option<CacheEntry<String>> {
        let mut data = self.data.write().unwrap();
        data.remove(key)
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
                self.expired_entries.fetch_add(1, Ordering::Relaxed);
                self.misses.fetch_add(1, Ordering::Relaxed);
                self.remove_(key);
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
            // Update counters based on whether the entry was expired
            if entry.expires_at <= Instant::now() {
                self.expired_entries.fetch_sub(1, Ordering::Relaxed);
            } else {
                self.active_entries.fetch_sub(1, Ordering::Relaxed);
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
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        // Use atomic counters for O(1) performance instead of O(n) calculations
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total_requests = hits + misses;

        let total_access_time = self.total_access_time.load(Ordering::Relaxed);
        let (hit_rate, miss_rate, average_access_time_ns) = if total_requests == 0 {
            (0.0, 0.0, 0)
        } else {
            let tr_f = total_requests as f64;
            (
                hits as f64 / tr_f,
                misses as f64 / tr_f,
                total_access_time / total_requests,
            )
        };

        let active_entries = self.active_entries.load(Ordering::Relaxed) as usize;
        let expired_entries = self.expired_entries.load(Ordering::Relaxed) as usize;
        let total_entries = active_entries + expired_entries;
        
        let last_cleanup = *self.last_cleanup.lock().unwrap();
        
        // Only calculate cache size when needed (this is still O(n) but less frequent)
        let cache_size_bytes = if total_entries > 0 {
            let data = self.data.read().unwrap();
            data.iter()
                .map(|(k, e)| k.len() + e.data.len())
                .sum()
        } else {
            0
        };

        CacheStats {
            total_entries,
            active_entries,
            expired_entries,
            hits,
            misses,
            total_requests,
            hit_rate,
            miss_rate,
            average_access_time_ns,
            total_access_time_ns: total_access_time,
            last_cleanup,
            cache_size_bytes,
        }
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

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub active_entries: usize,
    pub expired_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub total_requests: u64,
    pub hit_rate: f64,
    pub miss_rate: f64,
    pub average_access_time_ns: u64,
    pub total_access_time_ns: u64,
    pub last_cleanup: chrono::DateTime<chrono::Utc>,
    pub cache_size_bytes: usize,
}

// Global cache instance
lazy_static::lazy_static! {
    static ref CACHE: Cache = Cache::new(30); // 5 minutes default TTL
}

/// Cache utility functions
pub fn get_cache() -> &'static Cache {
    &CACHE
}

/// Invalidate all project-related cache entries
pub fn invalidate_project_cache(project_id: i32) {
    let cache = get_cache();
    cache.remove(&keys::Requirements::by_project(project_id));
    cache.remove(&keys::Tests::by_project(project_id));
    cache.remove(&keys::Matrix::by_project(project_id));
    cache.remove(&keys::Verification::by_project(project_id));
    cache.remove(&keys::Categories::by_project(project_id));
    cache.remove(&keys::Applicability::by_project(project_id));
    cache.remove(&keys::Projects::by_id(project_id));
}

/// Invalidate all user-related cache entries
pub fn invalidate_user_cache(user_id: i32) {
    let cache = get_cache();
    cache.remove(&keys::Users::by_id(user_id));
    cache.remove(keys::USERS_ALL);
}

/// Invalidate all requirement-related cache entries
pub fn invalidate_requirement_cache(req_id: i32) {
    let cache = get_cache();
    cache.remove(&keys::Requirements::by_id(req_id));
    cache.remove(&keys::LinkedTests::for_requirement(req_id));
    cache.remove(&keys::RequirementTitle::by_id(req_id));
    // Also invalidate global lists and project-level caches
    cache.remove(keys::REQUIREMENTS_ALL);
    // Note: In a real implementation, you'd need to track which project the requirement belongs to
}

/// Invalidate all test-related cache entries
pub fn invalidate_test_cache(test_id: i32) {
    let cache = get_cache();
    cache.remove(&keys::Tests::by_id(test_id));
    cache.remove(&keys::LinkedRequirements::for_test(test_id));
    cache.remove(&keys::TestStatus::by_id(test_id));
    // Also invalidate global lists and project-level caches
    cache.remove(keys::TESTS_ALL);
    // Note: In a real implementation, you'd need to track which project the test belongs to
}

/// Invalidate all category-related cache entries
pub fn invalidate_category_cache(cat_id: i32) {
    let cache = get_cache();
    cache.remove(&keys::Categories::by_id(cat_id));
    cache.remove(keys::CATEGORIES_ALL);
}

/// Invalidate all status-related cache entries
pub fn invalidate_status_cache(status_id: i32) {
    let cache = get_cache();
    cache.remove(&keys::Status::by_id(status_id));
    cache.remove(keys::STATUS_ALL);
}

/// Invalidate all verification-related cache entries
pub fn invalidate_verification_cache(verification_id: i32) {
    let cache = get_cache();
    cache.remove(&keys::Verification::by_id(verification_id));
    cache.remove(keys::VERIFICATION_ALL);
}

/// Invalidate all applicability-related cache entries
pub fn invalidate_applicability_cache(applicability_id: i32) {
    let cache = get_cache();
    cache.remove(&keys::Applicability::by_id(applicability_id));
    cache.remove(keys::APPLICABILITY_ALL);
}

/// Invalidate all cache entries (use with caution)
pub fn invalidate_all_cache() {
    let cache = get_cache();
    cache.clear();
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

/// Get cache statistics
pub fn get_cache_stats() -> serde_json::Value {
    let cache = get_cache();
    let stats = cache.stats();
    
    json!({
        "total_entries": stats.total_entries,
        "active_entries": stats.active_entries,
        "expired_entries": stats.expired_entries,
        "memory_usage": get_memory_usage(),
        "cleanup_available": stats.expired_entries > 0
    })
}

/// Clear expired cache entries
pub fn cleanup_expired() {
    let cache = get_cache();
    cache.cleanup();
}

/// Get cache memory usage
pub fn get_memory_usage() -> usize {
    let cache = get_cache();
    let stats = cache.stats();
    stats.total_entries
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

/// Get cache health status
pub fn get_cache_health() -> serde_json::Value {
    let cache = get_cache();
    let stats = cache.stats();
    let memory_usage = get_memory_usage();
    
    let health_status = if stats.expired_entries > stats.active_entries * 2 {
        "warning"
    } else if stats.expired_entries > stats.active_entries {
        "degraded"
    } else {
        "healthy"
    };
    
    json!({
        "status": health_status,
        "total_entries": stats.total_entries,
        "active_entries": stats.active_entries,
        "expired_entries": stats.expired_entries,
        "memory_usage": memory_usage,
        "cleanup_needed": stats.expired_entries > 0,
        "last_cleanup": chrono::Utc::now().to_rfc3339()
    })
}

/// Get detailed cache performance metrics
pub fn get_cache_performance() -> serde_json::Value {
    let cache = get_cache();
    let stats = cache.stats();
    
    let performance_metrics = json!({
        "hit_rate_percentage": (stats.hit_rate * 100.0).round() / 100.0,
        "miss_rate_percentage": (stats.miss_rate * 100.0).round() / 100.0,
        "total_requests": stats.total_requests,
        "hits": stats.hits,
        "misses": stats.misses,
        "average_access_time_ms": (stats.average_access_time_ns as f64 / 1_000_000.0).round() / 1000.0,
        "total_access_time_ms": (stats.total_access_time_ns as f64 / 1_000_000.0).round() / 1000.0,
        "cache_efficiency": if stats.total_entries > 0 {
            (stats.active_entries as f64 / stats.total_entries as f64 * 100.0).round() / 100.0
        } else {
            0.0
        },
        "memory_usage_mb": (stats.cache_size_bytes as f64 / 1_048_576.0).round() / 1000.0,
        "last_cleanup": stats.last_cleanup.to_rfc3339(),
        "cleanup_needed": stats.expired_entries > 0,
        "expired_entries_percentage": if stats.total_entries > 0 {
            (stats.expired_entries as f64 / stats.total_entries as f64 * 100.0).round() / 100.0
        } else {
            0.0
        }
    });
    
    // Cache the performance metrics
    cache.set_with_ttl(keys::CACHE_PERFORMANCE, performance_metrics.to_string(), Duration::from_secs(60));
    
    performance_metrics
}

/// Get cache optimization recommendations
pub fn get_cache_recommendations() -> serde_json::Value {
    let cache = get_cache();
    let stats = cache.stats();
    let mut recommendations = Vec::new();
    
    // Analyze hit rate
    if stats.hit_rate < 0.7 {
        recommendations.push("Consider increasing cache TTL for frequently accessed data");
    }
    if stats.hit_rate < 0.5 {
        recommendations.push("Cache hit rate is low - review cache invalidation strategy");
    }
    
    // Analyze expired entries
    if stats.expired_entries > stats.active_entries {
        recommendations.push("High number of expired entries - consider adjusting TTL values");
    }
    
    // Analyze memory usage
    if stats.cache_size_bytes > 100 * 1024 * 1024 { // 100MB
        recommendations.push("Cache memory usage is high - consider implementing compression");
    }
    
    // Analyze access patterns
    if stats.average_access_time_ns > 1_000_000 { // 1ms
        recommendations.push("Cache access time is slow - consider optimizing data structures");
    }
    
    // If no issues, provide positive feedback
    if recommendations.is_empty() {
        recommendations.push("Cache is performing well - no immediate optimizations needed");
    }
    
    json!({
        "recommendations": recommendations,
        "priority": if recommendations.len() > 2 { "high" } else if recommendations.len() > 1 { "medium" } else { "low" },
        "analysis_timestamp": chrono::Utc::now().to_rfc3339()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    use std::time::Instant;

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
        let handles: Vec<_> = (0..10).map(|_| {
            let cache = Arc::clone(&cache);
            thread::spawn(move || {
                for i in 0..1000 {
                    let _ = cache.get(&format!("key{}", i % 1000));
                }
            })
        }).collect();
        
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
}

use super::keys::{self, Keyspace};
use chrono;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
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

    /// Start background cache maintenance tasks
    pub fn start_cache_maintenance(self: &Arc<Self>) {
        let weak: Weak<Self> = Arc::downgrade(self);
        thread::spawn(move || {
            loop {
                // stop if `Cache` is dropped everywhere
                match weak.upgrade() {
                    Some(this) => {
                        this.cleanup();
                        thread::sleep(this.default_ttl);
                    }
                    None => break, // Cache gone; exit thread
                }
            }
        });
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
        let expired_count = data
            .values()
            .filter(|entry| entry.expires_at <= now)
            .count();

        data.retain(|_, entry| entry.expires_at > now);

        let cleaned_count = initial_count - data.len();

        // Update counters for cleaned entries
        if expired_count > 0 {
            self.expired_entries
                .fetch_sub(expired_count as u64, Ordering::Relaxed);
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
        let actual_expired = data
            .values()
            .filter(|entry| entry.expires_at <= now)
            .count() as u64;

        self.active_entries.store(actual_active, Ordering::Relaxed);
        self.expired_entries
            .store(actual_expired, Ordering::Relaxed);
    }

    /// Invalidate all project-related cache entries
    pub fn invalidate_project(&self, project_id: i32) {
        self.remove(&keys::Requirements::by_project(project_id));
        self.remove(&keys::Tests::by_project(project_id));
        self.remove(&keys::Matrix::by_project(project_id));
        self.remove(&keys::VerificationMethod::by_project(project_id));
        self.remove(&keys::Categories::by_project(project_id));
        self.remove(&keys::Applicability::by_project(project_id));
        self.remove(&keys::Projects::by_id(project_id));
        self.remove(&keys::ProjectMembers::by_project(project_id));
        self.remove(keys::PROJECTS_ALL);
        self.remove(keys::PROJECTS_NAV);
    }

    /// Invalidate all user-related cache entries
    pub fn invalidate_user(&self, user_id: i32) {
        self.remove(&keys::Users::by_id(user_id));
        self.remove(keys::USERS_ALL);
    }

    /// Invalidate cache entries related to a user/project membership tuple
    pub fn invalidate_project_membership(&self, project_id: i32, user_id: i32) {
        self.remove(&keys::ProjectMembers::by_project(project_id));
        self.remove(&keys::ProjectMembers::for_user(user_id));
    }

    /// Invalidate all requirement-related cache entries
    pub fn invalidate_requirement(&self, id: i32) {
        self.remove(&keys::Requirements::by_id(id));
        self.remove(&keys::LinkedTests::for_requirement(id));
        self.remove(&keys::RequirementTitle::by_id(id));
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
        self.remove(&keys::VerificationMethod::by_id(verification_id));
        self.remove(keys::VERIFICATION_ALL);
    }

    /// Invalidate all applicability-related cache entries
    pub fn invalidate_applicability(&self, applicability_id: i32) {
        self.remove(&keys::Applicability::by_id(applicability_id));
        self.remove(keys::APPLICABILITY_ALL);
    }
}

#[cfg(test)]
mod tests {
    use super::super::keys;
    use super::*;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

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
        println!(
            "Concurrent read performance test completed in {:?}",
            duration
        );

        // Verify cache integrity
        assert_eq!(cache.get("key0"), Some("value0".to_string()));
        assert_eq!(cache.get("key999"), Some("value999".to_string()));
    }

    // tests moved to stats.rs

    #[test]
    fn test_invalidate_project_removes_related_keys() {
        let cache = Cache::new(300);
        let pid = 1;
        cache.set(&keys::Requirements::by_project(pid), "r".to_string());
        cache.set(&keys::Tests::by_project(pid), "t".to_string());
        cache.set(&keys::Matrix::by_project(pid), "m".to_string());
        cache.set(&keys::VerificationMethod::by_project(pid), "v".to_string());
        cache.set(&keys::Categories::by_project(pid), "c".to_string());
        cache.set(&keys::Applicability::by_project(pid), "a".to_string());
        cache.set(&keys::Projects::by_id(pid), "p".to_string());
        cache.set(&keys::ProjectMembers::by_project(pid), "pm".to_string());
        cache.set(keys::PROJECTS_ALL, "pa".to_string());
        cache.set(keys::PROJECTS_NAV, "pn".to_string());
        cache.invalidate_project(pid);
        assert!(cache.get(&keys::Requirements::by_project(pid)).is_none());
        assert!(cache.get(&keys::Tests::by_project(pid)).is_none());
        assert!(cache.get(&keys::Matrix::by_project(pid)).is_none());
        assert!(cache.get(&keys::VerificationMethod::by_project(pid)).is_none());
        assert!(cache.get(&keys::Categories::by_project(pid)).is_none());
        assert!(cache.get(&keys::Applicability::by_project(pid)).is_none());
        assert!(cache.get(&keys::Projects::by_id(pid)).is_none());
        assert!(cache.get(&keys::ProjectMembers::by_project(pid)).is_none());
        assert!(cache.get(keys::PROJECTS_ALL).is_none());
        assert!(cache.get(keys::PROJECTS_NAV).is_none());
    }

    #[test]
    fn test_invalidate_user_removes_related_keys() {
        let cache = Cache::new(300);
        let uid = 7;
        cache.set(&keys::Users::by_id(uid), "u".to_string());
        cache.set(keys::USERS_ALL, "ua".to_string());
        cache.invalidate_user(uid);
        assert!(cache.get(&keys::Users::by_id(uid)).is_none());
        assert!(cache.get(keys::USERS_ALL).is_none());
        assert!(cache.get(&keys::ProjectMembers::for_user(uid)).is_none());
    }

    #[test]
    fn test_invalidate_project_membership_removes_related_keys() {
        let cache = Cache::new(300);
        let uid = 11;
        let pid = 22;
        cache.set(&keys::ProjectMembers::by_project(pid), "pm".to_string());
        cache.set(&keys::ProjectMembers::for_user(uid), "pmu".to_string());
        cache.invalidate_project_membership(pid, uid);
        assert!(cache.get(&keys::ProjectMembers::by_project(pid)).is_none());
        assert!(cache.get(&keys::ProjectMembers::for_user(uid)).is_none());
    }

    #[test]
    fn test_invalidate_requirement_removes_related_keys() {
        let cache = Cache::new(300);
        let rid = 42;
        cache.set(&keys::Requirements::by_id(rid), "r".to_string());
        cache.set(&keys::LinkedTests::for_requirement(rid), "lt".to_string());
        cache.set(&keys::RequirementTitle::by_id(rid), "rt".to_string());
        cache.set(keys::REQUIREMENTS_ALL, "ra".to_string());
        cache.invalidate_requirement(rid);
        assert!(cache.get(&keys::Requirements::by_id(rid)).is_none());
        assert!(cache
            .get(&keys::LinkedTests::for_requirement(rid))
            .is_none());
        assert!(cache.get(&keys::RequirementTitle::by_id(rid)).is_none());
        assert!(cache.get(keys::REQUIREMENTS_ALL).is_none());
    }

    #[test]
    fn test_invalidate_test_removes_related_keys() {
        let cache = Cache::new(300);
        let tid = 5;
        cache.set(&keys::Tests::by_id(tid), "t".to_string());
        cache.set(&keys::LinkedRequirements::for_test(tid), "lr".to_string());
        cache.set(&keys::TestStatus::by_id(tid), "ts".to_string());
        cache.set(keys::TESTS_ALL, "ta".to_string());
        cache.invalidate_test(tid);
        assert!(cache.get(&keys::Tests::by_id(tid)).is_none());
        assert!(cache
            .get(&keys::LinkedRequirements::for_test(tid))
            .is_none());
        assert!(cache.get(&keys::TestStatus::by_id(tid)).is_none());
        assert!(cache.get(keys::TESTS_ALL).is_none());
    }

    #[test]
    fn test_invalidate_category_removes_related_keys() {
        let cache = Cache::new(300);
        let cid = 3;
        cache.set(&keys::Categories::by_id(cid), "c".to_string());
        cache.set(keys::CATEGORIES_ALL, "ca".to_string());
        cache.invalidate_category(cid);
        assert!(cache.get(&keys::Categories::by_id(cid)).is_none());
        assert!(cache.get(keys::CATEGORIES_ALL).is_none());
    }

    #[test]
    fn test_invalidate_status_removes_related_keys() {
        let cache = Cache::new(300);
        let sid = 9;
        cache.set(&keys::Status::by_id(sid), "s".to_string());
        cache.set(keys::STATUS_ALL, "sa".to_string());
        cache.invalidate_status(sid);
        assert!(cache.get(&keys::Status::by_id(sid)).is_none());
        assert!(cache.get(keys::STATUS_ALL).is_none());
    }

    #[test]
    fn test_invalidate_verification_removes_related_keys() {
        let cache = Cache::new(300);
        let vid = 4;
        cache.set(&keys::VerificationMethod::by_id(vid), "v".to_string());
        cache.set(keys::VERIFICATION_ALL, "va".to_string());
        cache.invalidate_verification(vid);
        assert!(cache.get(&keys::VerificationMethod::by_id(vid)).is_none());
        assert!(cache.get(keys::VERIFICATION_ALL).is_none());
    }

    #[test]
    fn test_invalidate_applicability_removes_related_keys() {
        let cache = Cache::new(300);
        let aid = 6;
        cache.set(&keys::Applicability::by_id(aid), "a".to_string());
        cache.set(keys::APPLICABILITY_ALL, "aa".to_string());
        cache.invalidate_applicability(aid);
        assert!(cache.get(&keys::Applicability::by_id(aid)).is_none());
        assert!(cache.get(keys::APPLICABILITY_ALL).is_none());
    }

    #[test]
    fn test_start_cache_maintenance_cleans_expired_entries() {
        let cache = Arc::new(Cache::new(60));

        // Insert enough active entries so warm_cache is not triggered
        for i in 0..10 {
            cache.set(&format!("key{}", i), "v".to_string());
        }

        // Insert an entry that will expire quickly
        cache.set_with_ttl("expired", "x".to_string(), Duration::from_millis(1));
        thread::sleep(Duration::from_millis(10));

        // Ensure the expired entry is still present before maintenance starts
        assert!(cache.data.read().unwrap().contains_key("expired"));

        cache.start_cache_maintenance();

        // Give maintenance thread time to run cleanup once
        thread::sleep(Duration::from_millis(50));

        let data = cache.data.read().unwrap();
        assert!(!data.contains_key("expired"));
        assert_eq!(data.len(), 10);
    }
}

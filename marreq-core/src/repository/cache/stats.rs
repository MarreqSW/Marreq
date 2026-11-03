// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::keys;
use super::repository::Cache;
use chrono;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{Duration, Instant};

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

impl Cache {
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        // Hit/miss/access-time use atomic counters, so reading them is O(1).
        let hits = self.hits.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.misses.load(std::sync::atomic::Ordering::Relaxed);
        let total_requests = hits + misses;

        let total_access_time = self
            .total_access_time
            .load(std::sync::atomic::Ordering::Relaxed);
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

        // Count the entries from the map itself. Entries expire by the clock
        // without anything touching them, so incrementally maintained counters
        // drift (and used to underflow); one pass under the read lock cannot.
        let (active_entries, expired_entries, cache_size_bytes) = {
            let data = self.data.read().unwrap();
            let now = Instant::now();
            let mut active = 0usize;
            let mut expired = 0usize;
            let mut size = 0usize;
            for (key, entry) in data.iter() {
                if entry.expires_at > now {
                    active += 1;
                } else {
                    expired += 1;
                }
                size += key.len() + entry.data.len();
            }
            (active, expired, size)
        };
        let total_entries = active_entries + expired_entries;

        let last_cleanup = *self.last_cleanup.lock().unwrap();

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

    /// Get cache statistics summary
    pub fn get_stats(&self) -> serde_json::Value {
        let stats = self.stats();

        json!({
            "total_entries": stats.total_entries,
            "active_entries": stats.active_entries,
            "expired_entries": stats.expired_entries,
            "memory_usage": self.get_memory_usage(),
            "cleanup_available": stats.expired_entries > 0
        })
    }

    /// Get cache memory usage
    pub fn get_memory_usage(&self) -> usize {
        let stats = self.stats();
        stats.total_entries
    }

    /// Get cache health status
    pub fn get_health(&self) -> serde_json::Value {
        let stats = self.stats();
        let memory_usage = self.get_memory_usage();

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
    pub fn get_performance(&self) -> serde_json::Value {
        let stats = self.stats();

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
        self.set_with_ttl(
            keys::CACHE_PERFORMANCE,
            performance_metrics.to_string(),
            Duration::from_secs(60),
        );

        performance_metrics
    }

    /// Get cache optimization recommendations
    pub fn get_recommendations(&self) -> serde_json::Value {
        let stats = self.stats();
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
        if stats.cache_size_bytes > 100 * 1024 * 1024 {
            // 100MB
            recommendations.push("Cache memory usage is high - consider implementing compression");
        }

        // Analyze access patterns
        if stats.average_access_time_ns > 1_000_000 {
            // 1ms
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::Ordering};
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn test_stats_performance_improvement() {
        let cache = Arc::new(Cache::new(300));
        for i in 0..10000 {
            cache.set(&format!("key{}", i), format!("value{}", i));
        }
        let start = Instant::now();
        let stats = cache.stats();
        let duration = start.elapsed();
        assert!(
            duration.as_micros() < 5000,
            "Stats should be calculated in under 5ms, took {:?}",
            duration
        );
        assert_eq!(stats.total_entries, 10000);
        assert_eq!(stats.active_entries, 10000);
        assert_eq!(stats.expired_entries, 0);
    }

    #[test]
    fn test_counters_and_reset() {
        let cache = Cache::new(300);
        assert_eq!(cache.stats().total_entries, 0);
        cache.set("a", "1".to_string());
        assert_eq!(cache.get("a"), Some("1".to_string()));
        assert_eq!(cache.get("missing"), None);
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.active_entries, 1);
        cache.reset_counters();
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.active_entries, 1);
    }

    /// Entries that expire by the clock are never touched by `get`, so the
    /// entry counts have to reflect what is really in the cache.
    #[test]
    fn test_expired_entries_are_counted_before_cleanup() {
        let cache = Cache::new(300);
        cache.set_with_ttl("old1", "v".to_string(), Duration::from_millis(1));
        cache.set_with_ttl("old2", "v".to_string(), Duration::from_millis(1));
        cache.set("keep", "v".to_string());
        thread::sleep(Duration::from_millis(10));
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.active_entries, 1);
        assert_eq!(stats.expired_entries, 2);
    }

    /// Regression: `cleanup` used to subtract from an unsigned counter that
    /// had never been incremented, so `expired_entries` wrapped to u64::MAX.
    #[test]
    fn test_cleanup_does_not_underflow_the_entry_counts() {
        let cache = Cache::new(300);
        cache.set_with_ttl("old1", "v".to_string(), Duration::from_millis(1));
        cache.set_with_ttl("old2", "v".to_string(), Duration::from_millis(1));
        cache.set("keep", "v".to_string());
        thread::sleep(Duration::from_millis(10));
        assert_eq!(cache.cleanup(), 2);
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.active_entries, 1);
        assert_eq!(stats.expired_entries, 0);
    }

    /// Regression: reading an expired entry after `reset_counters` used to
    /// subtract from a zeroed counter and wrap `active_entries`.
    #[test]
    fn test_reading_an_expired_entry_does_not_underflow_the_entry_counts() {
        let cache = Cache::new(300);
        cache.set_with_ttl("old", "v".to_string(), Duration::from_millis(1));
        cache.reset_counters();
        thread::sleep(Duration::from_millis(10));
        assert_eq!(cache.get("old"), None);
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.active_entries, 0);
        assert_eq!(stats.expired_entries, 0);
        assert_eq!(stats.misses, 1);
    }

    /// `reset_counters` only zeroes the performance counters; the cached
    /// entries (and therefore the entry counts) stay.
    #[test]
    fn test_reset_counters_keeps_the_entries() {
        let cache = Cache::new(300);
        cache.set("a", "1".to_string());
        assert_eq!(cache.get("a"), Some("1".to_string()));
        cache.reset_counters();
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.active_entries, 1);
        assert_eq!(stats.total_entries, 1);
        assert_eq!(cache.get("a"), Some("1".to_string()));
    }

    #[test]
    fn test_stats_helpers() {
        let cache = Cache::new(300);
        cache.reset_counters();
        cache.set("keep", "v".to_string());
        cache.set_with_ttl("gone", "v".to_string(), Duration::from_millis(1));
        thread::sleep(Duration::from_millis(10));
        let stats = cache.get_stats();
        assert_eq!(stats["total_entries"].as_u64(), Some(2));
        assert_eq!(stats["active_entries"].as_u64(), Some(1));
        assert_eq!(stats["expired_entries"].as_u64(), Some(1));
        assert_eq!(stats["cleanup_available"].as_bool(), Some(true));
        cache.cleanup();
        let stats = cache.get_stats();
        assert_eq!(stats["total_entries"].as_u64(), Some(1));
        assert_eq!(stats["active_entries"].as_u64(), Some(1));
        assert_eq!(stats["expired_entries"].as_u64(), Some(0));
        assert_eq!(cache.get_memory_usage(), 1);
        let health = cache.get_health();
        assert_eq!(health["status"].as_str(), Some("healthy"));
        assert_eq!(health["cleanup_needed"].as_bool(), Some(false));
        cache.clear();
        cache.reset_counters();
        for i in 0..5 {
            let _ = cache.get(&format!("missing{}", i));
        }
        for i in 0..1001 {
            cache.set(&format!("key{}", i), "v".to_string());
        }
        let perf = cache.get_performance();
        assert_eq!(perf["total_requests"].as_u64(), Some(5));
        let recs = cache.get_recommendations();
        let arr = recs["recommendations"].as_array().unwrap();
        assert!(arr.iter().any(|r| r.as_str().unwrap().contains("hit rate")));
    }

    #[test]
    fn test_cache_stats_branches() {
        let cache = Cache::new(300);
        cache.reset_counters();
        let stats_empty = cache.stats();
        assert_eq!(stats_empty.total_entries, 0);
        assert_eq!(stats_empty.cache_size_bytes, 0);
        let perf_empty = cache.get_performance();
        assert_eq!(perf_empty["total_requests"].as_u64(), Some(0));
        assert_eq!(perf_empty["cache_efficiency"].as_f64(), Some(0.0));
        assert_eq!(perf_empty["expired_entries_percentage"].as_f64(), Some(0.0));
        let big_value = String::from_utf8(vec![b'x'; 105 * 1024 * 1024]).unwrap();
        cache.set("huge", big_value);
        cache.hits.store(1, Ordering::Relaxed);
        cache.misses.store(2, Ordering::Relaxed);
        cache.total_access_time.store(6_000_000, Ordering::Relaxed);
        // `get_performance` caches its own snapshot, so count the active entries
        // instead of assuming how many there are.
        let active = cache.stats().active_entries;
        let add_expired = |from: usize, to: usize| {
            for i in from..to {
                cache.set_with_ttl(
                    &format!("old{i}"),
                    "v".to_string(),
                    Duration::from_millis(1),
                );
            }
            thread::sleep(Duration::from_millis(10));
        };
        // expired > active -> degraded
        add_expired(0, active + 1);
        let health_degraded = cache.get_health();
        assert_eq!(health_degraded["status"].as_str(), Some("degraded"));
        assert!(health_degraded["cleanup_needed"].as_bool().unwrap());
        // expired > 2 * active -> warning
        add_expired(active + 1, active * 2 + 1);
        let health_warning = cache.get_health();
        assert_eq!(health_warning["status"].as_str(), Some("warning"));
        let recs_bad = cache.get_recommendations();
        let recs_bad_arr = recs_bad["recommendations"].as_array().unwrap();
        assert!(
            recs_bad_arr
                .iter()
                .any(|r| r.as_str().unwrap().contains("increasing cache TTL"))
        );
        assert!(
            recs_bad_arr
                .iter()
                .any(|r| r.as_str().unwrap().contains("hit rate is low"))
        );
        assert!(
            recs_bad_arr
                .iter()
                .any(|r| r.as_str().unwrap().contains("expired entries"))
        );
        assert!(
            recs_bad_arr
                .iter()
                .any(|r| r.as_str().unwrap().contains("memory usage is high"))
        );
        assert!(
            recs_bad_arr
                .iter()
                .any(|r| r.as_str().unwrap().contains("access time is slow"))
        );
        assert_eq!(recs_bad["priority"].as_str(), Some("high"));
        cache.clear();
        cache.reset_counters();
        cache.hits.store(10, Ordering::Relaxed);
        cache.total_access_time.store(10, Ordering::Relaxed);
        let recs_good = cache.get_recommendations();
        let recs_good_arr = recs_good["recommendations"].as_array().unwrap();
        assert_eq!(recs_good_arr.len(), 1);
        assert!(
            recs_good_arr[0]
                .as_str()
                .unwrap()
                .contains("performing well")
        );
        assert_eq!(recs_good["priority"].as_str(), Some("low"));
        let health_healthy = cache.get_health();
        assert_eq!(health_healthy["status"].as_str(), Some("healthy"));
        cache.clear();
        cache.reset_counters();
    }
}

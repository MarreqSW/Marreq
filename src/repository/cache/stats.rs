use super::cache::{get_cache, Cache};
use super::keys;
use chrono;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

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
        // Use atomic counters for O(1) performance instead of O(n) calculations
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

        let active_entries = self
            .active_entries
            .load(std::sync::atomic::Ordering::Relaxed) as usize;
        let expired_entries = self
            .expired_entries
            .load(std::sync::atomic::Ordering::Relaxed) as usize;
        let total_entries = active_entries + expired_entries;

        let last_cleanup = *self.last_cleanup.lock().unwrap();

        // Only calculate cache size when needed (this is still O(n) but less frequent)
        let cache_size_bytes = if total_entries > 0 {
            let data = self.data.read().unwrap();
            data.iter().map(|(k, e)| k.len() + e.data.len()).sum()
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
}

/// Get cache statistics summary
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

/// Get cache memory usage
pub fn get_memory_usage() -> usize {
    let cache = get_cache();
    let stats = cache.stats();
    stats.total_entries
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
    cache.set_with_ttl(
        keys::CACHE_PERFORMANCE,
        performance_metrics.to_string(),
        Duration::from_secs(60),
    );

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

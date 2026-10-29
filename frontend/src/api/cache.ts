import { fetchJson } from './transport';

/** `GET /api/cache/stats` (site administrators only). Rates are fractions in 0..1. */
export type CacheStats = {
  total_entries: number;
  active_entries: number;
  expired_entries: number;
  hits: number;
  misses: number;
  total_requests: number;
  hit_rate: number;
  miss_rate: number;
  average_access_time_ns: number;
  total_access_time_ns: number;
  /** RFC 3339 timestamp. */
  last_cleanup: string;
  cache_size_bytes: number;
};

export type CacheRecommendationPriority = 'high' | 'medium' | 'low';

export type CacheRecommendations = {
  recommendations: string[];
  priority: CacheRecommendationPriority;
  /** RFC 3339 timestamp. */
  analysis_timestamp: string;
};

/** Answer of the write endpoints; `cleaned_entries` only comes with a cleanup. */
export type CacheActionResult = {
  message: string;
  timestamp: string;
  cleaned_entries?: number;
};

export async function getCacheStats(): Promise<CacheStats> {
  return fetchJson<CacheStats>('/api/cache/stats');
}

export async function getCacheRecommendations(): Promise<CacheRecommendations> {
  return fetchJson<CacheRecommendations>('/api/cache/recommendations');
}

function cachePost(path: string, csrfToken: string): Promise<CacheActionResult> {
  return fetchJson<CacheActionResult>(path, {
    method: 'POST',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

/** Drops every cached entry; the next requests rebuild it from the database. */
export function clearCache(csrfToken: string): Promise<CacheActionResult> {
  return cachePost('/api/cache/clear', csrfToken);
}

/** Drops only the expired entries. */
export function cleanupCache(csrfToken: string): Promise<CacheActionResult> {
  return cachePost('/api/cache/cleanup', csrfToken);
}

/** Zeroes the hit/miss counters without touching the cached entries. */
export function resetCacheCounters(csrfToken: string): Promise<CacheActionResult> {
  return cachePost('/api/cache/reset-counters', csrfToken);
}

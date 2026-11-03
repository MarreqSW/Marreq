import { afterEach, describe, expect, it, vi } from 'vitest';
import { CSRF, itSendsEachRequest, stubFetch } from '@/test/apiHarness';
import { ApiError } from '../transport';
import {
  clearCache,
  cleanupCache,
  getCacheRecommendations,
  getCacheStats,
  resetCacheCounters,
} from '../cache';

const stats = {
  total_entries: 10,
  active_entries: 8,
  expired_entries: 2,
  hits: 30,
  misses: 10,
  total_requests: 40,
  hit_rate: 0.75,
  miss_rate: 0.25,
  average_access_time_ns: 1200,
  total_access_time_ns: 48000,
  last_cleanup: '2026-10-29T08:00:00Z',
  cache_size_bytes: 4096,
};

describe('cache API', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    {
      name: 'getCacheStats',
      call: () => getCacheStats(),
      method: 'GET',
      url: '/api/cache/stats',
      response: { json: stats },
      returns: stats,
    },
    {
      name: 'getCacheRecommendations',
      call: () => getCacheRecommendations(),
      method: 'GET',
      url: '/api/cache/recommendations',
      response: {
        json: { recommendations: ['Run cleanup'], priority: 'low', analysis_timestamp: '2026-10-29T08:00:00Z' },
      },
      returns: { recommendations: ['Run cleanup'], priority: 'low', analysis_timestamp: '2026-10-29T08:00:00Z' },
    },
    {
      name: 'clearCache',
      call: () => clearCache(CSRF),
      method: 'POST',
      url: '/api/cache/clear',
      response: { json: { message: 'Cache cleared successfully', timestamp: 't' } },
      returns: { message: 'Cache cleared successfully', timestamp: 't' },
    },
    {
      name: 'cleanupCache returns the number of removed entries',
      call: () => cleanupCache(CSRF),
      method: 'POST',
      url: '/api/cache/cleanup',
      response: { json: { message: 'done', cleaned_entries: 2, timestamp: 't' } },
      returns: { message: 'done', cleaned_entries: 2, timestamp: 't' },
    },
    {
      name: 'resetCacheCounters',
      call: () => resetCacheCounters(CSRF),
      method: 'POST',
      url: '/api/cache/reset-counters',
      response: { json: { message: 'reset', timestamp: 't' } },
      returns: { message: 'reset', timestamp: 't' },
    },
  ]);

  it('rejects with an ApiError for non-administrators', async () => {
    stubFetch({ status: 403, json: { message: 'forbidden' } });
    await expect(getCacheStats()).rejects.toBeInstanceOf(ApiError);
  });
});

import { useCallback, useEffect, useState } from 'react';
import {
  type CacheActionResult,
  type CacheRecommendationPriority,
  type CacheRecommendations,
  type CacheStats,
  cleanupCache,
  clearCache,
  getCacheRecommendations,
  getCacheStats,
  getCsrfToken,
  listUsersOptional,
  resetCacheCounters,
} from '@/api/client';
import StitchPageHeader from '@/components/StitchPageHeader';
import { useDashboard } from '@/context/DashboardContext';
import { btnDanger, btnPrimary } from '@/pages/catalog/catalogUi';
import { ADMIN_BREADCRUMB } from '@/pages/admin/adminArea';
import { formatBytes } from '@/utils/formatBytes';
import { parseUser } from '@/utils/parseUser';

const CLEAR_CONFIRM =
  'Clear all cache entries? This will temporarily slow down the application until the cache is rebuilt.';
const RESET_CONFIRM = 'Reset the cache hit/miss counters? The cached entries are kept.';

const PRIORITY_BADGE: Record<CacheRecommendationPriority, string> = {
  high: 'border-red-500/30 bg-red-500/10 text-red-800 dark:text-red-100',
  medium: 'border-amber-500/30 bg-amber-500/10 text-amber-900 dark:text-amber-100',
  low: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-900 dark:text-emerald-100',
};

const cardClass = 'rounded-xl border border-stitch-border bg-stitch-surface p-5 shadow-stitch space-y-3';
const cardTitleClass = 'text-sm font-bold text-stitch-accent uppercase tracking-wide';
const btnSecondary =
  'inline-flex items-center justify-center rounded-md border border-stitch-border px-4 py-2 text-xs font-bold uppercase tracking-wider text-stitch-accent hover:bg-stitch-higher disabled:opacity-50';

/** The API reports the average access time in nanoseconds. */
function formatDuration(ns: number): string {
  if (!Number.isFinite(ns) || ns < 0) return '—';
  if (ns < 1_000) return `${Math.round(ns)} ns`;
  if (ns < 1_000_000) return `${(ns / 1_000).toFixed(1)} µs`;
  return `${(ns / 1_000_000).toFixed(2)} ms`;
}

/** `hit_rate` is a fraction in 0..1, not a percentage. */
function formatRate(rate: number): string {
  return Number.isFinite(rate) ? `${(rate * 100).toFixed(1)}%` : '—';
}

function formatTimestamp(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? '—' : date.toLocaleString();
}

function Metric({ label, value }: { label: string; value: string | number }) {
  return (
    <div>
      <dt className="text-xs uppercase tracking-wide text-stitch-muted">{label}</dt>
      <dd className="text-lg font-semibold text-stitch-fg">{value}</dd>
    </div>
  );
}

export default function CachePage() {
  const { dashboard, csrfToken } = useDashboard();
  const me = parseUser(dashboard?.user);

  const [allowed, setAllowed] = useState<boolean | null>(null);
  const [stats, setStats] = useState<CacheStats | null>(null);
  const [advice, setAdvice] = useState<CacheRecommendations | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const load = useCallback(async () => {
    const [nextStats, nextAdvice] = await Promise.all([
      getCacheStats(),
      // The recommendations are advice only; the page still works without them.
      getCacheRecommendations().catch(() => null),
    ]);
    setStats(nextStats);
    setAdvice(nextAdvice);
  }, []);

  useEffect(() => {
    let alive = true;
    // `/api/users` is admin-only, so it doubles as the access check (same as Backup).
    listUsersOptional().then((users) => {
      if (!alive) return;
      setAllowed(users !== null);
      if (users === null) return;
      load().catch((e) => {
        if (alive) setError(e instanceof Error ? e.message : 'Could not load the cache statistics');
      });
    });
    return () => {
      alive = false;
    };
  }, [load]);

  async function run(
    action: (csrf: string) => Promise<CacheActionResult>,
    describe: (result: CacheActionResult) => string,
  ) {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const result = await action(csrfToken ?? (await getCsrfToken()));
      setNotice(describe(result));
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Cache action failed');
    } finally {
      setBusy(false);
    }
  }

  function onRefresh() {
    setBusy(true);
    setError(null);
    setNotice(null);
    load()
      .catch((e) => setError(e instanceof Error ? e.message : 'Could not load the cache statistics'))
      .finally(() => setBusy(false));
  }

  function onCleanup() {
    void run(cleanupCache, (r) => {
      const n = r.cleaned_entries ?? 0;
      return `Removed ${n} expired ${n === 1 ? 'entry' : 'entries'}.`;
    });
  }

  function onClear() {
    if (!window.confirm(CLEAR_CONFIRM)) return;
    void run(clearCache, () => 'Cache cleared.');
  }

  function onResetCounters() {
    if (!window.confirm(RESET_CONFIRM)) return;
    void run(resetCacheCounters, () => 'Cache counters reset.');
  }

  if (allowed === null) {
    return (
      <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
        Loading…
      </div>
    );
  }

  if (!allowed) {
    return (
      <div>
        <StitchPageHeader
          projectName={ADMIN_BREADCRUMB}
          section="Cache"
          title="Application cache"
          subtitle="Restricted area."
        />
        <div className="rounded-xl border border-stitch-border bg-stitch-surface p-8 text-center">
          <span className="material-symbols-outlined text-4xl text-stitch-muted mb-3 block">
            lock
          </span>
          <p className="text-stitch-fg font-semibold">Access denied</p>
          <p className="text-sm text-stitch-muted mt-2 max-w-md mx-auto">
            Managing the application cache requires a global administrator account. You are signed
            in as <span className="text-stitch-accent">{me?.username ?? '?'}</span>.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div>
      <StitchPageHeader
        projectName={ADMIN_BREADCRUMB}
        section="Cache"
        title="Application cache"
        subtitle="Inspect the in-memory cache and clear it, for example after fixing data directly in the database."
      />

      {error ? (
        <div
          role="alert"
          className="mb-4 rounded-lg border border-red-500/30 bg-red-500/10 text-red-800 dark:text-red-100 text-sm px-4 py-2"
        >
          {error}
        </div>
      ) : null}
      {notice ? (
        <div
          role="status"
          className="mb-4 rounded-lg border border-emerald-500/30 bg-emerald-500/10 text-emerald-900 dark:text-emerald-100 text-sm px-4 py-2"
        >
          {notice}
        </div>
      ) : null}

      <div className="max-w-3xl space-y-4">
        <div className={cardClass}>
          <h3 className={cardTitleClass}>Entries</h3>
          {stats ? (
            <dl className="grid grid-cols-2 sm:grid-cols-4 gap-4" data-testid="cache-entries">
              <Metric label="Total" value={stats.total_entries} />
              <Metric label="Active" value={stats.active_entries} />
              <Metric label="Expired" value={stats.expired_entries} />
              <Metric label="Size" value={formatBytes(stats.cache_size_bytes)} />
            </dl>
          ) : (
            <p className="text-sm text-stitch-muted">No statistics available.</p>
          )}
        </div>

        <div className={cardClass}>
          <h3 className={cardTitleClass}>Performance</h3>
          {stats ? (
            <>
              <dl className="grid grid-cols-2 sm:grid-cols-4 gap-4" data-testid="cache-performance">
                <Metric label="Hit rate" value={formatRate(stats.hit_rate)} />
                <Metric label="Hits" value={stats.hits} />
                <Metric label="Misses" value={stats.misses} />
                <Metric label="Requests" value={stats.total_requests} />
                <Metric label="Avg access time" value={formatDuration(stats.average_access_time_ns)} />
              </dl>
              <p className="text-xs text-stitch-muted">
                Last cleanup: {formatTimestamp(stats.last_cleanup)}
              </p>
            </>
          ) : (
            <p className="text-sm text-stitch-muted">No statistics available.</p>
          )}
        </div>

        <div className={cardClass}>
          <div className="flex items-center justify-between gap-3">
            <h3 className={cardTitleClass}>Recommendations</h3>
            {advice ? (
              <span
                className={`rounded-full border px-2 py-0.5 text-xs font-bold uppercase ${PRIORITY_BADGE[advice.priority] ?? PRIORITY_BADGE.low}`}
                data-testid="cache-priority"
              >
                {advice.priority}
              </span>
            ) : null}
          </div>
          {advice && advice.recommendations.length > 0 ? (
            <ul className="list-disc pl-5 text-sm text-stitch-fg space-y-1">
              {advice.recommendations.map((text) => (
                <li key={text}>{text}</li>
              ))}
            </ul>
          ) : (
            <p className="text-sm text-stitch-muted">No recommendations.</p>
          )}
          {advice ? (
            <p className="text-xs text-stitch-muted">
              Analysis performed at {formatTimestamp(advice.analysis_timestamp)}
            </p>
          ) : null}
        </div>

        <div className={cardClass}>
          <h3 className={cardTitleClass}>Actions</h3>
          <p className="text-sm text-stitch-muted">
            Clean up removes only expired entries. Clear all drops every entry, so the next
            requests rebuild the cache from the database. Resetting the counters keeps the
            entries.
          </p>
          <div className="flex flex-wrap gap-2">
            <button type="button" className={btnSecondary} onClick={onRefresh} disabled={busy}>
              Refresh
            </button>
            <button type="button" className={btnPrimary} onClick={onCleanup} disabled={busy}>
              Clean up expired
            </button>
            <button type="button" className={btnSecondary} onClick={onResetCounters} disabled={busy}>
              Reset counters
            </button>
            <button type="button" className={btnDanger} onClick={onClear} disabled={busy}>
              Clear all
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

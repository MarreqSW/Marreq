import { useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { getAdminLogStats, listUsersOptional } from '@/api/client';
import type { AdminLogStats } from '@/api/types';
import DailyBarChart, { formatDay } from '@/components/analytics/DailyBarChart';
import RankedBars from '@/components/analytics/RankedBars';
import StitchPageHeader from '@/components/StitchPageHeader';
import { useDashboard } from '@/context/DashboardContext';
import { parseUser } from '@/utils/parseUser';
import { ADMIN_BASE, ADMIN_BREADCRUMB } from '@/pages/admin/adminArea';

const RANGES = [7, 30, 90] as const;
type RangeDays = (typeof RANGES)[number];
const DEFAULT_RANGE: RangeDays = 30;
const TOP = 10;

const card = 'rounded-xl border border-stitch-border bg-stitch-surface p-5 shadow-stitch';

/** UTC midnight `days - 1` days before today, as `YYYY-MM-DD` (so the range covers `days` days). */
export function rangeStart(days: number, now: Date = new Date()): string {
  const start = new Date(
    Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate() - (days - 1)),
  );
  return start.toISOString().slice(0, 10);
}

function parseRange(raw: string | null): RangeDays {
  const n = Number(raw);
  return (RANGES as readonly number[]).includes(n) ? (n as RangeDays) : DEFAULT_RANGE;
}

function Tile({ label, value, hint }: { label: string; value: string | number; hint?: string }) {
  return (
    <div className={card}>
      <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-widest">{label}</p>
      <p className="text-3xl font-extrabold text-stitch-fg mt-2 tabular-nums">{value}</p>
      {hint ? <p className="text-xs text-stitch-muted mt-2">{hint}</p> : null}
    </div>
  );
}

export default function LogAnalyticsPage() {
  const { dashboard } = useDashboard();
  const me = parseUser(dashboard?.user);
  const [searchParams, setSearchParams] = useSearchParams();
  const days = parseRange(searchParams.get('days'));

  const [allowed, setAllowed] = useState<boolean | null>(null);
  const [stats, setStats] = useState<AdminLogStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    listUsersOptional().then((users) => {
      if (alive) setAllowed(users !== null);
    });
    return () => {
      alive = false;
    };
  }, []);

  const since = rangeStart(days);

  useEffect(() => {
    if (!allowed) return;
    let alive = true;
    setLoading(true);
    setError(null);
    getAdminLogStats({ since, top: TOP })
      .then((s) => {
        if (alive) setStats(s);
      })
      .catch((e) => {
        if (alive) setError(e instanceof Error ? e.message : 'Failed to load analytics');
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [allowed, since]);

  const busiest = useMemo(() => {
    if (!stats || stats.total === 0) return null;
    return stats.by_day.reduce((best, d) => (d.count > best.count ? d : best), stats.by_day[0]);
  }, [stats]);

  const projectName = ADMIN_BREADCRUMB;
  const logsLink = (params: Record<string, string>) =>
    `${ADMIN_BASE}/logs?${new URLSearchParams({ since: `${since}T00:00`, ...params })}`;

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
          projectName={projectName}
          section="Admin"
          title="Log analytics"
          subtitle="Restricted area."
        />
        <div className="rounded-xl border border-stitch-border bg-stitch-surface p-8 text-center">
          <span className="material-symbols-outlined text-4xl text-stitch-muted mb-3 block">
            lock
          </span>
          <p className="text-stitch-fg font-semibold">Access denied</p>
          <p className="text-sm text-stitch-muted mt-2 max-w-md mx-auto">
            Log analytics requires a global administrator account. You are signed in as{' '}
            <span className="text-stitch-accent">{me?.username ?? '?'}</span>.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div>
      <StitchPageHeader
        projectName={projectName}
        section="Admin"
        title="Log analytics"
        subtitle="Instance-wide activity from the audit log. Days are in UTC."
      />

      <div className="mb-4 flex items-center gap-2" role="group" aria-label="Time range">
        {RANGES.map((r) => (
          <button
            key={r}
            type="button"
            aria-pressed={r === days}
            onClick={() => setSearchParams(r === DEFAULT_RANGE ? {} : { days: String(r) })}
            className={`rounded-md border px-3 py-1.5 text-xs font-bold uppercase tracking-wider ${
              r === days
                ? 'border-stitch-accent bg-stitch-accent text-stitch-canvas'
                : 'border-stitch-border text-stitch-muted hover:text-stitch-fg'
            }`}
          >
            {r} days
          </button>
        ))}
      </div>

      {error ? (
        <div
          role="alert"
          className="mb-4 rounded-lg border border-red-500/30 bg-red-500/10 text-red-800 dark:text-red-100 text-sm px-4 py-2"
        >
          {error}
        </div>
      ) : null}

      {loading && !stats ? (
        <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
          Loading…
        </div>
      ) : stats ? (
        <div className={`space-y-4 ${loading ? 'opacity-60' : ''}`} aria-busy={loading}>
          <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
            <Tile label="Events" value={stats.total} hint={`Last ${days} days`} />
            <Tile
              label="Average per day"
              value={stats.by_day.length ? (stats.total / stats.by_day.length).toFixed(1) : '0'}
            />
            <Tile label="Active users" value={stats.active_users} />
            <Tile
              label="Busiest day"
              value={busiest ? busiest.count : '—'}
              hint={busiest ? formatDay(busiest.day) : undefined}
            />
          </div>

          {stats.total === 0 ? (
            <div className={`${card} text-center text-sm text-stitch-muted`} data-testid="analytics-empty">
              No activity in this period.
            </div>
          ) : (
            <>
              <section className={card}>
                <h3 className="mb-4 text-sm font-bold uppercase tracking-wide text-stitch-accent">
                  Events per day
                </h3>
                <DailyBarChart data={stats.by_day} label="Events per day" />
              </section>
              <div className="grid gap-4 lg:grid-cols-2">
                <section className={card}>
                  <h3 className="mb-4 text-sm font-bold uppercase tracking-wide text-stitch-accent">
                    Top actions
                  </h3>
                  <RankedBars
                    label="Top actions"
                    items={stats.by_action.map((a) => ({
                      key: a.action_type,
                      label: a.action_type,
                      count: a.count,
                      to: logsLink({ action_type: a.action_type }),
                    }))}
                  />
                </section>
                <section className={card}>
                  <h3 className="mb-4 text-sm font-bold uppercase tracking-wide text-stitch-accent">
                    Top users
                  </h3>
                  <RankedBars
                    label="Top users"
                    items={stats.by_user.map((u) => ({
                      key: String(u.user_id),
                      label: u.username,
                      count: u.count,
                      to: logsLink({ user_id: String(u.user_id) }),
                    }))}
                  />
                </section>
              </div>
            </>
          )}
        </div>
      ) : null}
    </div>
  );
}

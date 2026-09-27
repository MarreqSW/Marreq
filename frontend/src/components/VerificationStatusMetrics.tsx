import { useMemo } from 'react';
import type { Verification, VerificationStatus } from '@/api/types';

const HEX6 = /^#[0-9A-Fa-f]{6}$/;

type VerificationStatusMetricsProps = {
  /** All verifications of the project (counts ignore the current filters). */
  verifications: Verification[];
  /** Project statuses, in display order. */
  statuses: VerificationStatus[];
  activeStatusId: number | null;
  /** Called with a status id to filter by it, or `null` to clear the filter. */
  onSelectStatus: (statusId: number | null) => void;
};

export type StatusMetrics = {
  total: number;
  byStatus: { status: VerificationStatus; count: number }[];
  /** Verifications whose status is not in `statuses` (keeps the parts summing to the total). */
  other: number;
  /** Count for the status titled "Passed", or `null` when the project has no such status. */
  passed: number | null;
};

export function computeStatusMetrics(
  verifications: Verification[],
  statuses: VerificationStatus[],
): StatusMetrics {
  const counts = new Map<number, number>();
  for (const v of verifications) counts.set(v.status_id, (counts.get(v.status_id) ?? 0) + 1);

  const byStatus = statuses.map((status) => ({ status, count: counts.get(status.id) ?? 0 }));
  const known = byStatus.reduce((sum, s) => sum + s.count, 0);
  const passedStatus = statuses.find((s) => s.title.trim().toLowerCase() === 'passed');

  return {
    total: verifications.length,
    byStatus,
    other: verifications.length - known,
    passed: passedStatus ? (counts.get(passedStatus.id) ?? 0) : null,
  };
}

/**
 * Project-wide verification counts by status, with a pass rate when a "Passed"
 * status exists. Each status chip toggles the list's status filter.
 */
export default function VerificationStatusMetrics({
  verifications,
  statuses,
  activeStatusId,
  onSelectStatus,
}: VerificationStatusMetricsProps) {
  const metrics = useMemo(
    () => computeStatusMetrics(verifications, statuses),
    [verifications, statuses],
  );
  const passRate =
    metrics.passed !== null && metrics.total > 0
      ? Math.round((metrics.passed / metrics.total) * 100)
      : null;

  const chip =
    'flex items-center gap-2 rounded-lg border px-3 py-2 text-left text-xs transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-stitch-accent';

  return (
    <section
      aria-label="Verification status summary"
      className="bg-stitch-surface p-4 rounded-xl border border-stitch-border shadow-stitch flex flex-wrap items-stretch gap-3"
    >
      <div className="pr-4 mr-1 border-r border-stitch-border flex flex-col justify-center">
        <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-widest">Total</p>
        <p className="text-2xl font-extrabold text-stitch-fg tabular-nums" data-testid="metric-total">
          {metrics.total}
        </p>
      </div>

      {passRate !== null ? (
        <div className="pr-4 mr-1 border-r border-stitch-border flex flex-col justify-center min-w-[9rem]">
          <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-widest">
            Pass rate
          </p>
          <p className="text-2xl font-extrabold text-stitch-fg tabular-nums" data-testid="metric-pass-rate">
            {passRate}%
          </p>
          <div className="mt-1 h-1.5 rounded-full bg-stitch-elevated" aria-hidden="true">
            <div
              className="h-1.5 rounded-full bg-stitch-accent"
              style={{ width: `${passRate}%` }}
            />
          </div>
          <p className="mt-1 text-[10px] text-stitch-muted">
            {metrics.passed} of {metrics.total} passed
          </p>
        </div>
      ) : null}

      <div className="flex flex-wrap items-center gap-2" role="group" aria-label="Filter by status">
        {metrics.byStatus.map(({ status, count }) => {
          const active = activeStatusId === status.id;
          const swatch = status.tag_color && HEX6.test(status.tag_color) ? status.tag_color : undefined;
          return (
            <button
              key={status.id}
              type="button"
              aria-pressed={active}
              data-testid="status-chip"
              title={active ? `Clear filter: ${status.title}` : `Show only ${status.title}`}
              onClick={() => onSelectStatus(active ? null : status.id)}
              className={`${chip} ${
                active
                  ? 'border-stitch-accent bg-stitch-accent/10 text-stitch-fg'
                  : 'border-stitch-border bg-stitch-elevated hover:border-stitch-accent/40'
              } ${count === 0 && !active ? 'opacity-60' : ''}`}
            >
              <span
                className={`h-2.5 w-2.5 shrink-0 rounded-full ${swatch ? '' : 'bg-stitch-muted'}`}
                style={swatch ? { backgroundColor: swatch } : undefined}
                aria-hidden="true"
              />
              <span className="text-stitch-fg">{status.title}</span>
              <span className="font-bold tabular-nums text-stitch-fg">{count}</span>
            </button>
          );
        })}
        {metrics.other > 0 ? (
          <span
            className={`${chip} border-dashed border-stitch-border text-stitch-muted`}
            data-testid="status-chip-other"
            title="Verifications whose status is not in this project's status list"
          >
            <span className="text-stitch-fg">Other</span>
            <span className="font-bold tabular-nums text-stitch-fg">{metrics.other}</span>
          </span>
        ) : null}
      </div>
    </section>
  );
}

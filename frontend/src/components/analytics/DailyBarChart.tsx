import { useState } from 'react';

export type DailyPoint = { day: string; count: number };

type DailyBarChartProps = {
  data: DailyPoint[];
  /** Accessible name for the chart, e.g. "Events per day". */
  label: string;
};

/** `2026-09-01` → `1 Sep` (UTC, no timezone shift). */
export function formatDay(day: string): string {
  const [y, m, d] = day.split('-').map(Number);
  return new Date(Date.UTC(y, m - 1, d)).toLocaleDateString(undefined, {
    day: 'numeric',
    month: 'short',
    timeZone: 'UTC',
  });
}

/** Keep edge tooltips inside the chart: left-align near the start, right-align near the end. */
function tooltipAnchor(i: number, n: number): string {
  if (i < n * 0.15) return 'left-0';
  if (i >= n * 0.85) return 'right-0';
  return '';
}

/**
 * Single-series column chart: one bar per day grown from a shared baseline, a
 * recessive max gridline, sparse date labels, a per-bar hover/focus tooltip,
 * and a table view so no value depends on hovering.
 */
export default function DailyBarChart({ data, label }: DailyBarChartProps) {
  const [active, setActive] = useState<number | null>(null);
  const max = Math.max(0, ...data.map((d) => d.count));
  const ticks =
    data.length > 2 ? [0, Math.floor((data.length - 1) / 2), data.length - 1] : data.map((_, i) => i);

  return (
    <figure className="m-0">
      <div className="relative">
        <div className="absolute inset-x-0 top-0 flex items-center gap-2 text-[10px] text-stitch-muted tabular-nums">
          <span>{max}</span>
          <span className="h-px flex-1 bg-stitch-border" aria-hidden="true" />
        </div>
        <div
          className="flex h-44 items-end gap-[2px] border-b border-stitch-border pt-4"
          role="group"
          aria-label={label}
          onMouseLeave={() => setActive(null)}
        >
          {data.map((d, i) => {
            const pct = max > 0 ? (d.count / max) * 100 : 0;
            const text = `${formatDay(d.day)}: ${d.count} ${d.count === 1 ? 'event' : 'events'}`;
            return (
              <button
                key={d.day}
                type="button"
                aria-label={text}
                title={text}
                data-testid="daily-bar"
                className="group relative flex h-full min-w-0 flex-1 items-end justify-center focus:outline-hidden"
                onMouseEnter={() => setActive(i)}
                onFocus={() => setActive(i)}
                onBlur={() => setActive(null)}
              >
                <span
                  className={`block w-full max-w-6 rounded-t-[4px] transition-opacity ${
                    d.count > 0 ? 'bg-stitch-accent' : ''
                  } ${active !== null && active !== i ? 'opacity-50' : ''} group-focus-visible:ring-2 group-focus-visible:ring-stitch-accent/50`}
                  style={{ height: d.count > 0 ? `max(${pct}%, 2px)` : 0 }}
                />
                {active === i ? (
                  <span
                    role="tooltip"
                    className={`pointer-events-none absolute bottom-full z-10 mb-1 whitespace-nowrap rounded-md border border-stitch-border bg-stitch-surface px-2 py-1 text-xs text-stitch-fg shadow-stitch ${tooltipAnchor(i, data.length)}`}
                  >
                    <span className="font-semibold">{formatDay(d.day)}</span>
                    <span className="text-stitch-muted"> · </span>
                    <span className="tabular-nums">{d.count}</span>
                  </span>
                ) : null}
              </button>
            );
          })}
        </div>
        <div className="relative mt-1 h-4 text-[10px] text-stitch-muted" aria-hidden="true">
          {ticks.map((i) => (
            <span
              key={i}
              className="absolute whitespace-nowrap"
              style={{
                left: `${((i + 0.5) / data.length) * 100}%`,
                transform:
                  i === 0 ? 'translateX(-25%)' : i === data.length - 1 ? 'translateX(-75%)' : 'translateX(-50%)',
              }}
            >
              {formatDay(data[i].day)}
            </span>
          ))}
        </div>
      </div>
      <details className="mt-3 text-xs text-stitch-muted">
        <summary className="cursor-pointer select-none">Show as table</summary>
        <table className="mt-2 w-full max-w-xs text-left">
          <thead>
            <tr>
              <th className="py-1 font-semibold">Day (UTC)</th>
              <th className="py-1 text-right font-semibold">Events</th>
            </tr>
          </thead>
          <tbody>
            {data.map((d) => (
              <tr key={d.day} className="border-t border-stitch-border">
                <td className="py-1 text-stitch-fg">{d.day}</td>
                <td className="py-1 text-right tabular-nums text-stitch-fg">{d.count}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </details>
    </figure>
  );
}

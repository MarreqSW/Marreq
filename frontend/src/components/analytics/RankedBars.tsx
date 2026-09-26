import { Link } from 'react-router-dom';

export type RankedItem = {
  key: string;
  label: string;
  count: number;
  /** Optional drill-down route (e.g. System logs pre-filtered to this item). */
  to?: string;
};

type RankedBarsProps = {
  items: RankedItem[];
  /** Accessible name for the list, e.g. "Top actions". */
  label: string;
  emptyText?: string;
};

/** Ranked horizontal bars: label and count in text ink, bar length = count ÷ max. */
export default function RankedBars({ items, label, emptyText = 'No data' }: RankedBarsProps) {
  if (items.length === 0) {
    return <p className="text-sm text-stitch-muted">{emptyText}</p>;
  }
  const max = Math.max(...items.map((i) => i.count));

  return (
    <ol className="space-y-2.5" aria-label={label}>
      {items.map((item) => {
        const pct = max > 0 ? (item.count / max) * 100 : 0;
        const name = item.to ? (
          <Link to={item.to} className="truncate text-stitch-fg hover:text-stitch-accent hover:underline">
            {item.label}
          </Link>
        ) : (
          <span className="truncate text-stitch-fg">{item.label}</span>
        );
        return (
          <li key={item.key} data-testid="ranked-row">
            <div className="flex items-baseline justify-between gap-3 text-sm">
              {name}
              <span className="shrink-0 tabular-nums text-stitch-muted">{item.count}</span>
            </div>
            <div className="mt-1 h-2 rounded-r-[4px] bg-stitch-elevated" aria-hidden="true">
              <div
                className="h-2 rounded-r-[4px] bg-stitch-accent"
                style={{ width: `max(${pct}%, 2px)` }}
              />
            </div>
          </li>
        );
      })}
    </ol>
  );
}

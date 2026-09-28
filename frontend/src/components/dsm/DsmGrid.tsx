import { useMemo, useState, type MouseEvent } from 'react';
import { Link } from 'react-router-dom';
import type { Dsm, DsmCell } from '@/api/types';
import { buildCellMap, cellAtPoint, cellKey, linkTypeMeta, loopNumberByRequirement } from '@/utils/dsm';

/** Cell edge in px; the body is `n × CELL` square. */
export const DSM_CELL = 26;
const ROW_HEADER_W = 300;
const COL_HEADER_H = 112;

const APPROVAL_DOT: Record<string, string> = {
  approved: 'bg-green-600',
  reviewed: 'bg-amber-500',
};

type Props = {
  dsm: Dsm;
  basePath: string;
  /** Requirement ids of the loop highlighted from the side panel. */
  highlightIds?: Set<number>;
  onOpenRequirement: (requirementId: number) => void;
};

/**
 * Sparse DSM rendering: grid lines are a CSS background, and only the
 * diagonal, marks, group frames and hover overlays are elements, so the DOM
 * grows with requirements + links rather than requirements².
 */
export default function DsmGrid({ dsm, basePath, highlightIds, onOpenRequirement }: Props) {
  const n = dsm.requirements.length;
  const size = n * DSM_CELL;
  const cellMap = useMemo(() => buildCellMap(dsm.cells), [dsm.cells]);
  const loopOf = useMemo(() => loopNumberByRequirement(dsm.loops), [dsm.loops]);
  const [hover, setHover] = useState<{ row: number; col: number; x: number; y: number } | null>(null);

  const groupStart = useMemo(() => new Map(dsm.groups.map((g) => [g.start, g.label])), [dsm.groups]);

  const hoverCell: DsmCell | undefined = hover ? cellMap.get(cellKey(hover.row, hover.col)) : undefined;

  const pointFromEvent = (e: MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    return cellAtPoint(e.clientX - rect.left, e.clientY - rect.top, DSM_CELL, n);
  };

  const onMove = (e: MouseEvent<HTMLDivElement>) => {
    const at = pointFromEvent(e);
    setHover(at ? { ...at, x: e.clientX, y: e.clientY } : null);
  };

  const onClick = (e: MouseEvent<HTMLDivElement>) => {
    const at = pointFromEvent(e);
    if (!at) return;
    if (at.row === at.col || cellMap.has(cellKey(at.row, at.col))) {
      onOpenRequirement(dsm.requirements[at.row]!.id);
    }
  };

  if (n === 0) {
    return (
      <div className="rounded-xl border border-stitch-border bg-stitch-surface p-8 text-sm text-stitch-muted">
        No requirements in this scope.
      </div>
    );
  }

  const gridLines = {
    backgroundImage:
      'linear-gradient(to right, var(--color-stitch-border) 1px, transparent 1px), linear-gradient(to bottom, var(--color-stitch-border) 1px, transparent 1px)',
    backgroundSize: `${DSM_CELL}px ${DSM_CELL}px`,
  };

  return (
    <div
      className="relative overflow-auto max-h-[70vh] rounded-xl border border-stitch-border bg-stitch-surface shadow-stitch"
      data-testid="dsm-grid"
    >
      {/* w-max: the grid box must span all tracks, or the sticky row headers
          (bounded by it) scroll away once the matrix is wider than the viewport. */}
      <div
        className="grid w-max"
        style={{
          gridTemplateColumns: `${ROW_HEADER_W}px ${size}px`,
          gridTemplateRows: `${COL_HEADER_H}px ${size}px`,
        }}
      >
        {/* Corner */}
        <div className="sticky top-0 left-0 z-30 bg-stitch-surface border-b border-r border-stitch-border flex items-end px-3 pb-2 text-[10px] uppercase tracking-widest font-bold text-stitch-muted">
          Requirement ↓ depends on →
        </div>

        {/* Column headers */}
        <div className="sticky top-0 z-20 bg-stitch-surface border-b border-stitch-border">
          {dsm.requirements.map((r) => (
            <div
              key={r.id}
              className={`absolute bottom-0 flex flex-col items-center justify-end ${
                hover?.col === r.index ? 'bg-stitch-accent/10' : ''
              }`}
              style={{ left: r.index * DSM_CELL, width: DSM_CELL, height: COL_HEADER_H }}
              title={`${r.reference_code} — ${r.title}`}
            >
              <span className="[writing-mode:vertical-rl] rotate-180 font-mono text-[10.5px] font-semibold text-stitch-accent whitespace-nowrap overflow-hidden max-h-[88px]">
                {r.reference_code}
              </span>
              <span className="font-mono text-[9.5px] text-stitch-subtle pb-1">{r.index + 1}</span>
            </div>
          ))}
        </div>

        {/* Row headers */}
        <div className="sticky left-0 z-10 bg-stitch-surface border-r border-stitch-border">
          {dsm.requirements.map((r) => {
            const label = groupStart.get(r.index);
            return (
              <div
                key={r.id}
                className={`absolute left-0 right-0 flex items-center gap-2 px-2 border-b border-stitch-border/60 ${
                  hover?.row === r.index ? 'bg-stitch-accent/10' : ''
                }`}
                style={{ top: r.index * DSM_CELL, height: DSM_CELL, paddingLeft: 8 + Math.min(r.depth, 4) * 8 }}
              >
                <span className="w-6 shrink-0 text-right font-mono text-[10px] text-stitch-subtle">
                  {r.index + 1}
                </span>
                <span
                  className={`h-2 w-2 shrink-0 rounded-full ${
                    APPROVAL_DOT[r.approval_state] ?? 'border border-stitch-muted'
                  }`}
                  title={r.approval_state}
                  aria-label={r.approval_state}
                />
                <Link
                  to={`${basePath}/requirements/${r.id}`}
                  className="font-mono text-[11.5px] font-semibold text-stitch-accent hover:underline shrink-0"
                >
                  {r.reference_code}
                </Link>
                <span className="truncate text-[11.5px] text-stitch-muted" title={r.title}>
                  {r.title}
                </span>
                {label ? (
                  <span className="ml-auto shrink-0 rounded bg-stitch-elevated px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wider text-stitch-muted">
                    {label}
                  </span>
                ) : null}
              </div>
            );
          })}
        </div>

        {/* Matrix body */}
        <div
          className="relative cursor-crosshair"
          style={gridLines}
          onMouseMove={onMove}
          onMouseLeave={() => setHover(null)}
          onClick={onClick}
          data-testid="dsm-body"
        >
          {hover ? (
            <>
              <div
                className="absolute left-0 right-0 bg-stitch-accent/8 pointer-events-none"
                style={{ top: hover.row * DSM_CELL, height: DSM_CELL }}
              />
              <div
                className="absolute top-0 bottom-0 bg-stitch-accent/8 pointer-events-none"
                style={{ left: hover.col * DSM_CELL, width: DSM_CELL }}
              />
            </>
          ) : null}

          {dsm.groups.map((g) => (
            <div
              key={`${g.label}-${g.start}`}
              className="absolute border-2 border-stitch-accent/50 rounded-sm pointer-events-none"
              style={{
                left: g.start * DSM_CELL,
                top: g.start * DSM_CELL,
                width: (g.end - g.start + 1) * DSM_CELL,
                height: (g.end - g.start + 1) * DSM_CELL,
              }}
              title={g.label}
            />
          ))}

          {dsm.requirements.map((r) => (
            <div
              key={r.id}
              className="absolute bg-stitch-muted pointer-events-none"
              style={{ left: r.index * DSM_CELL, top: r.index * DSM_CELL, width: DSM_CELL, height: DSM_CELL }}
            />
          ))}

          {dsm.cells.map((c) => {
            const source = dsm.requirements[c.row]!;
            const highlighted = highlightIds?.has(source.id) && highlightIds.has(dsm.requirements[c.col]!.id);
            const first = linkTypeMeta(c.link_types[0] ?? '');
            return (
              <div
                key={cellKey(c.row, c.col)}
                data-testid={`dsm-cell-${c.row}-${c.col}`}
                className={`absolute flex items-center justify-center font-mono text-[11px] font-bold pointer-events-none ${first.text} ${
                  c.in_loop ? 'bg-amber-400/20' : ''
                } ${c.upstream_changed ? 'shadow-[inset_0_0_0_2px_var(--color-stitch-danger)]' : ''} ${
                  highlighted ? 'outline-2 outline-amber-500' : ''
                }`}
                style={{ left: c.col * DSM_CELL, top: c.row * DSM_CELL, width: DSM_CELL, height: DSM_CELL }}
              >
                {c.link_types.map((t) => linkTypeMeta(t).letter).join('')}
                {c.upstream_changed ? (
                  <span className="absolute top-0.5 right-0.5 h-1.5 w-1.5 rounded-full bg-stitch-danger" />
                ) : null}
              </div>
            );
          })}

          {hover && (hoverCell || hover.row === hover.col) ? (
            <HoverCard
              dsm={dsm}
              row={hover.row}
              col={hover.col}
              cell={hoverCell}
              loopNumber={
                hoverCell?.in_loop ? loopOf.get(dsm.requirements[hover.row]!.id) : undefined
              }
              x={hover.x}
              y={hover.y}
            />
          ) : null}
        </div>
      </div>
    </div>
  );
}

function HoverCard({
  dsm,
  row,
  col,
  cell,
  loopNumber,
  x,
  y,
}: {
  dsm: Dsm;
  row: number;
  col: number;
  cell: DsmCell | undefined;
  loopNumber: number | undefined;
  /** Mouse position in viewport coordinates. */
  x: number;
  y: number;
}) {
  const source = dsm.requirements[row]!;
  const target = dsm.requirements[col]!;
  // Fixed to the viewport so the scrolling matrix never clips it; flip left/up near the edges.
  const width = 300;
  const estimatedHeight = 150;
  const left = x + 16 + width > window.innerWidth ? Math.max(8, x - 16 - width) : x + 16;
  const top = y + 16 + estimatedHeight > window.innerHeight ? Math.max(8, y - 16 - estimatedHeight) : y + 16;
  return (
    <div
      role="tooltip"
      className="fixed z-50 rounded-lg border border-stitch-border bg-stitch-surface p-3 text-xs shadow-stitch pointer-events-none"
      style={{ left, top, width }}
    >
      {cell ? (
        <>
          <p className="font-mono font-bold text-stitch-accent">
            {source.reference_code} → {target.reference_code}
          </p>
          <p className="mt-1 text-stitch-fg">
            {cell.link_types.map((t) => linkTypeMeta(t).label).join(' · ')}
            {cell.link_ids.length > 1 ? ` (${cell.link_ids.length} links)` : ''}
          </p>
          <p className="mt-1 text-stitch-muted truncate">{target.title}</p>
          {loopNumber ? (
            <p className="mt-1 text-amber-700 dark:text-amber-300 font-semibold">Part of loop {loopNumber}</p>
          ) : null}
          {cell.upstream_changed ? (
            <p className="mt-2 rounded-r border-l-2 border-stitch-danger bg-stitch-danger/10 px-2 py-1 text-stitch-fg">
              <b>Upstream changed.</b> {target.reference_code} was edited after {source.reference_code} was approved.
            </p>
          ) : null}
        </>
      ) : (
        <>
          <p className="font-mono font-bold text-stitch-accent">{source.reference_code}</p>
          <p className="mt-1 text-stitch-fg">{source.title}</p>
          <p className="mt-1 text-stitch-muted">
            {source.category} · {source.approval_state}
          </p>
        </>
      )}
    </div>
  );
}

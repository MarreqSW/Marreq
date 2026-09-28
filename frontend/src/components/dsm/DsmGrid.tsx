import { useEffect, useMemo, useRef, useState, type MouseEvent } from 'react';
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

/** Area of the matrix to bring into view and frame (inclusive indices). */
export type DsmFocus = {
  /** Changes whenever a new focus is requested, so the same area can be re-centred. */
  key: string;
  kind: 'loop' | 'finding';
  rows: [number, number];
  cols: [number, number];
};

type Props = {
  dsm: Dsm;
  basePath: string;
  /** Requirement ids of the loop highlighted from the side panel. */
  highlightIds?: Set<number>;
  /** Selected problem: scrolled into the centre of the matrix and framed. */
  focus?: DsmFocus | null;
  onOpenRequirement: (requirementId: number) => void;
};

/** Scroll offsets that centre `focus` in the part of the viewport not covered by the sticky headers. */
export function focusScrollPosition(
  focus: Pick<DsmFocus, 'rows' | 'cols'>,
  viewport: { width: number; height: number },
): { left: number; top: number } {
  const bodyWidth = Math.max(0, viewport.width - ROW_HEADER_W);
  const bodyHeight = Math.max(0, viewport.height - COL_HEADER_H);
  const centreX = ((focus.cols[0] + focus.cols[1] + 1) / 2) * DSM_CELL;
  const centreY = ((focus.rows[0] + focus.rows[1] + 1) / 2) * DSM_CELL;
  return {
    left: Math.max(0, Math.round(centreX - bodyWidth / 2)),
    top: Math.max(0, Math.round(centreY - bodyHeight / 2)),
  };
}

/**
 * Sparse DSM rendering: grid lines are a CSS background, and only the
 * diagonal, marks, group frames and hover overlays are elements, so the DOM
 * grows with requirements + links rather than requirements².
 */
export default function DsmGrid({ dsm, basePath, highlightIds, focus, onOpenRequirement }: Props) {
  const n = dsm.requirements.length;
  const size = n * DSM_CELL;
  const cellMap = useMemo(() => buildCellMap(dsm.cells), [dsm.cells]);
  const loopOf = useMemo(() => loopNumberByRequirement(dsm.loops), [dsm.loops]);
  const [hover, setHover] = useState<{ row: number; col: number; x: number; y: number } | null>(null);
  /** Requirement hovered in the row or column header. */
  const [headerHover, setHeaderHover] = useState<{ index: number; axis: 'row' | 'col'; x: number; y: number } | null>(
    null,
  );
  const hoveredRow = hover?.row ?? (headerHover?.axis === 'row' ? headerHover.index : undefined);
  const hoveredCol = hover?.col ?? (headerHover?.axis === 'col' ? headerHover.index : undefined);

  const headerHandlers = (index: number, axis: 'row' | 'col') => ({
    onMouseEnter: (e: MouseEvent<HTMLElement>) => setHeaderHover({ index, axis, x: e.clientX, y: e.clientY }),
    onMouseMove: (e: MouseEvent<HTMLElement>) => setHeaderHover({ index, axis, x: e.clientX, y: e.clientY }),
    onMouseLeave: () => setHeaderHover(null),
  });

  const groupStart = useMemo(() => new Map(dsm.groups.map((g) => [g.start, g.label])), [dsm.groups]);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Bring a newly selected problem into view: the page first, then the matrix itself.
  useEffect(() => {
    const el = scrollRef.current;
    if (!focus || !el) return;
    el.scrollIntoView?.({ block: 'nearest', behavior: 'smooth' });
    const { left, top } = focusScrollPosition(focus, { width: el.clientWidth, height: el.clientHeight });
    if (typeof el.scrollTo === 'function') el.scrollTo({ left, top, behavior: 'smooth' });
    else {
      el.scrollLeft = left;
      el.scrollTop = top;
    }
    // Only a new focus request (new key) re-centres the matrix, not every re-render.
  }, [focus?.key]);

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
      ref={scrollRef}
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
              data-testid={`dsm-col-${r.index}`}
              className={`absolute bottom-0 flex flex-col items-center justify-end cursor-default ${
                hoveredCol === r.index ? 'bg-stitch-accent/10' : ''
              }`}
              style={{ left: r.index * DSM_CELL, width: DSM_CELL, height: COL_HEADER_H }}
              {...headerHandlers(r.index, 'col')}
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
                data-testid={`dsm-row-${r.index}`}
                className={`absolute left-0 right-0 flex items-center gap-2 px-2 border-b border-stitch-border/60 ${
                  hoveredRow === r.index ? 'bg-stitch-accent/10' : ''
                }`}
                style={{ top: r.index * DSM_CELL, height: DSM_CELL, paddingLeft: 8 + Math.min(r.depth, 4) * 8 }}
                {...headerHandlers(r.index, 'row')}
              >
                <span className="w-6 shrink-0 text-right font-mono text-[10px] text-stitch-subtle">
                  {r.index + 1}
                </span>
                <span
                  className={`h-2 w-2 shrink-0 rounded-full ${
                    APPROVAL_DOT[r.approval_state] ?? 'border border-stitch-muted'
                  }`}
                  aria-label={r.approval_state}
                />
                <Link
                  to={`${basePath}/requirements/${r.id}`}
                  className="font-mono text-[11.5px] font-semibold text-stitch-accent hover:underline shrink-0"
                >
                  {r.reference_code}
                </Link>
                <span className="truncate text-[11.5px] text-stitch-muted">
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
          {hoveredRow != null ? (
            <div
              className="absolute left-0 right-0 bg-stitch-accent/8 pointer-events-none"
              style={{ top: hoveredRow * DSM_CELL, height: DSM_CELL }}
            />
          ) : null}
          {hoveredCol != null ? (
            <div
              className="absolute top-0 bottom-0 bg-stitch-accent/8 pointer-events-none"
              style={{ left: hoveredCol * DSM_CELL, width: DSM_CELL }}
            />
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

          {focus ? (
            <div
              data-testid="dsm-focus"
              className={`absolute rounded-sm border-2 border-dashed pointer-events-none ${
                focus.kind === 'loop' ? 'border-amber-500 bg-amber-400/10' : 'border-stitch-danger bg-stitch-danger/10'
              }`}
              style={{
                left: focus.cols[0] * DSM_CELL - 3,
                top: focus.rows[0] * DSM_CELL - 3,
                width: (focus.cols[1] - focus.cols[0] + 1) * DSM_CELL + 6,
                height: (focus.rows[1] - focus.rows[0] + 1) * DSM_CELL + 6,
              }}
            />
          ) : null}

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
      {headerHover ? (
        <FloatingCard x={headerHover.x} y={headerHover.y}>
          <RequirementSummary dsm={dsm} index={headerHover.index} />
        </FloatingCard>
      ) : null}
    </div>
  );
}

/** Viewport-fixed card at the mouse, flipped left/up near the edges so it is never clipped. */
function FloatingCard({ x, y, children }: { x: number; y: number; children: React.ReactNode }) {
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
      {children}
    </div>
  );
}

/** Code, title, category, approval state, parent and link counts of one requirement. */
function RequirementSummary({ dsm, index }: { dsm: Dsm; index: number }) {
  const r = dsm.requirements[index]!;
  const parent = r.parent_id != null ? dsm.requirements.find((p) => p.id === r.parent_id) : undefined;
  const dependsOn = dsm.cells.filter((c) => c.row === index).length;
  const usedBy = dsm.cells.filter((c) => c.col === index).length;
  return (
    <>
      <p className="font-mono font-bold text-stitch-accent">{r.reference_code}</p>
      <p className="mt-1 font-semibold text-stitch-fg">{r.title}</p>
      <p className="mt-1 text-stitch-muted">
        {r.category} · {r.approval_state}
        {parent ? ` · parent ${parent.reference_code}` : ''}
      </p>
      <p className="mt-1 text-stitch-muted">
        Depends on {dependsOn} · Used by {usedBy}
      </p>
    </>
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
  return (
    <FloatingCard x={x} y={y}>
      {cell ? (
        <>
          <p className="font-mono font-bold text-stitch-accent">
            {source.reference_code} → {target.reference_code}
          </p>
          <p className="mt-1 text-stitch-fg">
            {cell.link_types.map((t) => linkTypeMeta(t).label).join(' · ')}
            {cell.link_ids.length > 1 ? ` (${cell.link_ids.length} links)` : ''}
          </p>
          <dl className="mt-1.5 space-y-1">
            <div>
              <dt className="inline font-mono text-stitch-muted">{source.reference_code}: </dt>
              <dd className="inline text-stitch-fg">{source.title}</dd>
            </div>
            <div>
              <dt className="inline font-mono text-stitch-muted">{target.reference_code}: </dt>
              <dd className="inline text-stitch-fg">{target.title}</dd>
            </div>
          </dl>
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
        <RequirementSummary dsm={dsm} index={row} />
      )}
    </FloatingCard>
  );
}

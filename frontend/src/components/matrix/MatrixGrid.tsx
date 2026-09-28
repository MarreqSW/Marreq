import { useMemo, useRef, useState, type CSSProperties, type MouseEvent } from 'react';
import { Link } from 'react-router-dom';
import type { MatrixLink } from '@/api/types';
import {
  FloatingCard,
  gridLinesStyle,
  useScrollToTarget,
  type GridLayout,
  type GridScrollTarget,
} from '@/components/grid/gridShared';
import type { StatusSemanticGroup } from '@/lib/verificationStatusSemantic';

/** Cell edge in px (same density as the DSM). */
export const MATRIX_CELL = 26;
export const MATRIX_COL_HEADER_H = 124;

export interface MatrixRowItem {
  id: number;
  code: string;
  title: string;
  category: string;
  statusTitle: string;
  approvalState: string;
}

export interface MatrixColItem {
  id: number;
  code: string;
  name: string;
  statusTitle: string;
  method: string | null;
}

export interface MatrixCellItem {
  row: number;
  col: number;
  link: MatrixLink;
  statusTitle: string;
  group: StatusSemanticGroup;
  symbol: string;
  /** Status tag colour for the "other" symbol. */
  hex: string | null;
}

/** Selected problem, scrolled into view and framed. A null axis spans the whole row/column. */
export interface MatrixFocus extends GridScrollTarget {
  key: string;
  kind: 'suspect' | 'row' | 'column';
}

/** Theme-aware colours per status group (the shared glyph classes target the dark theme only). */
const GROUP_CLASS: Record<StatusSemanticGroup, string> = {
  pass: 'text-emerald-600 dark:text-emerald-400',
  verified: 'text-amber-600 dark:text-amber-300',
  pending: 'text-amber-500 dark:text-amber-200',
  draft: 'text-stitch-muted',
  fail: 'text-red-600 dark:text-red-300',
  other: 'text-stitch-muted',
};

const APPROVAL_DOT: Record<string, string> = {
  approved: 'bg-green-600',
  reviewed: 'bg-amber-500',
};

type SortState = { kind: 'requirement' } | { kind: 'verification'; verId: number };

type Props = {
  rows: MatrixRowItem[];
  cols: MatrixColItem[];
  cells: MatrixCellItem[];
  /** Row bands (inclusive indices), e.g. categories; empty when sorted by a column. */
  groups: { label: string; start: number; end: number }[];
  basePath: string;
  sort: SortState;
  dir: 'asc' | 'desc';
  onSortRequirement: () => void;
  onSortVerification: (verId: number) => void;
  rowHeaderWidth: number;
  onResizeStart: (e: MouseEvent) => void;
  focus?: MatrixFocus | null;
  onOpenRequirement: (requirementId: number) => void;
};

type Hover =
  | { kind: 'cell'; row: number; col: number; x: number; y: number }
  | { kind: 'row'; row: number; x: number; y: number }
  | { kind: 'col'; col: number; x: number; y: number };

export default function MatrixGrid({
  rows,
  cols,
  cells,
  groups,
  basePath,
  sort,
  dir,
  onSortRequirement,
  onSortVerification,
  rowHeaderWidth,
  onResizeStart,
  focus,
  onOpenRequirement,
}: Props) {
  const width = cols.length * MATRIX_CELL;
  const height = rows.length * MATRIX_CELL;
  const layout: GridLayout = {
    rowHeaderWidth,
    columnHeaderHeight: MATRIX_COL_HEADER_H,
    cellWidth: MATRIX_CELL,
    cellHeight: MATRIX_CELL,
  };
  const scrollRef = useRef<HTMLDivElement>(null);
  useScrollToTarget(scrollRef, focus, layout);

  const cellMap = useMemo(() => new Map(cells.map((c) => [`${c.row}:${c.col}`, c])), [cells]);
  const countsByRow = useMemo(() => countBy(cells, (c) => c.row), [cells]);
  const countsByCol = useMemo(() => countBy(cells, (c) => c.col), [cells]);
  const groupStart = useMemo(() => new Map(groups.map((g) => [g.start, g.label])), [groups]);
  const [hover, setHover] = useState<Hover | null>(null);

  const hoveredRow = hover?.kind === 'cell' || hover?.kind === 'row' ? hover.row : undefined;
  const hoveredCol = hover?.kind === 'cell' || hover?.kind === 'col' ? hover.col : undefined;

  const cellFromEvent = (e: MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const col = Math.floor((e.clientX - rect.left) / MATRIX_CELL);
    const row = Math.floor((e.clientY - rect.top) / MATRIX_CELL);
    return row >= 0 && col >= 0 && row < rows.length && col < cols.length ? { row, col } : null;
  };

  if (rows.length === 0 || cols.length === 0) {
    return (
      <div className="rounded-xl border border-stitch-border bg-stitch-surface p-8 text-sm text-stitch-muted">
        {cols.length === 0 ? 'No verifications match the filters.' : 'No requirements match the filters.'}
      </div>
    );
  }

  const sortMark = (active: boolean) => (active ? (dir === 'asc' ? '↑' : '↓') : '↕');
  const hoverCell = hover?.kind === 'cell' ? cellMap.get(`${hover.row}:${hover.col}`) : undefined;

  return (
    <div
      ref={scrollRef}
      className="relative overflow-auto max-h-[70vh] rounded-xl border border-stitch-border bg-stitch-surface shadow-stitch"
      data-testid="matrix-grid"
    >
      {/* w-max: the grid box must span all tracks so the sticky row headers stay visible. */}
      <div
        className="grid w-max"
        style={{
          gridTemplateColumns: `${rowHeaderWidth}px ${width}px`,
          gridTemplateRows: `${MATRIX_COL_HEADER_H}px ${height}px`,
        }}
      >
        {/* Corner: requirement sort + column resize */}
        <div className="sticky top-0 left-0 z-30 bg-stitch-surface border-b border-r border-stitch-border flex items-end px-3 pb-2 select-none">
          <button
            type="button"
            onClick={onSortRequirement}
            title="Sort rows by requirement reference. Click again to reverse."
            className="flex items-center gap-2 text-[10px] uppercase tracking-widest font-bold text-stitch-muted hover:text-stitch-accent"
          >
            Requirement ↓ verified by →
            <span className="font-mono text-stitch-accent" aria-hidden>
              {sortMark(sort.kind === 'requirement')}
            </span>
          </button>
          <button
            type="button"
            aria-label="Resize requirement column"
            title="Drag to resize"
            onMouseDown={onResizeStart}
            className="absolute right-0 top-0 bottom-0 w-2 cursor-col-resize bg-transparent hover:bg-stitch-accent/25"
          />
        </div>

        {/* Column headers (verifications) */}
        <div className="sticky top-0 z-20 bg-stitch-surface border-b border-stitch-border">
          {cols.map((v, j) => {
            const active = sort.kind === 'verification' && sort.verId === v.id;
            return (
              <div
                key={v.id}
                data-testid={`matrix-col-${j}`}
                className={`absolute bottom-0 flex flex-col items-center justify-end ${
                  hoveredCol === j ? 'bg-stitch-accent/10' : ''
                }`}
                style={{ left: j * MATRIX_CELL, width: MATRIX_CELL, height: MATRIX_COL_HEADER_H }}
                onMouseEnter={(e) => setHover({ kind: 'col', col: j, x: e.clientX, y: e.clientY })}
                onMouseMove={(e) => setHover({ kind: 'col', col: j, x: e.clientX, y: e.clientY })}
                onMouseLeave={() => setHover(null)}
              >
                <button
                  type="button"
                  onClick={() => onSortVerification(v.id)}
                  aria-label={`Sort rows by ${v.code}`}
                  className="flex flex-col items-center"
                >
                  <span className="[writing-mode:vertical-rl] rotate-180 font-mono text-[10.5px] font-semibold text-stitch-accent whitespace-nowrap overflow-hidden max-h-[84px]">
                    {v.code}
                  </span>
                  <span className={`font-mono text-[10px] ${active ? 'text-stitch-accent' : 'text-stitch-subtle'}`} aria-hidden>
                    {sortMark(active)}
                  </span>
                </button>
                <Link
                  to={`${basePath}/verifications/${v.id}`}
                  aria-label={`Open ${v.code}`}
                  className="pb-1 text-[10px] leading-none text-stitch-muted hover:text-stitch-accent"
                >
                  ↗
                </Link>
              </div>
            );
          })}
        </div>

        {/* Row headers (requirements) */}
        <div className="sticky left-0 z-10 bg-stitch-surface border-r border-stitch-border">
          {rows.map((r, i) => {
            const label = groupStart.get(i);
            return (
              <div
                key={r.id}
                data-testid={`matrix-row-${i}`}
                className={`absolute left-0 right-0 flex items-center gap-2 px-2 border-b border-stitch-border/60 ${
                  hoveredRow === i ? 'bg-stitch-accent/10' : ''
                } ${label && i > 0 ? 'border-t-2 border-t-stitch-accent/40' : ''}`}
                style={{ top: i * MATRIX_CELL, height: MATRIX_CELL }}
                onMouseEnter={(e) => setHover({ kind: 'row', row: i, x: e.clientX, y: e.clientY })}
                onMouseMove={(e) => setHover({ kind: 'row', row: i, x: e.clientX, y: e.clientY })}
                onMouseLeave={() => setHover(null)}
              >
                <span className="w-6 shrink-0 text-right font-mono text-[10px] text-stitch-subtle">{i + 1}</span>
                <span
                  className={`h-2 w-2 shrink-0 rounded-full ${APPROVAL_DOT[r.approvalState] ?? 'border border-stitch-muted'}`}
                  aria-label={r.approvalState}
                />
                <Link
                  to={`${basePath}/requirements/${r.id}`}
                  className="font-mono text-[11.5px] font-semibold text-stitch-accent hover:underline shrink-0"
                >
                  {r.code}
                </Link>
                <span className="truncate text-[11.5px] text-stitch-muted">{r.title}</span>
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
          style={gridLinesStyle(MATRIX_CELL, MATRIX_CELL)}
          onMouseMove={(e) => {
            const at = cellFromEvent(e);
            setHover(at ? { kind: 'cell', ...at, x: e.clientX, y: e.clientY } : null);
          }}
          onMouseLeave={() => setHover(null)}
          onClick={(e) => {
            const at = cellFromEvent(e);
            if (at && cellMap.has(`${at.row}:${at.col}`)) onOpenRequirement(rows[at.row]!.id);
          }}
          data-testid="matrix-body"
        >
          {hoveredRow != null ? (
            <div
              className="absolute left-0 right-0 bg-stitch-accent/8 pointer-events-none"
              style={{ top: hoveredRow * MATRIX_CELL, height: MATRIX_CELL }}
            />
          ) : null}
          {hoveredCol != null ? (
            <div
              className="absolute top-0 bottom-0 bg-stitch-accent/8 pointer-events-none"
              style={{ left: hoveredCol * MATRIX_CELL, width: MATRIX_CELL }}
            />
          ) : null}

          {groups.slice(1).map((g) => (
            <div
              key={`${g.label}-${g.start}`}
              className="absolute left-0 right-0 border-t-2 border-stitch-accent/40 pointer-events-none"
              style={{ top: g.start * MATRIX_CELL }}
            />
          ))}

          {focus ? <FocusFrame focus={focus} width={width} height={height} /> : null}

          {cells.map((c) => {
            const style: CSSProperties = {
              left: c.col * MATRIX_CELL,
              top: c.row * MATRIX_CELL,
              width: MATRIX_CELL,
              height: MATRIX_CELL,
              ...(c.group === 'other' && c.hex ? { color: c.hex } : {}),
            };
            return (
              <div
                key={`${c.row}:${c.col}`}
                data-testid={`matrix-cell-${c.row}-${c.col}`}
                className={`absolute flex items-center justify-center text-sm font-semibold pointer-events-none ${
                  GROUP_CLASS[c.group]
                } ${c.link.suspect ? 'shadow-[inset_0_0_0_2px_var(--color-stitch-danger)]' : ''}`}
                style={style}
              >
                {c.symbol}
                {c.link.suspect ? (
                  <span className="absolute top-0.5 right-0.5 h-1.5 w-1.5 rounded-full bg-stitch-danger" />
                ) : null}
              </div>
            );
          })}
        </div>
      </div>

      {hover?.kind === 'cell' && hoverCell ? (
        <FloatingCard x={hover.x} y={hover.y}>
          <CellSummary cell={hoverCell} row={rows[hoverCell.row]!} col={cols[hoverCell.col]!} />
        </FloatingCard>
      ) : null}
      {hover?.kind === 'row' ? (
        <FloatingCard x={hover.x} y={hover.y}>
          <RowSummary row={rows[hover.row]!} counts={countsByRow.get(hover.row)} />
        </FloatingCard>
      ) : null}
      {hover?.kind === 'col' ? (
        <FloatingCard x={hover.x} y={hover.y}>
          <ColSummary col={cols[hover.col]!} counts={countsByCol.get(hover.col)} />
        </FloatingCard>
      ) : null}
    </div>
  );
}

function countBy(cells: MatrixCellItem[], key: (c: MatrixCellItem) => number) {
  const out = new Map<number, { links: number; suspect: number }>();
  for (const c of cells) {
    const k = key(c);
    const cur = out.get(k) ?? { links: 0, suspect: 0 };
    cur.links += 1;
    if (c.link.suspect) cur.suspect += 1;
    out.set(k, cur);
  }
  return out;
}

function FocusFrame({ focus, width, height }: { focus: MatrixFocus; width: number; height: number }) {
  const left = focus.cols ? focus.cols[0] * MATRIX_CELL - 3 : 0;
  const top = focus.rows ? focus.rows[0] * MATRIX_CELL - 3 : 0;
  const w = focus.cols ? (focus.cols[1] - focus.cols[0] + 1) * MATRIX_CELL + 6 : width;
  const h = focus.rows ? (focus.rows[1] - focus.rows[0] + 1) * MATRIX_CELL + 6 : height;
  return (
    <div
      data-testid="matrix-focus"
      className={`absolute rounded-sm border-2 border-dashed pointer-events-none ${
        focus.kind === 'suspect' ? 'border-stitch-danger bg-stitch-danger/10' : 'border-amber-500 bg-amber-400/10'
      }`}
      style={{ left, top, width: w, height: h }}
    />
  );
}

function CellSummary({ cell, row, col }: { cell: MatrixCellItem; row: MatrixRowItem; col: MatrixColItem }) {
  const { link } = cell;
  return (
    <>
      <p className="font-mono font-bold text-stitch-accent">
        {row.code} → {col.code}
      </p>
      <p className="mt-1 text-stitch-fg">
        <span className={`mr-1 font-semibold ${GROUP_CLASS[cell.group]}`}>{cell.symbol}</span>
        {cell.statusTitle}
      </p>
      <dl className="mt-1.5 space-y-1">
        <div>
          <dt className="inline font-mono text-stitch-muted">{row.code}: </dt>
          <dd className="inline text-stitch-fg">{row.title}</dd>
        </div>
        <div>
          <dt className="inline font-mono text-stitch-muted">{col.code}: </dt>
          <dd className="inline text-stitch-fg">{col.name}</dd>
        </div>
      </dl>
      {link.suspect ? (
        <p className="mt-2 rounded-r border-l-2 border-stitch-danger bg-stitch-danger/10 px-2 py-1 text-stitch-fg">
          <b>Suspect.</b> {link.suspect_reason ?? 'The requirement changed after this link was verified'}
          {link.suspect_at ? ` (since ${link.suspect_at.slice(0, 10)})` : ''}. Review or clear it in the Suspect links
          panel.
        </p>
      ) : null}
    </>
  );
}

function RowSummary({ row, counts }: { row: MatrixRowItem; counts?: { links: number; suspect: number } }) {
  return (
    <>
      <p className="font-mono font-bold text-stitch-accent">{row.code}</p>
      <p className="mt-1 font-semibold text-stitch-fg">{row.title}</p>
      <p className="mt-1 text-stitch-muted">
        {row.category} · {row.statusTitle} · {row.approvalState}
      </p>
      <p className="mt-1 text-stitch-muted">
        Verified by {counts?.links ?? 0}
        {counts?.suspect ? ` · ${counts.suspect} suspect` : ''}
      </p>
    </>
  );
}

function ColSummary({ col, counts }: { col: MatrixColItem; counts?: { links: number; suspect: number } }) {
  return (
    <>
      <p className="font-mono font-bold text-stitch-accent">{col.code}</p>
      <p className="mt-1 font-semibold text-stitch-fg">{col.name}</p>
      <p className="mt-1 text-stitch-muted">
        {col.statusTitle}
        {col.method ? ` · ${col.method}` : ''}
      </p>
      <p className="mt-1 text-stitch-muted">
        Covers {counts?.links ?? 0} requirement{counts?.links === 1 ? '' : 's'}
        {counts?.suspect ? ` · ${counts.suspect} suspect` : ''}
      </p>
      <p className="mt-1 text-[10px] text-stitch-subtle">Click the code to sort rows by this column.</p>
    </>
  );
}

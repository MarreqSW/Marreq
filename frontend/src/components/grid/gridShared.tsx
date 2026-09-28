import { useEffect, type CSSProperties, type ReactNode, type RefObject } from 'react';

/**
 * Building blocks shared by the sparse matrix grids (DSM and requirement × verification):
 * hover card placement, grid lines, and scrolling a selected area into view.
 */

/** Header sizes and cell size of a sparse grid, in px. */
export interface GridLayout {
  rowHeaderWidth: number;
  columnHeaderHeight: number;
  cellWidth: number;
  cellHeight: number;
}

/**
 * Area to bring into view. A `null` axis keeps the current scroll position on that axis
 * (e.g. focusing a whole row only scrolls vertically).
 */
export interface GridScrollTarget {
  rows: [number, number] | null;
  cols: [number, number] | null;
}

/** Scroll offsets that centre `target` in the part of the viewport not covered by the sticky headers. */
export function centreScrollPosition(
  target: GridScrollTarget,
  viewport: { width: number; height: number },
  current: { left: number; top: number },
  layout: GridLayout,
): { left: number; top: number } {
  const bodyWidth = Math.max(0, viewport.width - layout.rowHeaderWidth);
  const bodyHeight = Math.max(0, viewport.height - layout.columnHeaderHeight);
  const centre = (span: [number, number], cell: number) => ((span[0] + span[1] + 1) / 2) * cell;
  return {
    left: target.cols
      ? Math.max(0, Math.round(centre(target.cols, layout.cellWidth) - bodyWidth / 2))
      : current.left,
    top: target.rows
      ? Math.max(0, Math.round(centre(target.rows, layout.cellHeight) - bodyHeight / 2))
      : current.top,
  };
}

/**
 * Scroll `ref` (the grid's scroll container) so a newly requested target is centred, and
 * bring the grid itself into the page viewport. Only a new `key` triggers scrolling.
 */
export function useScrollToTarget(
  ref: RefObject<HTMLElement | null>,
  target: (GridScrollTarget & { key: string }) | null | undefined,
  layout: GridLayout,
): void {
  useEffect(() => {
    const el = ref.current;
    if (!target || !el) return;
    el.scrollIntoView?.({ block: 'nearest', behavior: 'smooth' });
    const { left, top } = centreScrollPosition(
      target,
      { width: el.clientWidth, height: el.clientHeight },
      { left: el.scrollLeft, top: el.scrollTop },
      layout,
    );
    if (typeof el.scrollTo === 'function') el.scrollTo({ left, top, behavior: 'smooth' });
    else {
      el.scrollLeft = left;
      el.scrollTop = top;
    }
    // Only a new request (new key) re-centres the grid, not every re-render.
  }, [target?.key]);
}

/** Grid lines as a CSS background, so empty cells cost no DOM elements. */
export function gridLinesStyle(cellWidth: number, cellHeight: number): CSSProperties {
  return {
    backgroundImage:
      'linear-gradient(to right, var(--color-stitch-border) 1px, transparent 1px), linear-gradient(to bottom, var(--color-stitch-border) 1px, transparent 1px)',
    backgroundSize: `${cellWidth}px ${cellHeight}px`,
  };
}

/** Viewport-fixed card at the mouse, flipped left/up near the edges so it is never clipped. */
export function FloatingCard({ x, y, children }: { x: number; y: number; children: ReactNode }) {
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

import { useCallback, useEffect, useRef, useState, type CSSProperties } from 'react';

export const GRAPH_HEIGHT_DEFAULT_PX = 600;
export const GRAPH_HEIGHT_MIN_PX = 320;
export const GRAPH_HEIGHT_MAX_PX = 1200;
const VIEWPORT_MAX_RATIO = 0.85;

export function graphHeightStorageKey(projectId: number, viewKey: string) {
  return `marreq-trace-graph-h-${projectId}-${viewKey}`;
}

export function clampGraphHeight(n: number, maxPx = maxGraphHeightPx()): number {
  return Math.min(maxPx, Math.max(GRAPH_HEIGHT_MIN_PX, Math.round(n)));
}

export function maxGraphHeightPx(): number {
  if (typeof window === 'undefined') return GRAPH_HEIGHT_MAX_PX;
  return Math.min(GRAPH_HEIGHT_MAX_PX, Math.round(window.innerHeight * VIEWPORT_MAX_RATIO));
}

function readStoredHeight(storageKey: string): number | null {
  try {
    const raw = localStorage.getItem(storageKey);
    const n = raw ? parseInt(raw, 10) : NaN;
    return Number.isFinite(n) ? clampGraphHeight(n) : null;
  } catch {
    return null;
  }
}

export function useResizableGraphHeight(projectId: number, viewKey: string) {
  const storageKey = graphHeightStorageKey(projectId, viewKey);
  const [heightPx, setHeightPx] = useState(GRAPH_HEIGHT_DEFAULT_PX);
  const heightRef = useRef(GRAPH_HEIGHT_DEFAULT_PX);
  heightRef.current = heightPx;
  const resizeDragRef = useRef<{ startY: number; startH: number } | null>(null);

  useEffect(() => {
    if (!Number.isFinite(projectId)) return;
    const stored = readStoredHeight(storageKey);
    const next = stored ?? GRAPH_HEIGHT_DEFAULT_PX;
    heightRef.current = next;
    setHeightPx(next);
  }, [projectId, storageKey]);

  useEffect(() => {
    function onMove(e: MouseEvent) {
      const drag = resizeDragRef.current;
      if (!drag) return;
      const next = clampGraphHeight(drag.startH + (e.clientY - drag.startY));
      heightRef.current = next;
      setHeightPx(next);
    }
    function onUp() {
      if (!resizeDragRef.current) return;
      resizeDragRef.current = null;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      if (Number.isFinite(projectId)) {
        try {
          localStorage.setItem(storageKey, String(heightRef.current));
        } catch {
          /* ignore quota */
        }
      }
    }
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
    return () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
  }, [projectId, storageKey]);

  const onResizeStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    resizeDragRef.current = { startY: e.clientY, startH: heightRef.current };
    document.body.style.cursor = 'ns-resize';
    document.body.style.userSelect = 'none';
  }, []);

  const containerStyle: CSSProperties = {
    height: heightPx,
    minHeight: heightPx,
    maxHeight: heightPx,
    boxSizing: 'border-box',
  };

  return { heightPx, containerStyle, onResizeStart };
}

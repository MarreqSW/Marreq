import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  clampGraphHeight,
  graphHeightStorageKey,
  GRAPH_HEIGHT_DEFAULT_PX,
  GRAPH_HEIGHT_MIN_PX,
  GRAPH_HEIGHT_MAX_PX,
  maxGraphHeightPx,
  useResizableGraphHeight,
} from '../useResizableGraphHeight';

describe('graphHeightStorageKey', () => {
  it('includes project id and view key', () => {
    expect(graphHeightStorageKey(42, 'coverage')).toBe('marreq-trace-graph-h-42-coverage');
    expect(graphHeightStorageKey(1, 'hierarchy')).toBe('marreq-trace-graph-h-1-hierarchy');
  });
});

describe('clampGraphHeight', () => {
  it('clamps below minimum', () => {
    expect(clampGraphHeight(100, 1200)).toBe(GRAPH_HEIGHT_MIN_PX);
  });

  it('clamps above maximum', () => {
    expect(clampGraphHeight(2000, 1200)).toBe(1200);
  });

  it('rounds and preserves in-range values', () => {
    expect(clampGraphHeight(599.4, 1200)).toBe(599);
    expect(clampGraphHeight(GRAPH_HEIGHT_DEFAULT_PX, 1200)).toBe(GRAPH_HEIGHT_DEFAULT_PX);
  });

  it('respects explicit maxPx cap', () => {
    expect(clampGraphHeight(GRAPH_HEIGHT_MAX_PX + 500, GRAPH_HEIGHT_MAX_PX)).toBe(
      GRAPH_HEIGHT_MAX_PX,
    );
  });
});

describe('maxGraphHeightPx', () => {
  it('caps by viewport height ratio', () => {
    vi.stubGlobal('innerHeight', 800);
    expect(maxGraphHeightPx()).toBe(Math.min(GRAPH_HEIGHT_MAX_PX, Math.round(800 * 0.85)));
    vi.unstubAllGlobals();
  });
});

describe('useResizableGraphHeight', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  afterEach(() => {
    localStorage.clear();
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  });

  it('loads a stored height from localStorage', () => {
    localStorage.setItem(graphHeightStorageKey(7, 'coverage'), '480');
    const { result } = renderHook(() => useResizableGraphHeight(7, 'coverage'));
    expect(result.current.heightPx).toBe(480);
    expect(result.current.containerStyle.height).toBe(480);
  });

  it('defaults when nothing is stored', () => {
    const { result } = renderHook(() => useResizableGraphHeight(3, 'hierarchy'));
    expect(result.current.heightPx).toBe(GRAPH_HEIGHT_DEFAULT_PX);
  });

  it('persists height on drag end', () => {
    vi.stubGlobal('innerHeight', 2000);
    const { result } = renderHook(() => useResizableGraphHeight(9, 'coverage'));

    act(() => {
      result.current.onResizeStart({
        preventDefault() {},
        clientY: 100,
      } as React.MouseEvent);
    });

    act(() => {
      window.dispatchEvent(new MouseEvent('mousemove', { clientY: 180 }));
    });
    expect(result.current.heightPx).toBe(GRAPH_HEIGHT_DEFAULT_PX + 80);

    act(() => {
      window.dispatchEvent(new MouseEvent('mouseup'));
    });

    expect(localStorage.getItem(graphHeightStorageKey(9, 'coverage'))).toBe(
      String(GRAPH_HEIGHT_DEFAULT_PX + 80),
    );
    vi.unstubAllGlobals();
  });
});

import { describe, expect, it } from 'vitest';
import {
  clampGraphHeight,
  graphHeightStorageKey,
  GRAPH_HEIGHT_DEFAULT_PX,
  GRAPH_HEIGHT_MIN_PX,
  GRAPH_HEIGHT_MAX_PX,
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

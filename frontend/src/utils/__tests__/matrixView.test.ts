import { describe, expect, it } from 'vitest';
import { DEFAULT_MATRIX_PARAMS, groupRuns, nextSort, readMatrixParams, toggleIn, writeMatrixParams } from '../matrixView';
import { centreScrollPosition } from '@/components/grid/gridShared';

describe('matrix URL params', () => {
  it('reads defaults from an empty URL', () => {
    expect(readMatrixParams(new URLSearchParams())).toEqual(DEFAULT_MATRIX_PARAMS);
  });

  it('round-trips non-default values and keeps other params', () => {
    const params = {
      suspectOnly: true,
      statusGroups: ['fail' as const, 'pending' as const],
      reqStatusIds: [2, 3],
      verStatusIds: [7],
      sort: { kind: 'verification' as const, verId: 42 },
      dir: 'asc' as const,
    };
    const url = writeMatrixParams(new URLSearchParams('q=1'), params);
    expect(url.get('q')).toBe('1');
    expect(readMatrixParams(url)).toEqual(params);
  });

  it('drops defaults, including the default direction of each sort', () => {
    expect(writeMatrixParams(new URLSearchParams(), DEFAULT_MATRIX_PARAMS).toString()).toBe('');
    const byColumn = writeMatrixParams(new URLSearchParams(), {
      ...DEFAULT_MATRIX_PARAMS,
      sort: { kind: 'verification', verId: 5 },
      dir: 'asc',
    });
    expect(byColumn.toString()).toBe('mx_sort=ver%3A5');
  });

  it('ignores invalid values', () => {
    const p = readMatrixParams(new URLSearchParams('mx_groups=fail,nope&mx_rs=a,2,-1&mx_sort=ver:x&mx_dir=up'));
    expect(p.statusGroups).toEqual(['fail']);
    expect(p.reqStatusIds).toEqual([2]);
    expect(p.sort).toEqual({ kind: 'requirement' });
    expect(p.dir).toBe('asc');
  });
});

describe('nextSort', () => {
  it('flips the same column and starts another with its default direction', () => {
    expect(nextSort(DEFAULT_MATRIX_PARAMS, { kind: 'requirement' })).toEqual({ sort: { kind: 'requirement' }, dir: 'desc' });
    expect(nextSort(DEFAULT_MATRIX_PARAMS, { kind: 'verification', verId: 3 })).toEqual({
      sort: { kind: 'verification', verId: 3 },
      dir: 'asc',
    });
    const byCol = { ...DEFAULT_MATRIX_PARAMS, sort: { kind: 'verification' as const, verId: 3 }, dir: 'asc' as const };
    expect(nextSort(byCol, { kind: 'verification', verId: 3 }).dir).toBe('desc');
  });
});

describe('helpers', () => {
  it('toggles list membership and groups runs', () => {
    expect(toggleIn([1, 2], 2)).toEqual([1]);
    expect(toggleIn([1], 2)).toEqual([1, 2]);
    expect(groupRuns(['A', 'A', 'B', 'A'])).toEqual([
      { label: 'A', start: 0, end: 1 },
      { label: 'B', start: 2, end: 2 },
      { label: 'A', start: 3, end: 3 },
    ]);
  });

  it('centres a target and keeps the current offset on a null axis', () => {
    const layout = { rowHeaderWidth: 300, columnHeaderHeight: 124, cellWidth: 26, cellHeight: 26 };
    const pos = centreScrollPosition({ rows: [50, 50], cols: null }, { width: 1000, height: 700 }, { left: 77, top: 0 }, layout);
    expect(pos).toEqual({ left: 77, top: Math.round(50.5 * 26 - (700 - 124) / 2) });
  });
});

import { describe, expect, it } from 'vitest';
import {
  DEFAULT_DSM_LINK_TYPES,
  DEFAULT_DSM_PARAMS,
  cellAtPoint,
  dsmQueryString,
  loopNumberByRequirement,
  readDsmParams,
  toggleLinkType,
  writeDsmParams,
} from '../dsm';

describe('DSM URL params', () => {
  it('reads defaults from an empty URL', () => {
    expect(readDsmParams(new URLSearchParams())).toEqual(DEFAULT_DSM_PARAMS);
  });

  it('round-trips non-default values and keeps other params', () => {
    const params = {
      linkTypes: ['REFINES', 'RELATES_TO'],
      order: 'partition' as const,
      categoryId: 3,
      rootId: 42,
    };
    const url = writeDsmParams(new URLSearchParams('view=dsm'), params);
    expect(url.get('view')).toBe('dsm');
    expect(readDsmParams(url)).toEqual(params);
  });

  it('drops values equal to the defaults', () => {
    const url = writeDsmParams(new URLSearchParams('dsm_order=partition&dsm_types=REFINES'), {
      ...DEFAULT_DSM_PARAMS,
      linkTypes: [...DEFAULT_DSM_LINK_TYPES].reverse(),
    });
    expect(url.toString()).toBe('');
  });

  it('ignores unknown link types and invalid ids', () => {
    const p = readDsmParams(new URLSearchParams('dsm_types=REFINES,BOGUS&dsm_category=x&dsm_root=-1'));
    expect(p.linkTypes).toEqual(['REFINES']);
    expect(p.categoryId).toBeNull();
    expect(p.rootId).toBeNull();
  });

  it('keeps an explicit empty selection', () => {
    const none = { ...DEFAULT_DSM_PARAMS, linkTypes: [] };
    const url = writeDsmParams(new URLSearchParams(), none);
    expect(url.get('dsm_types')).toBe('');
    expect(readDsmParams(url).linkTypes).toEqual([]);
    expect(dsmQueryString(none)).toBe('?link_types=');
  });
});

describe('toggleLinkType', () => {
  it('adds in chip order and removes', () => {
    const on = toggleLinkType(DEFAULT_DSM_PARAMS, 'RELATES_TO');
    expect(on.linkTypes).toEqual([...DEFAULT_DSM_LINK_TYPES, 'RELATES_TO']);
    const off = toggleLinkType(DEFAULT_DSM_PARAMS, 'REFINES');
    expect(off.linkTypes).toEqual(['DERIVES_FROM', 'DEPENDS_ON', 'SATISFIES']);
  });
});

describe('dsmQueryString', () => {
  it('is empty for defaults and encodes the rest', () => {
    expect(dsmQueryString(DEFAULT_DSM_PARAMS)).toBe('');
    expect(
      dsmQueryString({ linkTypes: ['DEPENDS_ON'], order: 'partition', categoryId: 2, rootId: 7 }),
    ).toBe('?link_types=DEPENDS_ON&order=partition&category_id=2&root_id=7');
  });
});

describe('geometry and loops', () => {
  it('maps a point to a cell', () => {
    expect(cellAtPoint(30, 5, 26, 3)).toEqual({ row: 0, col: 1 });
    expect(cellAtPoint(80, 5, 26, 3)).toBeNull();
    expect(cellAtPoint(-1, 5, 26, 3)).toBeNull();
  });

  it('numbers loops by requirement', () => {
    const map = loopNumberByRequirement([
      { requirement_ids: [1, 2], path: [1, 2] },
      { requirement_ids: [5, 6, 7], path: [5, 6, 7] },
    ]);
    expect(map.get(2)).toBe(1);
    expect(map.get(7)).toBe(2);
    expect(map.has(3)).toBe(false);
  });
});

import type { DsmCell, DsmLoop } from '@/api/types';

/** Link types the DSM can draw, in chip order. */
export const DSM_LINK_TYPES = [
  'DERIVES_FROM',
  'REFINES',
  'DEPENDS_ON',
  'SATISFIES',
  'RELATES_TO',
] as const;

/** Server default when no `link_types` is sent: everything except RELATES_TO. */
export const DEFAULT_DSM_LINK_TYPES: string[] = DSM_LINK_TYPES.filter(
  (t) => t !== 'RELATES_TO',
);

export type DsmOrder = 'hierarchy' | 'partition';

export interface DsmParams {
  /** `null` = server default. An empty array means no link type selected. */
  linkTypes: string[] | null;
  order: DsmOrder;
  categoryId: number | null;
  rootId: number | null;
}

export const DEFAULT_DSM_PARAMS: DsmParams = {
  linkTypes: null,
  order: 'hierarchy',
  categoryId: null,
  rootId: null,
};

/** Letter, label and colour classes per link type (text / chip background). */
export const LINK_TYPE_META: Record<string, { letter: string; label: string; text: string; chip: string }> = {
  DERIVES_FROM: {
    letter: 'D',
    label: 'Derives from',
    text: 'text-stitch-accent',
    chip: 'bg-stitch-accent',
  },
  REFINES: {
    letter: 'R',
    label: 'Refines',
    text: 'text-teal-700 dark:text-teal-300',
    chip: 'bg-teal-700 dark:bg-teal-300',
  },
  DEPENDS_ON: {
    letter: 'P',
    label: 'Depends on',
    text: 'text-violet-700 dark:text-violet-300',
    chip: 'bg-violet-700 dark:bg-violet-300',
  },
  SATISFIES: {
    letter: 'S',
    label: 'Satisfies',
    text: 'text-amber-700 dark:text-amber-300',
    chip: 'bg-amber-700 dark:bg-amber-300',
  },
  RELATES_TO: {
    letter: '~',
    label: 'Relates to',
    text: 'text-stitch-muted',
    chip: 'bg-stitch-muted',
  },
};

export function linkTypeMeta(type: string) {
  return (
    LINK_TYPE_META[type] ?? { letter: '?', label: type, text: 'text-stitch-muted', chip: 'bg-stitch-muted' }
  );
}

const PARAM = {
  types: 'dsm_types',
  order: 'dsm_order',
  category: 'dsm_category',
  root: 'dsm_root',
} as const;

function positiveInt(value: string | null): number | null {
  if (value == null || value === '') return null;
  const n = Number(value);
  return Number.isInteger(n) && n > 0 ? n : null;
}

function sameSet(a: string[], b: string[]): boolean {
  return a.length === b.length && a.every((x) => b.includes(x));
}

/** Read the DSM filters from the page URL (unknown values fall back to defaults). */
export function readDsmParams(search: URLSearchParams): DsmParams {
  const rawTypes = search.get(PARAM.types);
  const linkTypes =
    rawTypes == null
      ? null
      : rawTypes
          .split(',')
          .map((t) => t.trim())
          .filter((t) => (DSM_LINK_TYPES as readonly string[]).includes(t));
  return {
    linkTypes,
    order: search.get(PARAM.order) === 'partition' ? 'partition' : 'hierarchy',
    categoryId: positiveInt(search.get(PARAM.category)),
    rootId: positiveInt(search.get(PARAM.root)),
  };
}

/** Write the DSM filters into a copy of `search`, dropping values equal to the defaults. */
export function writeDsmParams(search: URLSearchParams, params: DsmParams): URLSearchParams {
  const next = new URLSearchParams(search);
  const types = params.linkTypes;
  if (types == null || sameSet(types, DEFAULT_DSM_LINK_TYPES)) {
    next.delete(PARAM.types);
  } else {
    next.set(PARAM.types, types.join(','));
  }
  if (params.order === 'hierarchy') next.delete(PARAM.order);
  else next.set(PARAM.order, params.order);
  if (params.categoryId == null) next.delete(PARAM.category);
  else next.set(PARAM.category, String(params.categoryId));
  if (params.rootId == null) next.delete(PARAM.root);
  else next.set(PARAM.root, String(params.rootId));
  return next;
}

/** Link types currently selected (defaults resolved). */
export function selectedLinkTypes(params: DsmParams): string[] {
  return params.linkTypes ?? DEFAULT_DSM_LINK_TYPES;
}

/** Toggle one link type, keeping chip order. */
export function toggleLinkType(params: DsmParams, type: string): DsmParams {
  const current = selectedLinkTypes(params);
  const next = current.includes(type)
    ? current.filter((t) => t !== type)
    : DSM_LINK_TYPES.filter((t) => t === type || current.includes(t));
  return { ...params, linkTypes: next };
}

/** Query string for `GET /api/projects/:id/dsm` and the Excel export (leading `?` or empty). */
export function dsmQueryString(params: DsmParams): string {
  const q = new URLSearchParams();
  if (params.linkTypes != null && !sameSet(params.linkTypes, DEFAULT_DSM_LINK_TYPES)) {
    q.set('link_types', params.linkTypes.join(','));
  }
  if (params.order !== 'hierarchy') q.set('order', params.order);
  if (params.categoryId != null) q.set('category_id', String(params.categoryId));
  if (params.rootId != null) q.set('root_id', String(params.rootId));
  const s = q.toString();
  return s ? `?${s}` : '';
}

export function cellKey(row: number, col: number): string {
  return `${row}:${col}`;
}

export function buildCellMap(cells: DsmCell[]): Map<string, DsmCell> {
  return new Map(cells.map((c) => [cellKey(c.row, c.col), c]));
}

/** Loop number (1-based, as listed) for each requirement id that is part of a loop. */
export function loopNumberByRequirement(loops: DsmLoop[]): Map<number, number> {
  const out = new Map<number, number>();
  loops.forEach((l, i) => l.requirement_ids.forEach((id) => out.set(id, i + 1)));
  return out;
}

/** Row/column under a point inside the matrix body, or null outside it. */
export function cellAtPoint(
  x: number,
  y: number,
  cellSize: number,
  count: number,
): { row: number; col: number } | null {
  const col = Math.floor(x / cellSize);
  const row = Math.floor(y / cellSize);
  if (x < 0 || y < 0 || row >= count || col >= count) return null;
  return { row, col };
}

import type { StatusSemanticGroup } from '@/lib/verificationStatusSemantic';

/** Filters and sort of the requirement × verification matrix, kept in the URL (`mx_*` params). */
export interface MatrixParams {
  suspectOnly: boolean;
  statusGroups: StatusSemanticGroup[];
  reqStatusIds: number[];
  verStatusIds: number[];
  sort: { kind: 'requirement' } | { kind: 'verification'; verId: number };
  dir: 'asc' | 'desc';
}

export const DEFAULT_MATRIX_PARAMS: MatrixParams = {
  suspectOnly: false,
  statusGroups: [],
  reqStatusIds: [],
  verStatusIds: [],
  sort: { kind: 'requirement' },
  dir: 'asc',
};

const GROUPS: StatusSemanticGroup[] = ['pass', 'verified', 'pending', 'draft', 'fail', 'other'];

const PARAM = {
  suspect: 'mx_suspect',
  groups: 'mx_groups',
  reqStatus: 'mx_rs',
  verStatus: 'mx_vs',
  sort: 'mx_sort',
  dir: 'mx_dir',
} as const;

/**
 * Every sort starts ascending: by reference code, or for a verification column with the
 * linked (and suspect) rows first. A second click on the same header reverses it.
 */
function defaultDir(_sort: MatrixParams['sort']): 'asc' | 'desc' {
  return 'asc';
}

function ids(value: string | null): number[] {
  if (!value) return [];
  return [...new Set(value.split(',').map(Number).filter((n) => Number.isInteger(n) && n > 0))];
}

export function readMatrixParams(search: URLSearchParams): MatrixParams {
  const rawSort = search.get(PARAM.sort);
  const verId = rawSort?.startsWith('ver:') ? Number(rawSort.slice(4)) : NaN;
  const sort: MatrixParams['sort'] =
    Number.isInteger(verId) && verId > 0 ? { kind: 'verification', verId } : { kind: 'requirement' };
  const rawDir = search.get(PARAM.dir);
  return {
    suspectOnly: search.get(PARAM.suspect) === '1',
    statusGroups: (search.get(PARAM.groups) ?? '')
      .split(',')
      .filter((g): g is StatusSemanticGroup => (GROUPS as string[]).includes(g)),
    reqStatusIds: ids(search.get(PARAM.reqStatus)),
    verStatusIds: ids(search.get(PARAM.verStatus)),
    sort,
    dir: rawDir === 'asc' || rawDir === 'desc' ? rawDir : defaultDir(sort),
  };
}

/** Write the params into a copy of `search`, dropping values equal to the defaults. */
export function writeMatrixParams(search: URLSearchParams, params: MatrixParams): URLSearchParams {
  const next = new URLSearchParams(search);
  const set = (key: string, value: string | null) => (value ? next.set(key, value) : next.delete(key));
  set(PARAM.suspect, params.suspectOnly ? '1' : null);
  set(PARAM.groups, params.statusGroups.length ? params.statusGroups.join(',') : null);
  set(PARAM.reqStatus, params.reqStatusIds.length ? params.reqStatusIds.join(',') : null);
  set(PARAM.verStatus, params.verStatusIds.length ? params.verStatusIds.join(',') : null);
  set(PARAM.sort, params.sort.kind === 'verification' ? `ver:${params.sort.verId}` : null);
  set(PARAM.dir, params.dir === defaultDir(params.sort) ? null : params.dir);
  return next;
}

export function toggleIn<T>(list: T[], value: T): T[] {
  return list.includes(value) ? list.filter((x) => x !== value) : [...list, value];
}

/**
 * Next sort after clicking a header: the same column flips direction, another column
 * starts with its default direction.
 */
export function nextSort(params: MatrixParams, clicked: MatrixParams['sort']): Pick<MatrixParams, 'sort' | 'dir'> {
  const same =
    clicked.kind === params.sort.kind &&
    (clicked.kind === 'requirement' || (params.sort.kind === 'verification' && params.sort.verId === clicked.verId));
  if (same) return { sort: params.sort, dir: params.dir === 'asc' ? 'desc' : 'asc' };
  return { sort: clicked, dir: defaultDir(clicked) };
}

/** Contiguous runs of equal labels, as inclusive index ranges (row bands). */
export function groupRuns(labels: string[]): { label: string; start: number; end: number }[] {
  const runs: { label: string; start: number; end: number }[] = [];
  labels.forEach((label, i) => {
    const last = runs[runs.length - 1];
    if (last && last.label === label) last.end = i;
    else runs.push({ label, start: i, end: i });
  });
  return runs;
}

/** Whether any matrix filter is set (sort does not count). */
export function hasMatrixFilters(params: MatrixParams): boolean {
  return (
    params.suspectOnly ||
    params.statusGroups.length > 0 ||
    params.reqStatusIds.length > 0 ||
    params.verStatusIds.length > 0
  );
}

/** The same parameters with every filter cleared; the sort is kept. */
export function clearMatrixFilters(params: MatrixParams): MatrixParams {
  return { ...params, suspectOnly: false, statusGroups: [], reqStatusIds: [], verStatusIds: [] };
}

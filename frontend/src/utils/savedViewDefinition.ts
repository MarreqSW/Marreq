import type {
  RequirementsColumnId,
  RequirementsSortColumn,
  SavedViewDefinition,
} from '@/api/types';

export const DEFAULT_REQUIREMENT_COLUMNS: RequirementsColumnId[] = [
  'key',
  'title',
  'category',
  'parents',
  'status',
  'approval',
  'verification',
  'modified',
  'author',
  'actions',
];

export type RequirementsQueryState = {
  statusFilter: 'all' | number;
  categoryFilter: 'all' | number;
  approvalFilter: 'all' | string;
  q: string;
  sortColumn: RequirementsSortColumn;
  sortDir: 'asc' | 'desc';
  columns: RequirementsColumnId[];
  viewMode: 'table' | 'list';
  pageSize: number;
};

export function defaultQueryState(): RequirementsQueryState {
  return {
    statusFilter: 'all',
    categoryFilter: 'all',
    approvalFilter: 'all',
    q: '',
    sortColumn: null,
    sortDir: 'asc',
    columns: [...DEFAULT_REQUIREMENT_COLUMNS],
    viewMode: 'table',
    pageSize: 25,
  };
}

export function buildSavedViewDefinition(state: RequirementsQueryState): SavedViewDefinition {
  return {
    version: 1,
    entity: 'requirements',
    filters: {
      status_id: state.statusFilter === 'all' ? null : state.statusFilter,
      category_id: state.categoryFilter === 'all' ? null : state.categoryFilter,
      approval_state: state.approvalFilter === 'all' ? null : state.approvalFilter,
      q: state.q,
    },
    sort: {
      column: state.sortColumn,
      dir: state.sortDir,
    },
    columns: state.columns,
    ui: {
      view_mode: state.viewMode,
      page_size: state.pageSize,
    },
  };
}

function asColumns(raw: unknown): RequirementsColumnId[] {
  if (!Array.isArray(raw) || raw.length === 0) {
    return [...DEFAULT_REQUIREMENT_COLUMNS];
  }
  const allowed = new Set<string>(DEFAULT_REQUIREMENT_COLUMNS);
  const cols = raw.filter((c): c is RequirementsColumnId => typeof c === 'string' && allowed.has(c));
  return cols.length > 0 ? cols : [...DEFAULT_REQUIREMENT_COLUMNS];
}

export function parseSavedViewDefinition(def: unknown): RequirementsQueryState {
  const base = defaultQueryState();
  if (!def || typeof def !== 'object') return base;
  const d = def as Partial<SavedViewDefinition>;
  const filters = d.filters ?? {};
  const sort = d.sort ?? { column: null, dir: 'asc' };
  const ui = d.ui ?? {};

  const statusId = filters.status_id;
  const categoryId = filters.category_id;
  const approval = filters.approval_state;
  const sortCol = sort.column;
  const allowedSort = new Set([
    'key',
    'title',
    'category',
    'status',
    'approval',
    'modified',
    'author',
  ]);

  return {
    statusFilter: typeof statusId === 'number' ? statusId : 'all',
    categoryFilter: typeof categoryId === 'number' ? categoryId : 'all',
    approvalFilter: typeof approval === 'string' && approval ? approval : 'all',
    q: typeof filters.q === 'string' ? filters.q : '',
    sortColumn:
      typeof sortCol === 'string' && allowedSort.has(sortCol)
        ? (sortCol as Exclude<RequirementsSortColumn, null>)
        : null,
    sortDir: sort.dir === 'desc' ? 'desc' : 'asc',
    columns: asColumns(d.columns),
    viewMode: ui.view_mode === 'list' ? 'list' : 'table',
    pageSize:
      typeof ui.page_size === 'number' && ui.page_size > 0
        ? Math.min(500, ui.page_size)
        : 25,
  };
}

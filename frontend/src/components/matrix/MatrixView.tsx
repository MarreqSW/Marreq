import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useNavigate, useOutletContext, useSearchParams } from 'react-router-dom';
import {
  clearTraceabilitySuspect,
  downloadMatrixXlsx,
  listCategories,
  listMatrix,
  listRequirementVersionsByProject,
  listRequirementStatuses,
  listRequirements,
  listVerificationMethodsByProject,
  listVerificationStatuses,
  listVerifications,
} from '@/api/client';
import RequirementVersionDiffDialog from '@/components/RequirementVersionDiffDialog';
import MatrixGrid, {
  type MatrixCellItem,
  type MatrixColItem,
  type MatrixFocus,
  type MatrixRowItem,
} from '@/components/matrix/MatrixGrid';
import AddStatusMenu from '@/components/matrix/AddStatusMenu';
import MatrixSidePanel, { type GapItem, type SuspectItem } from '@/components/matrix/MatrixSidePanel';
import StatusFilterChip from '@/components/matrix/StatusFilterChip';
import SuspectSwitch from '@/components/matrix/SuspectSwitch';
import { useDashboard } from '@/context/DashboardContext';
import { useProjectArchived } from '@/hooks/useProjectArchived';
import type {
  Category,
  MatrixLink,
  Requirement,
  RequirementStatus,
  RequirementVersion,
  Verification,
  VerificationMethod,
  VerificationStatus,
} from '@/api/types';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import {
  GROUP_TEXT_CLASS,
  STATUS_GROUP_OPTIONS,
  statusGlyph,
  statusSemanticGroup,
  type StatusSemanticGroup,
} from '@/lib/verificationStatusSemantic';
import {
  clearMatrixFilters,
  groupRuns,
  hasMatrixFilters,
  nextSort,
  readMatrixParams,
  toggleIn,
  writeMatrixParams,
  type MatrixParams,
} from '@/utils/matrixView';

const REQ_COL_DEFAULT_PX = 300;
const REQ_COL_MIN_PX = 160;
const REQ_COL_MAX_PX = 560;
const UNCATEGORISED = 'Uncategorised';

function reqColStorageKey(projectId: number) {
  return `reqman-matrix-req-col-w-${projectId}`;
}

function clampReqColW(n: number) {
  return Math.min(REQ_COL_MAX_PX, Math.max(REQ_COL_MIN_PX, Math.round(n)));
}

function compareRefCode(a: Requirement, b: Requirement): number {
  return (a.reference_code || '').localeCompare(b.reference_code || '', undefined, { numeric: true });
}

/** Sort rows by one verification column: linked before unlinked, suspect before non-suspect among linked, then ref. */
function compareByVerificationColumn(
  a: Requirement,
  b: Requirement,
  verId: number,
  linkByPair: Map<string, MatrixLink>,
): number {
  const la = linkByPair.get(`${a.id}-${verId}`);
  const lb = linkByPair.get(`${b.id}-${verId}`);
  const linkedA = la ? 0 : 1;
  const linkedB = lb ? 0 : 1;
  if (linkedA !== linkedB) return linkedA - linkedB;
  if (!la && !lb) return compareRefCode(a, b);
  if (la!.suspect !== lb!.suspect) return la!.suspect ? -1 : 1;
  return compareRefCode(a, b);
}

/** Status-group buttons: neutral ghost style; the selected state is restrained (issue #361). */
const groupOn = 'bg-stitch-higher text-stitch-fg font-semibold shadow-[inset_0_-2px_0_var(--color-stitch-accent)]';
const groupOff = 'text-stitch-muted hover:bg-stitch-higher hover:text-stitch-fg';
const label = 'text-[10px] uppercase tracking-widest text-stitch-muted font-bold mr-1 shrink-0';
const headerButton =
  'text-xs font-bold uppercase tracking-wider text-stitch-accent border border-stitch-border rounded-md px-3 py-2 hover:bg-stitch-higher disabled:opacity-50';

type Selection =
  | ({ kind: 'suspect'; req: number; ver: number } | { kind: 'row'; req: number } | { kind: 'column'; ver: number }) & {
      seq: number;
    };

/**
 * Requirement × verification matrix, shown as the Matrix tab of Traceability.
 * Filters live in `mx_*` URL params, so they survive switching tabs.
 */
export default function MatrixView() {
  const { globalSearch, basePath, projectId } = useOutletContext<ProjectOutletContext>();
  const pid = projectId;
  const archived = useProjectArchived(pid);
  const { csrfToken } = useDashboard();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const params = useMemo(() => readMatrixParams(searchParams), [searchParams]);
  const update = (next: Partial<MatrixParams>) =>
    setSearchParams(writeMatrixParams(searchParams, { ...params, ...next }), { replace: true });

  const [matrix, setMatrix] = useState<MatrixLink[]>([]);
  const [reqs, setReqs] = useState<Requirement[]>([]);
  const [vers, setVers] = useState<Verification[]>([]);
  const [statuses, setStatuses] = useState<VerificationStatus[]>([]);
  const [reqStatuses, setReqStatuses] = useState<RequirementStatus[]>([]);
  const [methods, setMethods] = useState<VerificationMethod[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  const [diffRequirementId, setDiffRequirementId] = useState<number | null>(null);
  const [diffVersions, setDiffVersions] = useState<RequirementVersion[]>([]);
  const [diffPair, setDiffPair] = useState<{ oldVersionId?: number; newVersionId?: number } | null>(null);
  const [selection, setSelection] = useState<Selection | null>(null);

  // Resizable requirement column, remembered per project.
  const [reqColWidthPx, setReqColWidthPx] = useState(REQ_COL_DEFAULT_PX);
  const resizeDragRef = useRef<{ startX: number; startW: number } | null>(null);
  const reqColWidthRef = useRef(REQ_COL_DEFAULT_PX);
  reqColWidthRef.current = reqColWidthPx;

  useEffect(() => {
    if (!Number.isFinite(pid)) return;
    try {
      const n = parseInt(localStorage.getItem(reqColStorageKey(pid)) ?? '', 10);
      setReqColWidthPx(Number.isFinite(n) ? clampReqColW(n) : REQ_COL_DEFAULT_PX);
    } catch {
      setReqColWidthPx(REQ_COL_DEFAULT_PX);
    }
  }, [pid]);

  useEffect(() => {
    function onMove(e: MouseEvent) {
      const drag = resizeDragRef.current;
      if (!drag) return;
      const next = clampReqColW(drag.startW + (e.clientX - drag.startX));
      reqColWidthRef.current = next;
      setReqColWidthPx(next);
    }
    function onUp() {
      if (!resizeDragRef.current) return;
      resizeDragRef.current = null;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      try {
        localStorage.setItem(reqColStorageKey(pid), String(reqColWidthRef.current));
      } catch {
        /* ignore quota */
      }
    }
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
    return () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
  }, [pid]);

  function onReqColResizeStart(e: React.MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    resizeDragRef.current = { startX: e.clientX, startW: reqColWidthRef.current };
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }

  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setLoading(true);
    setErr(null);
    try {
      const [mx, r, allV, st, rs, m, cats] = await Promise.all([
        listMatrix(pid),
        listRequirements(pid),
        listVerifications(),
        listVerificationStatuses(),
        listRequirementStatuses(),
        listVerificationMethodsByProject(pid),
        listCategories(),
      ]);
      setMatrix(mx);
      setReqs(r);
      setVers(allV.filter((v) => v.project_id === pid));
      setStatuses(st);
      setReqStatuses(rs);
      setMethods(m);
      setCategories(cats.filter((c) => c.project_id === pid));
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load matrix');
    } finally {
      setLoading(false);
    }
  }, [pid]);

  useEffect(() => {
    void load();
  }, [load]);

  const openSuspectDiff = useCallback(
    async (link: MatrixLink) => {
      setBusyKey(`${link.req_id}-${link.verification_id}`);
      try {
        const versions = await listRequirementVersionsByProject(pid, link.req_id);
        const ordered = [...versions].sort(
          (a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime() || a.id - b.id,
        );
        const triggerIndex =
          link.triggering_version_id == null
            ? ordered.length - 1
            : ordered.findIndex((version) => version.id === link.triggering_version_id);
        const newerIndex = triggerIndex > 0 ? triggerIndex : ordered.length - 1;
        const olderIndex = newerIndex - 1;
        setDiffVersions(versions);
        setDiffRequirementId(link.req_id);
        setDiffPair(
          olderIndex >= 0
            ? { oldVersionId: ordered[olderIndex]!.id, newVersionId: ordered[newerIndex]!.id }
            : null,
        );
      } catch (reason) {
        setErr(reason instanceof Error ? reason.message : 'Failed to load requirement versions');
      } finally {
        setBusyKey(null);
      }
    },
    [pid],
  );

  async function onClearSuspect(m: MatrixLink) {
    const token = csrfToken ?? '';
    if (!token) return;
    setBusyKey(`${m.req_id}-${m.verification_id}`);
    try {
      await clearTraceabilitySuspect(m.req_id, m.verification_id, token);
      await load();
    } finally {
      setBusyKey(null);
    }
  }

  async function onExport() {
    setExporting(true);
    try {
      await downloadMatrixXlsx(pid);
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Export failed');
    } finally {
      setExporting(false);
    }
  }

  const reqById = useMemo(() => new Map(reqs.map((r) => [r.id, r])), [reqs]);
  const verById = useMemo(() => new Map(vers.map((v) => [v.id, v])), [vers]);
  const statusById = useMemo(() => new Map(statuses.map((s) => [s.id, s])), [statuses]);
  const reqStatusById = useMemo(() => new Map(reqStatuses.map((s) => [s.id, s])), [reqStatuses]);
  const methodById = useMemo(() => new Map(methods.map((m) => [m.id, m])), [methods]);
  const categoryById = useMemo(() => new Map(categories.map((c) => [c.id, c.title])), [categories]);

  const reqStatusOptions = useMemo(
    () => reqStatuses.filter((s) => s.project_id === pid).sort((a, b) => a.title.localeCompare(b.title)),
    [reqStatuses, pid],
  );
  const verStatusOptions = useMemo(
    () => statuses.filter((s) => s.project_id === pid).sort((a, b) => a.title.localeCompare(b.title)),
    [statuses, pid],
  );

  const linkByPair = useMemo(() => new Map(matrix.map((x) => [`${x.req_id}-${x.verification_id}`, x])), [matrix]);

  const q = globalSearch.trim().toLowerCase();
  const statusGroups = useMemo(() => new Set(params.statusGroups), [params.statusGroups]);
  const reqStatusFilter = useMemo(() => new Set(params.reqStatusIds), [params.reqStatusIds]);
  const verStatusFilter = useMemo(() => new Set(params.verStatusIds), [params.verStatusIds]);
  const suspectOnly = params.suspectOnly;

  const verStatusTitle = useCallback(
    (v: Verification | undefined) => {
      if (!v) return 'Unknown';
      return statusById.get(v.status_id)?.title ?? `Status #${v.status_id}`;
    },
    [statusById],
  );

  const verMatchesStatusGroup = useCallback(
    (verId: number) => {
      if (statusGroups.size === 0) return true;
      const v = verById.get(verId);
      if (!v) return false;
      return statusGroups.has(statusSemanticGroup(verStatusTitle(v), statusById.get(v.status_id)?.tag_color));
    },
    [statusGroups, verById, verStatusTitle, statusById],
  );

  const linkMatchesVerFilters = useCallback(
    (link: MatrixLink) => {
      if (!verMatchesStatusGroup(link.verification_id)) return false;
      if (verStatusFilter.size > 0) {
        const v = verById.get(link.verification_id);
        if (!v || !verStatusFilter.has(v.status_id)) return false;
      }
      return true;
    },
    [verMatchesStatusGroup, verStatusFilter, verById],
  );

  const reqMatches = useCallback(
    (reqId: number) => {
      const r = reqById.get(reqId);
      if (!r) return false;
      return [r.reference_code, r.title, String(r.id)].join(' ').toLowerCase().includes(q);
    },
    [reqById, q],
  );

  const verMatches = useCallback(
    (verId: number) => {
      const t = verById.get(verId);
      if (!t) return false;
      const st = statusById.get(t.status_id);
      const meth = t.verification_method_id != null ? methodById.get(t.verification_method_id) : undefined;
      return [t.reference_code, t.name, st?.title, st?.tag, meth?.title, String(t.id)]
        .join(' ')
        .toLowerCase()
        .includes(q);
    },
    [verById, statusById, methodById, q],
  );

  const verColumnFilterActive = statusGroups.size > 0 || verStatusFilter.size > 0;

  const filteredReqs = useMemo(
    () =>
      reqs.filter((r) => {
        if (reqStatusFilter.size > 0 && !reqStatusFilter.has(r.status_id)) return false;
        if (suspectOnly && !matrix.some((m) => m.req_id === r.id && m.suspect)) return false;
        if (verColumnFilterActive && !matrix.some((m) => m.req_id === r.id && linkMatchesVerFilters(m))) return false;
        if (!q) return true;
        if (reqMatches(r.id)) return true;
        return matrix.some((m) => m.req_id === r.id && verMatches(m.verification_id));
      }),
    [reqs, matrix, reqStatusFilter, suspectOnly, verColumnFilterActive, linkMatchesVerFilters, q, reqMatches, verMatches],
  );

  const categoryOf = useCallback((r: Requirement) => categoryById.get(r.category_id) ?? UNCATEGORISED, [categoryById]);

  /** Rows: category blocks when sorted by requirement; flat when sorted by a verification column. */
  const orderedReqs = useMemo(() => {
    const arr = [...filteredReqs];
    const mult = params.dir === 'asc' ? 1 : -1;
    const sort = params.sort;
    if (sort.kind === 'verification') {
      arr.sort((a, b) => mult * compareByVerificationColumn(a, b, sort.verId, linkByPair));
      return arr;
    }
    const rank = (c: string) => (c === UNCATEGORISED ? 1 : 0);
    arr.sort((a, b) => {
      const ca = categoryOf(a);
      const cb = categoryOf(b);
      return rank(ca) - rank(cb) || ca.localeCompare(cb) || mult * compareRefCode(a, b);
    });
    return arr;
  }, [filteredReqs, params.sort, params.dir, linkByPair, categoryOf]);

  const displayVers = useMemo(
    () =>
      [...vers]
        .sort((a, b) => (a.reference_code || '').localeCompare(b.reference_code || '', undefined, { numeric: true }))
        .filter((v) => {
          if (suspectOnly && !matrix.some((m) => m.verification_id === v.id && m.suspect)) return false;
          if (!verMatchesStatusGroup(v.id)) return false;
          if (verStatusFilter.size > 0 && !verStatusFilter.has(v.status_id)) return false;
          if (!q) return true;
          if (verMatches(v.id)) return true;
          return matrix.some((m) => m.verification_id === v.id && reqMatches(m.req_id));
        }),
    [vers, matrix, suspectOnly, verMatchesStatusGroup, verStatusFilter, q, verMatches, reqMatches],
  );

  const rowIndexById = useMemo(() => new Map(orderedReqs.map((r, i) => [r.id, i])), [orderedReqs]);
  const colIndexById = useMemo(() => new Map(displayVers.map((v, j) => [v.id, j])), [displayVers]);

  const rows: MatrixRowItem[] = useMemo(
    () =>
      orderedReqs.map((r) => ({
        id: r.id,
        code: r.reference_code || `#${r.id}`,
        title: r.title,
        category: categoryOf(r),
        statusTitle: reqStatusById.get(r.status_id)?.title ?? `Status #${r.status_id}`,
        approvalState: r.approval_state,
      })),
    [orderedReqs, categoryOf, reqStatusById],
  );

  const cols: MatrixColItem[] = useMemo(
    () =>
      displayVers.map((v) => ({
        id: v.id,
        code: v.reference_code || `#${v.id}`,
        name: v.name,
        statusTitle: verStatusTitle(v),
        method: v.verification_method_id != null ? methodById.get(v.verification_method_id)?.title ?? null : null,
      })),
    [displayVers, verStatusTitle, methodById],
  );

  const cells: MatrixCellItem[] = useMemo(() => {
    const out: MatrixCellItem[] = [];
    for (const link of matrix) {
      const row = rowIndexById.get(link.req_id);
      const col = colIndexById.get(link.verification_id);
      if (row == null || col == null) continue;
      if (suspectOnly && !link.suspect) continue;
      const v = verById.get(link.verification_id);
      const title = verStatusTitle(v);
      const tag = v ? statusById.get(v.status_id)?.tag_color : undefined;
      const hex = tag?.trim() ?? '';
      out.push({
        row,
        col,
        link,
        statusTitle: title,
        group: statusSemanticGroup(title, tag),
        symbol: statusGlyph(title, tag).symbol,
        hex: /^#[0-9A-Fa-f]{6}$/.test(hex) ? hex : null,
      });
    }
    return out;
  }, [matrix, rowIndexById, colIndexById, suspectOnly, verById, verStatusTitle, statusById]);

  const groups = useMemo(
    () => (params.sort.kind === 'requirement' ? groupRuns(rows.map((r) => r.category)) : []),
    [params.sort.kind, rows],
  );

  const suspects: SuspectItem[] = useMemo(
    () =>
      cells
        .filter((c) => c.link.suspect)
        .sort((a, b) => a.row - b.row || a.col - b.col)
        .map((c) => ({ row: c.row, col: c.col, reqCode: rows[c.row]!.code, verCode: cols[c.col]!.code, link: c.link })),
    [cells, rows, cols],
  );

  const linkedReqIds = useMemo(() => new Set(matrix.map((m) => m.req_id)), [matrix]);
  const linkedVerIds = useMemo(() => new Set(matrix.map((m) => m.verification_id)), [matrix]);
  const reqsWithoutVerification: GapItem[] = useMemo(
    () => rows.flatMap((r, i) => (linkedReqIds.has(r.id) ? [] : [{ index: i, code: r.code, title: r.title }])),
    [rows, linkedReqIds],
  );
  const versWithoutRequirement: GapItem[] = useMemo(
    () => cols.flatMap((v, j) => (linkedVerIds.has(v.id) ? [] : [{ index: j, code: v.code, title: v.name }])),
    [cols, linkedVerIds],
  );

  /** Roll-up: count visible linked cells by verification status. */
  const statusRollup = useMemo(() => {
    const counts = new Map<string, { n: number; tagColor: string | null }>();
    for (const c of cells) {
      const v = verById.get(c.link.verification_id);
      const tag = v ? statusById.get(v.status_id)?.tag_color ?? null : null;
      const prev = counts.get(c.statusTitle);
      counts.set(c.statusTitle, { n: (prev?.n ?? 0) + 1, tagColor: tag ?? prev?.tagColor ?? null });
    }
    return [...counts.entries()].sort((a, b) => b[1].n - a[1].n);
  }, [cells, verById, statusById]);

  // Selection resolved against the current rows/columns (disappears if filtered out).
  const selected = useMemo(() => {
    if (!selection) return null;
    if (selection.kind === 'suspect') {
      const row = rowIndexById.get(selection.req);
      const col = colIndexById.get(selection.ver);
      return row != null && col != null ? { kind: 'suspect' as const, row, col } : null;
    }
    if (selection.kind === 'row') {
      const row = rowIndexById.get(selection.req);
      return row != null ? { kind: 'row' as const, row } : null;
    }
    const col = colIndexById.get(selection.ver);
    return col != null ? { kind: 'column' as const, col } : null;
  }, [selection, rowIndexById, colIndexById]);

  const focus: MatrixFocus | null = useMemo(() => {
    if (!selected || !selection) return null;
    const key = `${selection.kind}-${selection.seq}`;
    if (selected.kind === 'suspect')
      return { key, kind: 'suspect', rows: [selected.row, selected.row], cols: [selected.col, selected.col] };
    if (selected.kind === 'row') return { key, kind: 'row', rows: [selected.row, selected.row], cols: null };
    return { key, kind: 'column', rows: null, cols: [selected.col, selected.col] };
  }, [selected, selection]);

  const seq = () => (selection?.seq ?? 0) + 1;
  const toggleSelection = (next: Selection, isSame: boolean) => setSelection(isSame ? null : next);

  if (loading && reqs.length === 0) {
    return (
      <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
        Loading matrix…
      </div>
    );
  }

  const filtersActive = hasMatrixFilters(params);
  const groupButton = (id: StatusSemanticGroup, text: string, symbol: string) => {
    const active = statusGroups.has(id);
    return (
      <button
        key={id}
        type="button"
        aria-pressed={active}
        title={`${active ? 'Remove' : 'Show'} ${text}`}
        onClick={() => update({ statusGroups: toggleIn<StatusSemanticGroup>(params.statusGroups, id) })}
        className={`inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-[11px] focus:outline-hidden focus-visible:ring-2 focus-visible:ring-stitch-accent ${
          active ? groupOn : groupOff
        }`}
      >
        <span className={`font-semibold ${GROUP_TEXT_CLASS[id]}`} aria-hidden>
          {symbol}
        </span>
        {text}
      </button>
    );
  };
  const statusCategory = (
    category: string,
    heading: string,
    options: { id: number; title: string; tag_color: string | null }[],
    selectedIds: number[],
    key: 'reqStatusIds' | 'verStatusIds',
  ) => {
    const byId = new Map(options.map((o) => [o.id, o]));
    return (
      <div className="flex flex-wrap items-center gap-1.5" role="group" aria-label={heading}>
        <span className={label}>{heading}</span>
        {selectedIds
          .map((id) => byId.get(id))
          .filter((st): st is (typeof options)[number] => st != null)
          .map((st) => (
            <StatusFilterChip
              key={st.id}
              title={st.title}
              tagColor={st.tag_color}
              category={category}
              onRemove={() => update({ [key]: toggleIn(params[key], st.id) })}
            />
          ))}
        <AddStatusMenu
          category={category}
          options={options}
          selected={selectedIds}
          onToggle={(id) => update({ [key]: toggleIn(params[key], id) })}
        />
      </div>
    );
  };

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-end gap-2">
        <button type="button" onClick={() => void onExport()} disabled={exporting} className={headerButton}>
          {exporting ? 'Exporting…' : 'Export Excel'}
        </button>
        <button type="button" onClick={() => void load()} className={headerButton}>
          Refresh
        </button>
      </div>

      <section
        aria-label="Matrix filters"
        className="rounded-xl border border-stitch-border bg-stitch-surface px-4 py-3 space-y-2.5"
      >
        <div className="flex flex-wrap items-center gap-x-5 gap-y-2">
          <div className="flex items-center gap-1.5">
            <span className={label}>Links</span>
            <SuspectSwitch checked={suspectOnly} onChange={(v) => update({ suspectOnly: v })} />
          </div>
          <div className="flex flex-wrap items-center gap-1" role="group" aria-label="Status groups">
            <span className={label}>Status groups</span>
            {STATUS_GROUP_OPTIONS.map(({ id, label: text, symbol }) => groupButton(id, text, symbol))}
          </div>
          <button
            type="button"
            onClick={() => update(clearMatrixFilters(params))}
            disabled={!filtersActive}
            className="ml-auto inline-flex items-center gap-1 rounded-md px-2 py-1 text-[11px] font-semibold text-stitch-accent hover:bg-stitch-higher disabled:cursor-default disabled:text-stitch-muted disabled:opacity-60 disabled:hover:bg-transparent focus:outline-hidden focus-visible:ring-2 focus-visible:ring-stitch-accent"
          >
            <span aria-hidden className="text-[13px] leading-none">
              ×
            </span>
            Clear all filters
          </button>
        </div>
        {reqStatusOptions.length > 0 || verStatusOptions.length > 0 ? (
          <div className="flex flex-wrap items-center gap-x-6 gap-y-2">
            {reqStatusOptions.length > 0
              ? statusCategory('requirement status', 'Requirement status', reqStatusOptions, params.reqStatusIds, 'reqStatusIds')
              : null}
            {verStatusOptions.length > 0
              ? statusCategory('verification status', 'Verification status', verStatusOptions, params.verStatusIds, 'verStatusIds')
              : null}
          </div>
        ) : null}
        <p className="font-mono text-[11px] text-stitch-muted" data-testid="matrix-stats">
          {rows.length} req × {cols.length} verifications · {cells.length} link{cells.length === 1 ? '' : 's'} ·{' '}
          {suspects.length} suspect · {reqsWithoutVerification.length + versWithoutRequirement.length} coverage gap
          {reqsWithoutVerification.length + versWithoutRequirement.length === 1 ? '' : 's'}
        </p>
      </section>

      {err ? (
        <p role="alert" className="rounded-lg border border-stitch-danger/40 bg-stitch-danger/10 px-3 py-2 text-sm text-stitch-fg">
          {err}
        </p>
      ) : null}

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_280px] items-start">
        <div className="min-w-0 space-y-2">
          <MatrixGrid
            rows={rows}
            cols={cols}
            cells={cells}
            groups={groups}
            basePath={basePath}
            sort={params.sort}
            dir={params.dir}
            onSortRequirement={() => update(nextSort(params, { kind: 'requirement' }))}
            onSortVerification={(verId) => update(nextSort(params, { kind: 'verification', verId }))}
            rowHeaderWidth={reqColWidthPx}
            onResizeStart={onReqColResizeStart}
            focus={focus}
            onOpenRequirement={(id) => navigate(`${basePath}/requirements/${id}`)}
          />
          <p className="text-[11px] text-stitch-muted">
            A symbol in row <b>i</b>, column <b>j</b>: requirement i is verified by verification j, with that
            verification&apos;s status. Click a symbol to open the requirement; hover for details.
          </p>
        </div>
        <MatrixSidePanel
          suspects={suspects}
          reqsWithoutVerification={reqsWithoutVerification}
          versWithoutRequirement={versWithoutRequirement}
          rollup={statusRollup}
          selected={selected}
          onSelectSuspect={(row, col) =>
            toggleSelection(
              { kind: 'suspect', req: rows[row]!.id, ver: cols[col]!.id, seq: seq() },
              selected?.kind === 'suspect' && selected.row === row && selected.col === col,
            )
          }
          onSelectRow={(row) =>
            toggleSelection({ kind: 'row', req: rows[row]!.id, seq: seq() }, selected?.kind === 'row' && selected.row === row)
          }
          onSelectColumn={(col) =>
            toggleSelection(
              { kind: 'column', ver: cols[col]!.id, seq: seq() },
              selected?.kind === 'column' && selected.col === col,
            )
          }
          onReview={(link) => void openSuspectDiff(link)}
          onClear={(link) => void onClearSuspect(link)}
          busyKey={busyKey}
          canClear={Boolean(csrfToken) && !archived}
        />
      </div>

      {diffRequirementId != null ? (
        <RequirementVersionDiffDialog
          open
          onClose={() => {
            setDiffRequirementId(null);
            setDiffVersions([]);
            setDiffPair(null);
          }}
          projectId={pid}
          requirementId={diffRequirementId}
          versions={diffVersions}
          initialPair={diffPair}
        />
      ) : null}
    </div>
  );
}

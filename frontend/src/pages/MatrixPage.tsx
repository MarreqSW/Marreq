import type { CSSProperties } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link, useOutletContext, useParams } from 'react-router-dom';
import {
  clearTraceabilitySuspect,
  listMatrix,
  listRequirements,
  listVerificationMethodsByProject,
  listVerificationStatuses,
  listVerifications,
} from '@/api/client';
import { StatusBadge } from '@/components/StatusBadge';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';
import type {
  MatrixLink,
  Requirement,
  Verification,
  VerificationMethod,
  VerificationStatus,
} from '@/api/types';
import type { ProjectOutletContext } from '@/types/projectOutlet';

const PAGE_SIZE = 40;

/** Verification columns: wide enough for horizontal reference labels. */
const VER_COL_CLASS =
  'w-[4.75rem] min-w-[4.75rem] max-w-[5.5rem] box-border';

const REQ_COL_DEFAULT_PX = 152;
const REQ_COL_MIN_PX = 96;
const REQ_COL_MAX_PX = 520;

function reqColStorageKey(projectId: number) {
  return `reqman-matrix-req-col-w-${projectId}`;
}

function clampReqColW(n: number) {
  return Math.min(REQ_COL_MAX_PX, Math.max(REQ_COL_MIN_PX, Math.round(n)));
}

type MatrixSortColumn =
  | { kind: 'requirement' }
  | { kind: 'verification'; verId: number };

function compareRefCode(a: Requirement, b: Requirement): number {
  return (a.reference_code || '').localeCompare(b.reference_code || '', undefined, {
    numeric: true,
  });
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

/** Visual + tooltip for a verification status in a matrix cell (catalog titles vary by project). */
function statusGlyph(
  statusTitle: string,
  tagColor: string | null | undefined,
): { symbol: string; className: string } {
  const t = statusTitle.toLowerCase();
  if (t.includes('fail') || t.includes('reject')) {
    return { symbol: '✗', className: 'text-red-300' };
  }
  if (
    /\bpass\b/.test(t) ||
    t.includes('passed') ||
    t.includes('success') ||
    t.includes('complete') ||
    t === 'ok'
  ) {
    return { symbol: '✓', className: 'text-emerald-400' };
  }
  if (t.includes('verified') || t.includes('accepted')) {
    return { symbol: '✓', className: 'text-amber-300' };
  }
  if (t.includes('pending') || t.includes('review') || t.includes('progress') || t.includes('blocked')) {
    return { symbol: '◐', className: 'text-amber-200' };
  }
  if (t.includes('draft')) {
    return { symbol: '○', className: 'text-stitch-muted' };
  }
  if (tagColor && /^#[0-9A-Fa-f]{6}$/.test(tagColor.trim())) {
    return { symbol: '●', className: '' };
  }
  return { symbol: '●', className: 'text-stitch-muted' };
}

export default function MatrixPage() {
  const { globalSearch } = useOutletContext<ProjectOutletContext>();
  const { projectId: projectIdParam } = useParams();
  const pid = Number(projectIdParam);
  const { csrfToken, dashboard } = useDashboard();

  const [matrix, setMatrix] = useState<MatrixLink[]>([]);
  const [reqs, setReqs] = useState<Requirement[]>([]);
  const [vers, setVers] = useState<Verification[]>([]);
  const [statuses, setStatuses] = useState<VerificationStatus[]>([]);
  const [methods, setMethods] = useState<VerificationMethod[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [page, setPage] = useState(1);
  const [suspectOnly, setSuspectOnly] = useState(false);
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [sortColumn, setSortColumn] = useState<MatrixSortColumn>({ kind: 'requirement' });
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');
  const [reqColWidthPx, setReqColWidthPx] = useState(REQ_COL_DEFAULT_PX);
  const resizeDragRef = useRef<{ startX: number; startW: number } | null>(null);
  const reqColWidthRef = useRef(REQ_COL_DEFAULT_PX);
  reqColWidthRef.current = reqColWidthPx;

  useEffect(() => {
    if (!Number.isFinite(pid)) return;
    try {
      const raw = localStorage.getItem(reqColStorageKey(pid));
      const n = raw ? parseInt(raw, 10) : NaN;
      if (Number.isFinite(n)) {
        setReqColWidthPx(clampReqColW(n));
      } else {
        setReqColWidthPx(REQ_COL_DEFAULT_PX);
      }
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
      if (Number.isFinite(pid)) {
        try {
          localStorage.setItem(reqColStorageKey(pid), String(reqColWidthRef.current));
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
  }, [pid]);

  function onReqColResizeStart(e: React.MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    resizeDragRef.current = { startX: e.clientX, startW: reqColWidthRef.current };
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }

  const reqColStyle = useMemo(
    (): CSSProperties => ({
      width: reqColWidthPx,
      minWidth: reqColWidthPx,
      maxWidth: reqColWidthPx,
      boxSizing: 'border-box',
    }),
    [reqColWidthPx],
  );

  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setLoading(true);
    setErr(null);
    try {
      const [mx, r, allV, st, m] = await Promise.all([
        listMatrix(pid),
        listRequirements(pid),
        listVerifications(),
        listVerificationStatuses(),
        listVerificationMethodsByProject(pid),
      ]);
      setMatrix(mx);
      setReqs(r);
      setVers(allV.filter((v) => v.project_id === pid));
      setStatuses(st);
      setMethods(m);
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load matrix');
    } finally {
      setLoading(false);
    }
  }, [pid]);

  useEffect(() => {
    void load();
  }, [load]);

  const reqById = useMemo(() => new Map(reqs.map((r) => [r.id, r])), [reqs]);
  const verById = useMemo(() => new Map(vers.map((v) => [v.id, v])), [vers]);
  const statusById = useMemo(() => new Map(statuses.map((s) => [s.id, s])), [statuses]);
  const methodById = useMemo(() => new Map(methods.map((m) => [m.id, m])), [methods]);

  const linkByPair = useMemo(() => {
    const m = new Map<string, MatrixLink>();
    for (const x of matrix) {
      m.set(`${x.req_id}-${x.verification_id}`, x);
    }
    return m;
  }, [matrix]);

  const q = globalSearch.trim().toLowerCase();

  const reqMatches = useCallback(
    (reqId: number) => {
      const r = reqById.get(reqId);
      if (!r) return false;
      const blob = [r.reference_code, r.title, String(r.id)].join(' ').toLowerCase();
      return blob.includes(q);
    },
    [reqById, q],
  );

  const verMatches = useCallback(
    (verId: number) => {
      const t = verById.get(verId);
      if (!t) return false;
      const st = statusById.get(t.status_id);
      const meth =
        t.verification_method_id != null ? methodById.get(t.verification_method_id) : undefined;
      const blob = [
        t.reference_code,
        t.name,
        st?.title,
        st?.tag,
        meth?.title,
        String(t.id),
      ]
        .join(' ')
        .toLowerCase();
      return blob.includes(q);
    },
    [verById, statusById, methodById, q],
  );

  const filteredReqs = useMemo(() => {
    return reqs.filter((r) => {
      if (suspectOnly) {
        const hit = matrix.some((m) => m.req_id === r.id && m.suspect);
        if (!hit) return false;
      }
      if (!q) return true;
      if (reqMatches(r.id)) return true;
      return matrix.some((m) => m.req_id === r.id && verMatches(m.verification_id));
    });
  }, [reqs, matrix, suspectOnly, q, reqMatches, verMatches]);

  const sortedDisplayReqs = useMemo(() => {
    const arr = [...filteredReqs];
    const mult = sortDir === 'asc' ? 1 : -1;
    if (sortColumn.kind === 'requirement') {
      arr.sort((a, b) => mult * compareRefCode(a, b));
      return arr;
    }
    arr.sort(
      (a, b) =>
        mult * compareByVerificationColumn(a, b, sortColumn.verId, linkByPair),
    );
    return arr;
  }, [filteredReqs, sortColumn, sortDir, linkByPair]);

  const displayVers = useMemo(() => {
    const sorted = [...vers].sort((a, b) =>
      (a.reference_code || '').localeCompare(b.reference_code || '', undefined, { numeric: true }),
    );
    return sorted.filter((v) => {
      if (suspectOnly) {
        const hit = matrix.some((m) => m.verification_id === v.id && m.suspect);
        if (!hit) return false;
      }
      if (!q) return true;
      if (verMatches(v.id)) return true;
      return matrix.some((m) => m.verification_id === v.id && reqMatches(m.req_id));
    });
  }, [vers, matrix, suspectOnly, q, verMatches, reqMatches]);

  const pageCount = Math.max(1, Math.ceil(sortedDisplayReqs.length / PAGE_SIZE));
  const safePage = Math.min(page, pageCount);
  const reqSlice = sortedDisplayReqs.slice(
    (safePage - 1) * PAGE_SIZE,
    safePage * PAGE_SIZE,
  );

  useEffect(() => {
    setPage(1);
  }, [suspectOnly, q, sortColumn, sortDir]);

  function onSortRequirementHeaderClick(e: React.MouseEvent) {
    if ((e.target as HTMLElement).closest('[data-matrix-resize-handle]')) return;
    setSortColumn((prev) => {
      if (prev.kind === 'requirement') {
        setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
        return prev;
      }
      setSortDir('asc');
      return { kind: 'requirement' };
    });
  }

  function onSortVerificationHeaderClick(verId: number) {
    setSortColumn((prev) => {
      if (prev.kind === 'verification' && prev.verId === verId) {
        setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
        return prev;
      }
      setSortDir('desc');
      return { kind: 'verification', verId };
    });
  }

  function sortIndicatorForRequirement(): string {
    if (sortColumn.kind !== 'requirement') return '↕';
    return sortDir === 'asc' ? '↑' : '↓';
  }

  function sortIndicatorForVer(verId: number): string {
    if (sortColumn.kind !== 'verification' || sortColumn.verId !== verId) return '↕';
    return sortDir === 'asc' ? '↑' : '↓';
  }

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  async function onClearSuspect(m: MatrixLink) {
    const token = csrfToken ?? '';
    if (!token) return;
    const key = `${m.req_id}-${m.verification_id}`;
    setBusyKey(key);
    try {
      await clearTraceabilitySuspect(m.req_id, m.verification_id, token);
      await load();
    } finally {
      setBusyKey(null);
    }
  }

  /** Roll-up: count linked cells by verification status (for legend). */
  const statusRollup = useMemo(() => {
    const counts = new Map<string, { n: number; tagColor: string | null }>();
    for (const link of matrix) {
      if (suspectOnly && !link.suspect) continue;
      if (q && !reqMatches(link.req_id) && !verMatches(link.verification_id)) continue;
      const t = verById.get(link.verification_id);
      const st = t ? statusById.get(t.status_id) : undefined;
      const label = st?.title ?? (t ? `Status #${t.status_id}` : 'Unknown');
      const prev = counts.get(label);
      counts.set(label, {
        n: (prev?.n ?? 0) + 1,
        tagColor: st?.tag_color ?? prev?.tagColor ?? null,
      });
    }
    return [...counts.entries()].sort((a, b) => b[1].n - a[1].n);
  }, [matrix, suspectOnly, q, verById, statusById, reqMatches, verMatches]);

  if (loading) {
    return (
      <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
        Loading matrix…
      </div>
    );
  }

  if (err) {
    return (
      <div className="p-4 rounded-xl bg-red-500/10 border border-red-500/25 text-red-200 text-sm">
        {err}
      </div>
    );
  }

  const linkCount = matrix.filter((m) => {
    if (suspectOnly && !m.suspect) return false;
    if (!q) return true;
    return reqMatches(m.req_id) || verMatches(m.verification_id);
  }).length;

  return (
    <div>
      <StitchPageHeader
        projectName={projectName}
        section="Matrix"
        title="Traceability matrix"
        subtitle="Requirements in rows, verifications in columns. Click any column header to sort rows (↑↓). Symbols show link + test status; empty cells are not linked."
      >
        <button
          type="button"
          onClick={() => void load()}
          className="text-xs font-bold uppercase tracking-wider text-stitch-muted border border-stitch-border rounded-md px-3 py-2 hover:bg-stitch-higher"
        >
          Refresh
        </button>
      </StitchPageHeader>

      <div className="flex flex-wrap items-center justify-between gap-4 mb-4">
        <label className="flex items-center gap-2 text-sm text-stitch-muted cursor-pointer">
          <input
            type="checkbox"
            checked={suspectOnly}
            onChange={(e) => setSuspectOnly(e.target.checked)}
            className="rounded border-stitch-border text-stitch-accent"
          />
          Suspect links only (rows &amp; columns filtered)
        </label>
        <p className="text-[10px] text-stitch-muted font-mono">
          {sortedDisplayReqs.length} req × {displayVers.length} test
          {linkCount > 0 ? ` · ${linkCount} visible link${linkCount === 1 ? '' : 's'}` : ''}
        </p>
      </div>

      <div className="mb-4 rounded-xl border border-stitch-border bg-stitch-elevated/50 px-4 py-3 text-[11px] text-stitch-muted space-y-2">
        <p className="font-bold text-[10px] uppercase tracking-widest text-stitch-fg/80">Symbols</p>
        <div className="flex flex-wrap gap-x-4 gap-y-1">
          <span>
            <span className="text-emerald-400 font-semibold mr-1">✓</span> pass / complete
          </span>
          <span>
            <span className="text-amber-300 font-semibold mr-1">✓</span> verified / accepted
          </span>
          <span>
            <span className="text-amber-200 font-semibold mr-1">◐</span> pending / review
          </span>
          <span>
            <span className="text-stitch-muted font-semibold mr-1">○</span> draft
          </span>
          <span>
            <span className="text-red-300 font-semibold mr-1">✗</span> fail / reject
          </span>
          <span>
            <span className="text-stitch-muted font-semibold mr-1">●</span> other (see tooltip)
          </span>
          <span>
            <span className="text-amber-400 font-semibold mr-1">⚠</span> suspect link
          </span>
        </div>
      </div>

      {statusRollup.length > 0 && (
        <div className="mb-4 flex flex-wrap items-center gap-2">
          <span className="text-[10px] uppercase tracking-widest text-stitch-muted font-bold">
            Linked cells by status
          </span>
          {statusRollup.map(([label, { n, tagColor }]) => (
            <span key={label} className="inline-flex items-center gap-1.5">
              <StatusBadge title={label} tagColor={tagColor} />
              <span className="text-xs text-stitch-muted tabular-nums">×{n}</span>
            </span>
          ))}
        </div>
      )}

      <div className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
        <div className="overflow-x-auto overflow-y-auto max-h-[min(70vh,720px)]">
          <table className="w-max min-w-full text-left text-sm border-collapse">
            <thead className="sticky top-0 z-20">
              <tr className="border-b border-stitch-border bg-stitch-elevated">
                <th
                  scope="col"
                  style={reqColStyle}
                  className="sticky left-0 z-30 bg-stitch-elevated border-r border-stitch-border pl-2 pr-1 py-2 text-[10px] uppercase tracking-widest text-stitch-muted shadow-[2px_0_8px_rgba(0,0,0,0.12)] relative select-none"
                >
                  <button
                    type="button"
                    title="Sort rows by requirement reference. Click again to reverse."
                    onClick={onSortRequirementHeaderClick}
                    className="w-full text-left flex items-center justify-between gap-1 pr-5 rounded-sm hover:bg-white/[0.06] focus:outline-none focus-visible:ring-2 focus-visible:ring-stitch-accent/40 -ml-0.5 pl-0.5 py-0.5"
                  >
                    <span className="block truncate">Requirement</span>
                    <span className="shrink-0 font-mono text-stitch-accent opacity-90" aria-hidden>
                      {sortIndicatorForRequirement()}
                    </span>
                  </button>
                  <button
                    type="button"
                    data-matrix-resize-handle
                    aria-label="Resize requirement column"
                    title="Drag to resize"
                    onMouseDown={onReqColResizeStart}
                    className="absolute right-0 top-0 bottom-0 w-2 cursor-col-resize z-40 border-0 bg-transparent p-0 hover:bg-stitch-accent/25 active:bg-stitch-accent/40 focus:outline-none focus-visible:ring-2 focus-visible:ring-stitch-accent/50 rounded-sm"
                  />
                </th>
                {displayVers.map((v) => {
                  const refLabel = v.reference_code || `#${v.id}`;
                  return (
                  <th
                    key={v.id}
                    scope="col"
                    className={`align-top p-0 border-l border-stitch-border/60 select-none ${VER_COL_CLASS}`}
                  >
                    <div className="flex flex-col items-stretch gap-1 px-1 py-2">
                      <button
                        type="button"
                        title={`${refLabel} — ${v.name}. Sort rows by this column; click again to reverse.`}
                        onClick={() => onSortVerificationHeaderClick(v.id)}
                        className="flex flex-col items-center gap-0.5 w-full cursor-pointer hover:bg-white/[0.06] transition-colors border-0 bg-transparent rounded-md py-1 outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-stitch-accent/50"
                      >
                        <span className="font-mono text-[9px] font-bold text-stitch-accent leading-snug text-center line-clamp-3 break-all w-full px-0.5">
                          {refLabel}
                        </span>
                        <span
                          className="font-mono text-[9px] text-stitch-muted"
                          aria-hidden
                        >
                          {sortIndicatorForVer(v.id)}
                        </span>
                      </button>
                      <Link
                        to={`/p/${pid}/verifications/${v.id}`}
                        title={`Open ${refLabel}`}
                        className="shrink-0 py-0.5 text-[8px] font-bold uppercase tracking-tighter text-center text-stitch-muted hover:text-stitch-accent border-t border-stitch-border/40"
                      >
                        View
                      </Link>
                    </div>
                  </th>
                  );
                })}
              </tr>
            </thead>
            <tbody className="divide-y divide-stitch-border/80">
              {displayVers.length === 0 ? (
                <tr>
                  <td colSpan={1} className="px-4 py-10 text-center text-stitch-muted">
                    No verifications match filters.
                  </td>
                </tr>
              ) : reqSlice.length === 0 ? (
                <tr>
                  <td
                    colSpan={displayVers.length + 1}
                    className="px-4 py-10 text-center text-stitch-muted"
                  >
                    No requirements match filters.
                  </td>
                </tr>
              ) : (
                reqSlice.map((r) => (
                  <tr key={r.id} className="hover:bg-white/[0.02]">
                    <th
                      scope="row"
                      style={reqColStyle}
                      className="sticky left-0 z-10 bg-stitch-surface border-r border-stitch-border pl-2 pr-2 py-2 text-left align-middle shadow-[2px_0_8px_rgba(0,0,0,0.08)] overflow-hidden"
                    >
                      <Link
                        to={`/p/${pid}/requirements/${r.id}`}
                        className="font-mono text-xs text-stitch-accent hover:underline block truncate"
                        title={r.reference_code ?? undefined}
                      >
                        {r.reference_code ?? `#${r.id}`}
                      </Link>
                      <span className="text-[10px] text-stitch-muted line-clamp-2 font-normal break-words">
                        {r.title}
                      </span>
                    </th>
                    {displayVers.map((v) => {
                      const link = linkByPair.get(`${r.id}-${v.id}`);
                      const test = verById.get(v.id);
                      const vst = test ? statusById.get(test.status_id) : undefined;
                      const statusTitle = vst?.title ?? (test ? `Status #${test.status_id}` : '—');
                      const { symbol, className } = statusGlyph(statusTitle, vst?.tag_color);
                      const bkey = `${r.id}-${v.id}`;
                      const hex = vst?.tag_color?.trim() ?? '';
                      const dotStyle =
                        symbol === '●' && /^#[0-9A-Fa-f]{6}$/.test(hex)
                          ? { color: hex }
                          : undefined;

                      return (
                        <td
                          key={v.id}
                          className={`border-l border-stitch-border/50 text-center align-middle p-1 ${VER_COL_CLASS}`}
                        >
                          {link ? (
                            <div
                              className="flex flex-col items-center gap-0.5 min-h-[2.25rem] justify-center"
                              title={`${statusTitle}${link.suspect ? ' · Suspect (re-review)' : ''} · ${r.reference_code ?? r.id} ↔ ${v.reference_code ?? v.id}`}
                            >
                              <span className="flex items-center gap-0.5 leading-none">
                                {link.suspect ? (
                                  <span className="text-amber-400 text-sm" aria-hidden>
                                    ⚠
                                  </span>
                                ) : null}
                                <span
                                  className={`text-lg font-semibold ${className}`}
                                  style={dotStyle}
                                >
                                  {symbol}
                                </span>
                              </span>
                              {link.suspect ? (
                                <button
                                  type="button"
                                  disabled={busyKey === bkey || !(csrfToken ?? '').length}
                                  onClick={() => void onClearSuspect(link)}
                                  className="text-[9px] font-bold uppercase text-stitch-accent hover:underline disabled:opacity-40 leading-none"
                                >
                                  {busyKey === bkey ? '…' : 'clear'}
                                </button>
                              ) : null}
                            </div>
                          ) : (
                            <span className="text-stitch-muted/40 text-xs select-none" title="Not linked">
                              ·
                            </span>
                          )}
                        </td>
                      );
                    })}
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>

      {pageCount > 1 && (
        <div className="flex items-center justify-between mt-4 text-xs text-stitch-muted">
          <span>
            Requirements page {safePage} / {pageCount} ({reqSlice.length} rows)
          </span>
          <div className="flex gap-2">
            <button
              type="button"
              disabled={safePage <= 1}
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              className="px-3 py-1 rounded border border-stitch-border disabled:opacity-30"
            >
              Prev
            </button>
            <button
              type="button"
              disabled={safePage >= pageCount}
              onClick={() => setPage((p) => Math.min(pageCount, p + 1))}
              className="px-3 py-1 rounded border border-stitch-border disabled:opacity-30"
            >
              Next
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

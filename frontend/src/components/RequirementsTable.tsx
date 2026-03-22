import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { useDashboard } from '@/context/DashboardContext';
import {
  getMyPermissions,
  listCategories,
  listProjectMembers,
  listRequirementStatuses,
  listRequirements,
  listUsersOptional,
  listVerificationMethodsByProject,
  patchRequirementByProject,
} from '@/api/client';
import type {
  Category,
  EffectivePermissions,
  ProjectMember,
  Requirement,
  RequirementPatchBody,
  RequirementStatus,
  User,
  VerificationMethod,
} from '@/api/types';
import { StatusBadge } from './StatusBadge';

function formatModified(iso: string): string {
  try {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    const h = String(d.getHours()).padStart(2, '0');
    const min = String(d.getMinutes()).padStart(2, '0');
    return `${y}-${m}-${day} ${h}:${min}`;
  } catch {
    return iso;
  }
}

function escapeCsv(s: string): string {
  if (/[",\n]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
  return s;
}

function approvalLabel(state: string): string {
  return state.replace(/_/g, ' ').toUpperCase();
}

/** Comma-separated verification method titles for table display. */
function verificationMethodsText(ids: number[], methods: VerificationMethod[]): string {
  if (!ids.length) return '—';
  return ids
    .map((id) => methods.find((m) => m.id === id)?.title?.trim() || `#${id}`)
    .join(', ');
}

/** Single open inline editor in the requirements table. */
type RequirementTableEditCell =
  | { reqId: number; kind: 'title' }
  | { reqId: number; kind: 'category' }
  | { reqId: number; kind: 'status' }
  | { reqId: number; kind: 'verification_methods' }
  | { reqId: number; kind: 'author' };

function mergeRequirementAfterPatch(req: Requirement, patch: RequirementPatchBody): Requirement {
  const next: Requirement = { ...req };
  if (patch.title !== undefined) next.title = patch.title;
  if (patch.description !== undefined) next.description = patch.description;
  if (patch.status_id !== undefined) next.status_id = patch.status_id;
  if (patch.author_id !== undefined) next.author_id = patch.author_id;
  if (patch.reviewer_id !== undefined) next.reviewer_id = patch.reviewer_id;
  if (patch.category_id !== undefined) next.category_id = patch.category_id;
  if (patch.applicability_id !== undefined) next.applicability_id = patch.applicability_id;
  if (patch.verification_method_ids !== undefined) {
    next.verification_method_ids = [...patch.verification_method_ids];
  }
  if (patch.custom_fields?.length) {
    const byId = new Map((next.custom_fields ?? []).map((c) => [c.field_id, { ...c }]));
    for (const u of patch.custom_fields) {
      const cur = byId.get(u.field_id) ?? {
        field_id: u.field_id,
        label: '',
        value: null as string | null,
      };
      byId.set(u.field_id, { ...cur, value: u.value });
    }
    next.custom_fields = [...byId.values()];
  }
  return next;
}

/** Compact page list: 1 2 3 4 5 … 50 style */
function paginationItems(current: number, total: number): (number | 'dots')[] {
  if (total <= 7) return Array.from({ length: total }, (_, i) => i + 1);
  if (current <= 4) return [1, 2, 3, 4, 5, 'dots', total];
  if (current >= total - 3) {
    return [1, 'dots', total - 4, total - 3, total - 2, total - 1, total];
  }
  return [1, 'dots', current - 1, current, current + 1, 'dots', total];
}

type ViewMode = 'table' | 'list';

export default function RequirementsTable({
  projectId,
  globalSearch,
  viewMode,
}: {
  projectId: number;
  globalSearch: string;
  viewMode: ViewMode;
}) {
  const { dashboard, csrfToken } = useDashboard();
  const projectSlug = dashboard?.projects?.find((p) => p.id === projectId)?.slug;
  const [requirements, setRequirements] = useState<Requirement[]>([]);
  const [statuses, setStatuses] = useState<RequirementStatus[]>([]);
  const [users, setUsers] = useState<User[] | null>(null);
  const [categories, setCategories] = useState<Category[]>([]);
  const [members, setMembers] = useState<ProjectMember[]>([]);
  const [verificationMethods, setVerificationMethods] = useState<VerificationMethod[]>([]);
  const [perms, setPerms] = useState<EffectivePermissions | null>(null);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [saveErr, setSaveErr] = useState<string | null>(null);
  const [savingId, setSavingId] = useState<number | null>(null);
  /** Last server-aligned row per id — avoid PATCH on blur when unchanged (reduces suspect churn). */
  const baselineRef = useRef<Map<number, Requirement>>(new Map());

  const [statusFilter, setStatusFilter] = useState<'all' | number>('all');
  const [pageSize, setPageSize] = useState(25);
  const [page, setPage] = useState(1);
  const [editCell, setEditCell] = useState<RequirementTableEditCell | null>(null);
  const inlineEditRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      setErr(null);
      try {
        const [reqs, st, u, cat, mem, methods, permRes] = await Promise.all([
          listRequirements(projectId),
          listRequirementStatuses(),
          listUsersOptional(),
          listCategories(),
          listProjectMembers(projectId),
          listVerificationMethodsByProject(projectId),
          getMyPermissions(projectId).catch(() => null),
        ]);
        if (cancelled) return;
        baselineRef.current = new Map(reqs.map((r) => [r.id, { ...r }]));
        setRequirements(reqs);
        setStatuses(st);
        setUsers(u);
        setCategories(cat.filter((c) => c.project_id === projectId));
        setMembers(mem);
        setVerificationMethods(methods);
        setPerms(permRes);
      } catch (e) {
        if (!cancelled) setErr(e instanceof Error ? e.message : 'Load failed');
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [projectId]);

  const statusById = useMemo(() => {
    const m = new Map<number, RequirementStatus>();
    for (const s of statuses) m.set(s.id, s);
    return m;
  }, [statuses]);

  const userById = useMemo(() => {
    const m = new Map<number, User>();
    if (users) for (const u of users) m.set(u.id, u);
    return m;
  }, [users]);

  const userLabel = useCallback(
    (id: number) => {
      const u = userById.get(id);
      if (u) return `${u.name} (${u.username})`;
      return `User #${id}`;
    },
    [userById],
  );

  const categoryById = useMemo(() => {
    const m = new Map<number, string>();
    for (const c of categories) m.set(c.id, c.title);
    return m;
  }, [categories]);

  const memberUserIds = useMemo(
    () => [...new Set(members.map((m) => m.user_id))].sort((a, b) => a - b),
    [members],
  );

  const reqTitleById = useMemo(() => {
    const m = new Map<number, string>();
    for (const r of requirements) m.set(r.id, r.title);
    return m;
  }, [requirements]);

  const canEdit = Boolean(perms?.edit_requirements && (csrfToken ?? '').length);

  const saveReq = useCallback(
    async (id: number, patch: RequirementPatchBody) => {
      const token = csrfToken ?? '';
      if (!token || !perms?.edit_requirements) return;
      setSaveErr(null);
      setEditCell((prev) => (prev?.reqId === id ? null : prev));
      setSavingId(id);
      try {
        await patchRequirementByProject(projectId, id, patch, token);
        setRequirements((prev) =>
          prev.map((r) => (r.id === id ? mergeRequirementAfterPatch(r, patch) : r)),
        );
        const base = baselineRef.current.get(id);
        if (base) {
          baselineRef.current.set(id, mergeRequirementAfterPatch(base, patch));
        }
      } catch (e) {
        setSaveErr(e instanceof Error ? e.message : 'Save failed');
      } finally {
        setSavingId(null);
      }
    },
    [csrfToken, perms?.edit_requirements, projectId],
  );

  const statusOptions = useMemo(() => {
    const forProject = statuses.filter((s) => s.project_id === projectId);
    return forProject.length > 0 ? forProject : statuses;
  }, [statuses, projectId]);

  const q = globalSearch.trim().toLowerCase();

  const filtered = useMemo(() => {
    return requirements.filter((req) => {
      if (statusFilter !== 'all' && req.status_id !== statusFilter) return false;
      if (!q) return true;
      const blob = [
        req.reference_code,
        req.title,
        req.description,
        String(req.id),
      ]
        .join(' ')
        .toLowerCase();
      return blob.includes(q);
    });
  }, [requirements, statusFilter, q]);

  useEffect(() => {
    setPage(1);
  }, [statusFilter, q, pageSize]);

  const pageCount = Math.max(1, Math.ceil(filtered.length / pageSize));
  const safePage = Math.min(page, pageCount);
  const sliceStart = (safePage - 1) * pageSize;
  const pageRows = useMemo(
    () => filtered.slice(sliceStart, sliceStart + pageSize),
    [filtered, sliceStart, pageSize],
  );

  useEffect(() => {
    setEditCell(null);
  }, [safePage, pageSize]);

  useEffect(() => {
    setEditCell(null);
  }, [viewMode]);

  useEffect(() => {
    if (editCell === null) return;
    const onDown = (e: MouseEvent) => {
      if (inlineEditRef.current?.contains(e.target as Node)) return;
      setEditCell(null);
    };
    const t = window.setTimeout(() => document.addEventListener('mousedown', onDown), 0);
    return () => {
      window.clearTimeout(t);
      document.removeEventListener('mousedown', onDown);
    };
  }, [editCell]);

  useEffect(() => {
    if (editCell === null) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setEditCell(null);
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [editCell]);

  useEffect(() => {
    if (editCell === null) return;
    const id = requestAnimationFrame(() => {
      const el = inlineEditRef.current?.querySelector<HTMLElement>(
        'input:not([type="checkbox"]), select, textarea',
      );
      el?.focus();
    });
    return () => cancelAnimationFrame(id);
  }, [editCell]);

  const resetFilters = useCallback(() => {
    setStatusFilter('all');
  }, []);

  const exportCsv = useCallback(() => {
    const headers = [
      'Key',
      'Title',
      'Category',
      'Parent id',
      'Status',
      'Approval',
      'Verification method ids',
      'Modified',
      'Author',
    ];
    const lines = [headers.join(',')];
    for (const req of filtered) {
      const st = statusById.get(req.status_id);
      lines.push(
        [
          escapeCsv(req.reference_code || `#${req.id}`),
          escapeCsv(req.title),
          escapeCsv(categoryById.get(req.category_id) ?? ''),
          escapeCsv(req.parent_id != null ? String(req.parent_id) : ''),
          escapeCsv(st?.title ?? ''),
          escapeCsv(req.approval_state),
          escapeCsv((req.verification_method_ids ?? []).join(';')),
          escapeCsv(formatModified(req.update_date)),
          escapeCsv(userLabel(req.author_id)),
        ].join(','),
      );
    }
    const blob = new Blob([lines.join('\n')], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `requirements-project-${projectId}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }, [filtered, statusById, categoryById, userLabel, projectId]);

  if (loading) {
    return (
      <div className="p-8 text-center text-stitch-muted font-mono italic border border-stitch-border rounded-xl bg-stitch-surface">
        Loading registry…
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

  const cellInput =
    'w-full min-w-[90px] max-w-[min(100%,280px)] text-xs bg-stitch-elevated border border-stitch-border rounded px-2 py-1.5 text-white focus:border-stitch-accent outline-none disabled:opacity-50';
  const cellSelect = `${cellInput} cursor-pointer`;
  /** Collapsed cell: click to open editor */
  const displayCellBtn =
    'w-full text-left text-xs text-white/90 leading-snug rounded-md px-1.5 py-1 hover:bg-white/[0.06] border border-transparent hover:border-stitch-border/40 transition-colors min-h-[1.75rem]';
  const closeCellEdit = () => setEditCell(null);

  return (
    <div className="space-y-6">
      {saveErr ? (
        <div className="rounded-lg border border-red-500/30 bg-red-500/10 text-red-100 text-sm px-4 py-2">
          {saveErr}
        </div>
      ) : null}
      {/* Filters bar — Image 2.html */}
      <div className="bg-stitch-elevated p-4 rounded-xl border border-stitch-border flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-2 md:gap-3">
          <div className="flex items-center gap-2 px-3 py-1.5 bg-stitch-surface border border-stitch-border rounded text-xs text-stitch-muted">
            <span className="material-symbols-outlined text-sm">filter_list</span>
            <span>Status:</span>
            <select
              value={statusFilter === 'all' ? 'all' : String(statusFilter)}
              onChange={(e) => {
                const v = e.target.value;
                setStatusFilter(v === 'all' ? 'all' : Number(v));
              }}
              className="bg-transparent text-stitch-accent font-bold text-xs border-none outline-none cursor-pointer"
            >
              <option value="all">All</option>
              {statusOptions.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.title}
                </option>
              ))}
            </select>
          </div>
          <div className="hidden sm:block h-4 w-px bg-stitch-border mx-1" />
          <button
            type="button"
            onClick={resetFilters}
            className="text-xs text-stitch-accent font-bold flex items-center gap-1 hover:underline"
          >
            <span className="material-symbols-outlined text-sm">restart_alt</span>
            Reset Filters
          </button>
        </div>
        <div className="flex items-center gap-4">
          <p className="text-[10px] text-stitch-muted font-mono">
            {filtered.length.toLocaleString()} Requirements found
          </p>
          <button
            type="button"
            onClick={exportCsv}
            title="Download CSV"
            className="p-2 text-stitch-muted hover:text-stitch-accent transition-colors"
          >
            <span className="material-symbols-outlined">file_download</span>
          </button>
          {projectSlug ? (
            <a
              href={`/p/${projectSlug}/requirements.xls`}
              title="Download Excel (classic)"
              className="p-2 text-stitch-muted hover:text-stitch-accent transition-colors"
            >
              <span className="material-symbols-outlined">table_chart</span>
            </a>
          ) : null}
        </div>
      </div>

      {viewMode === 'list' ? (
        <ul className="space-y-3">
          {pageRows.map((req) => {
            const st = statusById.get(req.status_id);
            const statusTitle = st?.title ?? `Status #${req.status_id}`;
            const methodIds = req.verification_method_ids ?? [];
            const busy = savingId === req.id;
            return (
              <li
                key={req.id}
                className="rounded-xl border border-stitch-border bg-stitch-surface p-4 shadow-stitch hover:bg-white/[0.03] transition-colors"
              >
                <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                  <div className="min-w-0 flex-1 space-y-3">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-mono text-sm text-stitch-accent font-semibold">
                        {req.reference_code || `#${req.id}`}
                      </span>
                      {editCell?.reqId === req.id && editCell.kind === 'status' && canEdit && !busy ? (
                        <div ref={inlineEditRef} className="min-w-[160px]">
                          <select
                            className={cellSelect}
                            value={req.status_id}
                            onChange={(e) => {
                              const v = Number(e.target.value);
                              setRequirements((prev) =>
                                prev.map((r) => (r.id === req.id ? { ...r, status_id: v } : r)),
                              );
                              void saveReq(req.id, { status_id: v });
                              closeCellEdit();
                            }}
                          >
                            {statusOptions.map((s) => (
                              <option key={s.id} value={s.id} className="bg-stitch-surface">
                                {s.title}
                              </option>
                            ))}
                            {!statusOptions.some((s) => s.id === req.status_id) && (
                              <option value={req.status_id} className="bg-stitch-surface">
                                Status #{req.status_id}
                              </option>
                            )}
                          </select>
                        </div>
                      ) : canEdit && !busy ? (
                        <button
                          type="button"
                          title="Click to edit status"
                          className="inline-flex"
                          onClick={() => setEditCell({ reqId: req.id, kind: 'status' })}
                        >
                          <StatusBadge title={statusTitle} tagColor={st?.tag_color} />
                        </button>
                      ) : (
                        <StatusBadge title={statusTitle} tagColor={st?.tag_color} />
                      )}
                      <span className="text-[10px] font-bold uppercase text-stitch-muted border border-stitch-border rounded px-1.5 py-0.5">
                        {approvalLabel(req.approval_state)}
                      </span>
                    </div>
                    <div>
                      <p className="text-[10px] uppercase text-stitch-muted font-bold tracking-wider mb-1">Title</p>
                      {editCell?.reqId === req.id && editCell.kind === 'title' && canEdit && !busy ? (
                        <div ref={inlineEditRef}>
                          <input
                            className={cellInput}
                            value={req.title}
                            onChange={(e) => {
                              const v = e.target.value;
                              setRequirements((prev) =>
                                prev.map((r) => (r.id === req.id ? { ...r, title: v } : r)),
                              );
                            }}
                            onBlur={(e) => {
                              const v = e.target.value.trim();
                              const b = baselineRef.current.get(req.id);
                              if (b && v === b.title) {
                                closeCellEdit();
                                return;
                              }
                              void saveReq(req.id, { title: v });
                              closeCellEdit();
                            }}
                          />
                        </div>
                      ) : canEdit && !busy ? (
                        <button
                          type="button"
                          title="Click to edit title"
                          className={`${displayCellBtn} text-left`}
                          onClick={() => setEditCell({ reqId: req.id, kind: 'title' })}
                        >
                          {req.title.trim() ? req.title : '—'}
                        </button>
                      ) : (
                        <p className="text-sm text-white/90">{req.title.trim() ? req.title : '—'}</p>
                      )}
                    </div>
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs">
                      <div>
                        <p className="text-[10px] uppercase text-stitch-muted font-bold tracking-wider mb-1">Category</p>
                        {editCell?.reqId === req.id && editCell.kind === 'category' && canEdit && !busy ? (
                          <div ref={inlineEditRef}>
                            <select
                              className={cellSelect}
                              value={req.category_id}
                              onChange={(e) => {
                                const v = Number(e.target.value);
                                setRequirements((prev) =>
                                  prev.map((r) => (r.id === req.id ? { ...r, category_id: v } : r)),
                                );
                                void saveReq(req.id, { category_id: v });
                                closeCellEdit();
                              }}
                            >
                              {categories.map((c) => (
                                <option key={c.id} value={c.id} className="bg-stitch-surface">
                                  {c.title}
                                </option>
                              ))}
                              {!categories.some((c) => c.id === req.category_id) && (
                                <option value={req.category_id} className="bg-stitch-surface">
                                  Category #{req.category_id}
                                </option>
                              )}
                            </select>
                          </div>
                        ) : canEdit && !busy ? (
                          <button
                            type="button"
                            className={displayCellBtn}
                            onClick={() => setEditCell({ reqId: req.id, kind: 'category' })}
                          >
                            {categoryById.get(req.category_id) ?? `Category #${req.category_id}`}
                          </button>
                        ) : (
                          <span className="text-white/90">
                            {categoryById.get(req.category_id) ?? `Category #${req.category_id}`}
                          </span>
                        )}
                      </div>
                      <div>
                        <p className="text-[10px] uppercase text-stitch-muted font-bold tracking-wider mb-1">Parent</p>
                        {req.parent_id != null ? (
                          <Link
                            to={`/p/${projectId}/requirements/${req.parent_id}/edit`}
                            className="text-stitch-accent hover:underline"
                          >
                            {reqTitleById.get(req.parent_id) ?? `REQ #${req.parent_id}`}
                          </Link>
                        ) : (
                          <span className="text-stitch-muted">—</span>
                        )}
                      </div>
                      <div className="sm:col-span-2">
                        <p className="text-[10px] uppercase text-stitch-muted font-bold tracking-wider mb-1">Verification</p>
                        {editCell?.reqId === req.id &&
                        editCell.kind === 'verification_methods' &&
                        canEdit &&
                        !busy ? (
                          <div ref={inlineEditRef} className="space-y-2">
                            <select
                              multiple
                              size={Math.min(8, Math.max(3, verificationMethods.length || 3))}
                              className={`${cellInput} min-h-[72px]`}
                              value={methodIds.map(String)}
                              aria-label="Verification methods"
                              onChange={(e) => {
                                const sel = [...e.target.selectedOptions].map((o) => Number(o.value));
                                setRequirements((prev) =>
                                  prev.map((r) =>
                                    r.id === req.id ? { ...r, verification_method_ids: sel } : r,
                                  ),
                                );
                                void saveReq(req.id, { verification_method_ids: sel });
                              }}
                            >
                              {verificationMethods.map((m) => (
                                <option key={m.id} value={m.id} className="bg-stitch-surface">
                                  {m.title}
                                </option>
                              ))}
                            </select>
                            <button
                              type="button"
                              className="text-[10px] font-bold uppercase tracking-wider text-stitch-accent hover:underline"
                              onClick={closeCellEdit}
                            >
                              Done
                            </button>
                          </div>
                        ) : canEdit && !busy ? (
                          <button
                            type="button"
                            className={displayCellBtn}
                            onClick={() => setEditCell({ reqId: req.id, kind: 'verification_methods' })}
                          >
                            {verificationMethodsText(methodIds, verificationMethods)}
                          </button>
                        ) : (
                          <span className="text-white/90">
                            {verificationMethodsText(methodIds, verificationMethods)}
                          </span>
                        )}
                      </div>
                      <div>
                        <p className="text-[10px] uppercase text-stitch-muted font-bold tracking-wider mb-1">Modified</p>
                        <span className="font-mono text-stitch-muted">{formatModified(req.update_date)}</span>
                      </div>
                      <div>
                        <p className="text-[10px] uppercase text-stitch-muted font-bold tracking-wider mb-1">Author</p>
                        {editCell?.reqId === req.id && editCell.kind === 'author' && canEdit && !busy ? (
                          <div ref={inlineEditRef}>
                            <select
                              className={cellSelect}
                              value={req.author_id}
                              onChange={(e) => {
                                const v = Number(e.target.value);
                                setRequirements((prev) =>
                                  prev.map((r) => (r.id === req.id ? { ...r, author_id: v } : r)),
                                );
                                void saveReq(req.id, { author_id: v });
                                closeCellEdit();
                              }}
                            >
                              {memberUserIds.map((id) => (
                                <option key={id} value={id} className="bg-stitch-surface">
                                  {userLabel(id)}
                                </option>
                              ))}
                              {!memberUserIds.includes(req.author_id) && (
                                <option value={req.author_id} className="bg-stitch-surface">
                                  {userLabel(req.author_id)}
                                </option>
                              )}
                            </select>
                          </div>
                        ) : canEdit && !busy ? (
                          <button type="button" className={displayCellBtn} onClick={() => setEditCell({ reqId: req.id, kind: 'author' })}>
                            {userLabel(req.author_id)}
                          </button>
                        ) : (
                          <span className="text-white/90">{userLabel(req.author_id)}</span>
                        )}
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-1 shrink-0 border-t sm:border-t-0 sm:border-l border-stitch-border/40 pt-3 sm:pt-0 sm:pl-3">
                    {projectSlug ? (
                      <a
                        href={`/p/${projectSlug}/requirements/show/${req.id}`}
                        className="p-1.5 text-stitch-muted hover:text-stitch-accent"
                        title="View"
                      >
                        <span className="material-symbols-outlined text-lg">visibility</span>
                      </a>
                    ) : null}
                    <Link
                      to={`/p/${projectId}/requirements/${req.id}/edit`}
                      className="p-1.5 text-stitch-muted hover:text-stitch-accent"
                      title="Edit"
                    >
                      <span className="material-symbols-outlined text-lg">edit</span>
                    </Link>
                  </div>
                </div>
              </li>
            );
          })}
        </ul>
      ) : (
        <div className="bg-stitch-surface overflow-x-auto rounded-xl border border-stitch-border shadow-stitch">
          <table className="w-full text-left border-collapse min-w-[960px]">
            <thead>
              <tr className="border-b border-stitch-border bg-stitch-elevated">
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider whitespace-nowrap sticky left-0 z-10 bg-stitch-elevated border-r border-stitch-border/60">
                  Key
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[140px]">
                  Title
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[120px]">
                  Category
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[120px]">
                  Parent
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[110px]">
                  Status
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider">
                  Approval
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[140px]">
                  Verification
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider whitespace-nowrap">
                  Modified
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[120px]">
                  Author
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider sticky right-0 bg-stitch-elevated min-w-[88px]">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-stitch-border">
              {pageRows.map((req) => {
                const methodIds = req.verification_method_ids ?? [];
                const busy = savingId === req.id;
                return (
                  <tr key={req.id} className="hover:bg-white/[0.03] transition-colors">
                    <td className="px-2 py-2 align-top sticky left-0 z-[1] bg-stitch-surface border-r border-stitch-border/60">
                      <span className="text-xs font-mono text-stitch-accent font-semibold whitespace-nowrap">
                        {req.reference_code || `REQ-${req.id}`}
                      </span>
                    </td>
                    <td className="px-2 py-2 align-top max-w-[min(280px,26vw)]">
                      {editCell?.reqId === req.id && editCell.kind === 'title' && canEdit && !busy ? (
                        <div ref={inlineEditRef}>
                          <input
                            className={cellInput}
                            value={req.title}
                            onChange={(e) => {
                              const v = e.target.value;
                              setRequirements((prev) =>
                                prev.map((r) => (r.id === req.id ? { ...r, title: v } : r)),
                              );
                            }}
                            onBlur={(e) => {
                              const v = e.target.value.trim();
                              const b = baselineRef.current.get(req.id);
                              if (b && v === b.title) {
                                closeCellEdit();
                                return;
                              }
                              void saveReq(req.id, { title: v });
                              closeCellEdit();
                            }}
                          />
                        </div>
                      ) : canEdit && !busy ? (
                        <button
                          type="button"
                          title="Click to edit title"
                          className={`${displayCellBtn} line-clamp-3`}
                          onClick={() => setEditCell({ reqId: req.id, kind: 'title' })}
                        >
                          {req.title.trim() ? req.title : '—'}
                        </button>
                      ) : (
                        <span className="text-xs text-white/90 px-1.5 py-1 block line-clamp-3">
                          {req.title.trim() ? req.title : '—'}
                        </span>
                      )}
                    </td>
                    <td className="px-2 py-2 align-top">
                      {editCell?.reqId === req.id && editCell.kind === 'category' && canEdit && !busy ? (
                        <div ref={inlineEditRef}>
                          <select
                            className={cellSelect}
                            value={req.category_id}
                            onChange={(e) => {
                              const v = Number(e.target.value);
                              setRequirements((prev) =>
                                prev.map((r) => (r.id === req.id ? { ...r, category_id: v } : r)),
                              );
                              void saveReq(req.id, { category_id: v });
                              closeCellEdit();
                            }}
                          >
                            {categories.map((c) => (
                              <option key={c.id} value={c.id} className="bg-stitch-surface">
                                {c.title}
                              </option>
                            ))}
                            {!categories.some((c) => c.id === req.category_id) && (
                              <option value={req.category_id} className="bg-stitch-surface">
                                Category #{req.category_id}
                              </option>
                            )}
                          </select>
                        </div>
                      ) : canEdit && !busy ? (
                        <button
                          type="button"
                          title="Click to edit category"
                          className={`${displayCellBtn} line-clamp-2`}
                          onClick={() => setEditCell({ reqId: req.id, kind: 'category' })}
                        >
                          {categoryById.get(req.category_id) ?? `Category #${req.category_id}`}
                        </button>
                      ) : (
                        <span className="text-xs text-white/90 px-1.5 py-1 block line-clamp-2">
                          {categoryById.get(req.category_id) ?? `Category #${req.category_id}`}
                        </span>
                      )}
                    </td>
                    <td className="px-2 py-2 align-top text-xs">
                      {req.parent_id != null ? (
                        <Link
                          to={`/p/${projectId}/requirements/${req.parent_id}/edit`}
                          className="text-stitch-accent hover:underline line-clamp-2"
                        >
                          {reqTitleById.get(req.parent_id) ?? `REQ #${req.parent_id}`}
                        </Link>
                      ) : (
                        <span className="text-stitch-muted">—</span>
                      )}
                    </td>
                    <td className="px-2 py-2 align-top">
                      {editCell?.reqId === req.id && editCell.kind === 'status' && canEdit && !busy ? (
                        <div ref={inlineEditRef}>
                          <select
                            className={cellSelect}
                            value={req.status_id}
                            onChange={(e) => {
                              const v = Number(e.target.value);
                              setRequirements((prev) =>
                                prev.map((r) => (r.id === req.id ? { ...r, status_id: v } : r)),
                              );
                              void saveReq(req.id, { status_id: v });
                              closeCellEdit();
                            }}
                          >
                            {statusOptions.map((s) => (
                              <option key={s.id} value={s.id} className="bg-stitch-surface">
                                {s.title}
                              </option>
                            ))}
                            {!statusOptions.some((s) => s.id === req.status_id) && (
                              <option value={req.status_id} className="bg-stitch-surface">
                                Status #{req.status_id}
                              </option>
                            )}
                          </select>
                        </div>
                      ) : canEdit && !busy ? (
                        <button
                          type="button"
                          title="Click to edit status"
                          className={`${displayCellBtn} line-clamp-2 inline-flex items-center`}
                          onClick={() => setEditCell({ reqId: req.id, kind: 'status' })}
                        >
                          <StatusBadge
                            title={statusById.get(req.status_id)?.title ?? `Status #${req.status_id}`}
                            tagColor={statusById.get(req.status_id)?.tag_color}
                          />
                        </button>
                      ) : (
                        <span className="text-xs px-1.5 py-1 block line-clamp-2 inline-flex">
                          <StatusBadge
                            title={statusById.get(req.status_id)?.title ?? `Status #${req.status_id}`}
                            tagColor={statusById.get(req.status_id)?.tag_color}
                          />
                        </span>
                      )}
                    </td>
                    <td className="px-2 py-2 align-top">
                      <span className="text-[10px] font-bold uppercase text-stitch-muted border border-stitch-border rounded px-1.5 py-1 inline-block max-w-[120px] truncate">
                        {approvalLabel(req.approval_state)}
                      </span>
                    </td>
                    <td className="px-2 py-2 align-top min-w-[140px] max-w-[min(320px,28vw)]">
                      {editCell?.reqId === req.id &&
                      editCell.kind === 'verification_methods' &&
                      canEdit &&
                      !busy ? (
                        <div ref={inlineEditRef} className="space-y-2">
                          <select
                            multiple
                            size={Math.min(
                              8,
                              Math.max(3, verificationMethods.length || 3),
                            )}
                            className={`${cellInput} min-h-[72px]`}
                            value={methodIds.map(String)}
                            aria-label="Verification methods"
                            onChange={(e) => {
                              const sel = [...e.target.selectedOptions].map((o) =>
                                Number(o.value),
                              );
                              setRequirements((prev) =>
                                prev.map((r) =>
                                  r.id === req.id ? { ...r, verification_method_ids: sel } : r,
                                ),
                              );
                              void saveReq(req.id, { verification_method_ids: sel });
                            }}
                          >
                            {verificationMethods.map((m) => (
                              <option key={m.id} value={m.id} className="bg-stitch-surface">
                                {m.title}
                              </option>
                            ))}
                          </select>
                          <p className="text-[10px] text-stitch-muted leading-snug">
                            Ctrl/Cmd+click to select several. Click outside or press Esc to close.
                          </p>
                          <button
                            type="button"
                            className="text-[10px] font-bold uppercase tracking-wider text-stitch-accent hover:underline"
                            onClick={closeCellEdit}
                          >
                            Done
                          </button>
                        </div>
                      ) : canEdit && !busy ? (
                        <button
                          type="button"
                          title="Click to change verification methods"
                          className={displayCellBtn}
                          onClick={() =>
                            setEditCell({ reqId: req.id, kind: 'verification_methods' })
                          }
                        >
                          {verificationMethodsText(methodIds, verificationMethods)}
                        </button>
                      ) : (
                        <span className="text-xs text-white/90 leading-snug block px-1.5 py-1">
                          {verificationMethodsText(methodIds, verificationMethods)}
                        </span>
                      )}
                    </td>
                    <td className="px-2 py-2 align-top text-[11px] text-stitch-muted font-mono whitespace-nowrap">
                      {formatModified(req.update_date)}
                    </td>
                    <td className="px-2 py-2 align-top">
                      {editCell?.reqId === req.id && editCell.kind === 'author' && canEdit && !busy ? (
                        <div ref={inlineEditRef}>
                          <select
                            className={cellSelect}
                            value={req.author_id}
                            onChange={(e) => {
                              const v = Number(e.target.value);
                              setRequirements((prev) =>
                                prev.map((r) => (r.id === req.id ? { ...r, author_id: v } : r)),
                              );
                              void saveReq(req.id, { author_id: v });
                              closeCellEdit();
                            }}
                          >
                            {memberUserIds.map((id) => (
                              <option key={id} value={id} className="bg-stitch-surface">
                                {userLabel(id)}
                              </option>
                            ))}
                            {!memberUserIds.includes(req.author_id) && (
                              <option value={req.author_id} className="bg-stitch-surface">
                                {userLabel(req.author_id)}
                              </option>
                            )}
                          </select>
                        </div>
                      ) : canEdit && !busy ? (
                        <button
                          type="button"
                          title="Click to edit author"
                          className={`${displayCellBtn} line-clamp-2`}
                          onClick={() => setEditCell({ reqId: req.id, kind: 'author' })}
                        >
                          {userLabel(req.author_id)}
                        </button>
                      ) : (
                        <span className="text-xs text-white/90 px-1.5 py-1 block line-clamp-2">
                          {userLabel(req.author_id)}
                        </span>
                      )}
                    </td>
                    <td className="px-2 py-2 align-top sticky right-0 z-[1] bg-stitch-surface border-l border-stitch-border/60">
                      <div className="flex items-center gap-1">
                        {projectSlug ? (
                          <a
                            href={`/p/${projectSlug}/requirements/show/${req.id}`}
                            className="p-1.5 text-stitch-muted hover:text-stitch-accent"
                            title="View"
                          >
                            <span className="material-symbols-outlined text-lg">visibility</span>
                          </a>
                        ) : null}
                        <Link
                          to={`/p/${projectId}/requirements/${req.id}/edit`}
                          className="p-1.5 text-stitch-muted hover:text-stitch-accent"
                          title="Edit"
                        >
                          <span className="material-symbols-outlined text-lg">edit</span>
                        </Link>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          {filtered.length === 0 && (
            <p className="p-6 text-center text-stitch-muted text-sm">No requirements match filters.</p>
          )}
        </div>
      )}

      {filtered.length > 0 && (
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div className="flex items-center gap-2 text-xs text-stitch-muted">
            <span>Show rows:</span>
            <select
              value={pageSize}
              onChange={(e) => setPageSize(Number(e.target.value))}
              className="bg-stitch-elevated border border-stitch-border rounded text-xs py-1 px-2 text-white focus:ring-1 focus:ring-stitch-accent outline-none"
            >
              <option value={25}>25</option>
              <option value={50}>50</option>
              <option value={100}>100</option>
            </select>
          </div>
          <div className="flex flex-wrap items-center gap-4">
            <p className="text-xs text-stitch-muted font-medium">
              Showing{' '}
              <span className="text-white font-bold">
                {filtered.length === 0 ? 0 : sliceStart + 1}-{Math.min(sliceStart + pageSize, filtered.length)}
              </span>{' '}
              of <span className="text-white font-bold">{filtered.length}</span>
            </p>
            <div className="flex items-center gap-1">
              <button
                type="button"
                disabled={safePage <= 1}
                onClick={() => setPage((p) => Math.max(1, p - 1))}
                className="p-1 text-stitch-muted hover:text-stitch-accent transition-colors disabled:opacity-30"
              >
                <span className="material-symbols-outlined">chevron_left</span>
              </button>
              <div className="flex gap-1 items-center">
                {paginationItems(safePage, pageCount).map((item, idx) =>
                  item === 'dots' ? (
                    <span key={`dots-${idx}`} className="px-1 text-stitch-muted text-xs">
                      …
                    </span>
                  ) : (
                    <button
                      key={item}
                      type="button"
                      onClick={() => setPage(item)}
                      className={`w-8 h-8 flex items-center justify-center rounded text-xs font-bold transition-colors ${
                        item === safePage
                          ? 'bg-stitch-accent text-stitch-canvas'
                          : 'hover:bg-white/[0.08] text-white'
                      }`}
                    >
                      {item}
                    </button>
                  ),
                )}
              </div>
              <button
                type="button"
                disabled={safePage >= pageCount}
                onClick={() => setPage((p) => Math.min(pageCount, p + 1))}
                className="p-1 text-stitch-muted hover:text-stitch-accent transition-colors disabled:opacity-30"
              >
                <span className="material-symbols-outlined">chevron_right</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

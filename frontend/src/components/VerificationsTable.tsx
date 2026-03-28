import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { useDashboard } from '@/context/DashboardContext';
import {
  getMyPermissions,
  listUsersOptional,
  listVerificationMethodsByProject,
  listVerificationStatuses,
  listVerifications,
  updateVerificationField,
} from '@/api/client';
import type {
  EffectivePermissions,
  User,
  Verification,
  VerificationMethod,
  VerificationStatus,
} from '@/api/types';
import { StatusBadge } from '@/components/StatusBadge';

function escapeCsv(s: string): string {
  if (/[",\n]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
  return s;
}

function paginationItems(current: number, total: number): (number | 'dots')[] {
  if (total <= 7) return Array.from({ length: total }, (_, i) => i + 1);
  if (current <= 4) return [1, 2, 3, 4, 5, 'dots', total];
  if (current >= total - 3) {
    return [1, 'dots', total - 4, total - 3, total - 2, total - 1, total];
  }
  return [1, 'dots', current - 1, current, current + 1, 'dots', total];
}

type ViewMode = 'table' | 'list';

type VerificationTableEditCell = {
  verId: number;
  kind: 'reference_code' | 'name' | 'status' | 'verification_method' | 'source';
};

export default function VerificationsTable({
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

  const [rows, setRows] = useState<Verification[]>([]);
  const [statuses, setStatuses] = useState<VerificationStatus[]>([]);
  const [methods, setMethods] = useState<VerificationMethod[]>([]);
  const [perms, setPerms] = useState<EffectivePermissions | null>(null);
  const [users, setUsers] = useState<User[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [saveErr, setSaveErr] = useState<string | null>(null);
  const [savingId, setSavingId] = useState<number | null>(null);
  const [editCell, setEditCell] = useState<VerificationTableEditCell | null>(null);
  const inlineEditRef = useRef<HTMLDivElement | null>(null);

  const [statusFilter, setStatusFilter] = useState<'all' | number>('all');
  const [pageSize, setPageSize] = useState(25);
  const [page, setPage] = useState(1);

  const load = useCallback(async () => {
    if (!Number.isFinite(projectId)) return;
    setLoading(true);
    setErr(null);
    try {
      const [ver, st, m, p, u] = await Promise.all([
        listVerifications(),
        listVerificationStatuses(),
        listVerificationMethodsByProject(projectId),
        getMyPermissions(projectId).catch(() => null),
        listUsersOptional(),
      ]);
      setRows(ver.filter((v) => v.project_id === projectId));
      setStatuses(st);
      setMethods(m);
      setPerms(p);
      setUsers(u);
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load');
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  const statusOptions = useMemo(() => {
    const forProject = statuses.filter((s) => s.project_id === projectId);
    return forProject.length > 0 ? forProject : statuses;
  }, [statuses, projectId]);

  const statusById = useMemo(() => {
    const m = new Map<number, VerificationStatus>();
    for (const s of statuses) m.set(s.id, s);
    return m;
  }, [statuses]);

  const q = globalSearch.trim().toLowerCase();

  const filtered = useMemo(() => {
    return rows.filter((v) => {
      if (statusFilter !== 'all' && v.status_id !== statusFilter) return false;
      if (!q) return true;
      const parentRow = v.parent_id != null ? rows.find((x) => x.id === v.parent_id) : null;
      const parentBlob = parentRow
        ? [parentRow.reference_code, parentRow.name].join(' ')
        : '';
      const blob = [v.reference_code, v.name, v.description, v.source, String(v.id), parentBlob]
        .join(' ')
        .toLowerCase();
      return blob.includes(q);
    });
  }, [rows, statusFilter, q]);

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
  }, [safePage, pageSize, viewMode]);

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
      inlineEditRef.current
        ?.querySelector<HTMLElement>('input:not([type="checkbox"]), select, textarea')
        ?.focus();
    });
    return () => cancelAnimationFrame(id);
  }, [editCell]);

  const closeCellEdit = () => setEditCell(null);

  const userLabel = useCallback(
    (uid: number) => {
      const u = users?.find((x) => x.id === uid);
      if (u) return `${u.name} (${u.username})`;
      return `User #${uid}`;
    },
    [users],
  );

  const saveField = useCallback(
    async (id: number, field: string, value: string) => {
      const token = csrfToken ?? '';
      if (!token || !perms?.edit_requirements) return;
      if (field === 'status_id' && !perms?.is_project_reviewer) return;
      setSaveErr(null);
      setEditCell((prev) => (prev?.verId === id ? null : prev));
      setSavingId(id);
      try {
        await updateVerificationField(projectId, id, field, value, token);
      } catch (e) {
        setSaveErr(e instanceof Error ? e.message : 'Save failed');
        await load();
      } finally {
        setSavingId(null);
      }
    },
    [csrfToken, perms?.edit_requirements, perms?.is_project_reviewer, projectId, load],
  );

  const resetFilters = useCallback(() => {
    setStatusFilter('all');
  }, []);

  const exportCsv = useCallback(() => {
    const headers = [
      'Key',
      'Title',
      'Parent key',
      'Status',
      'Author',
      'Reviewer',
      'Verification',
      'Source',
    ];
    const lines = [headers.join(',')];
    for (const v of filtered) {
      const stRow = statusById.get(v.status_id);
      const methodTitle =
        v.verification_method_id == null
          ? ''
          : methods.find((m) => m.id === v.verification_method_id)?.title ?? '';
      const parentRow = v.parent_id != null ? rows.find((x) => x.id === v.parent_id) : null;
      const parentKey =
        parentRow != null
          ? (parentRow.reference_code ?? '').trim() || `#${parentRow.id}`
          : v.parent_id != null
            ? `Parent #${v.parent_id}`
            : '';
      lines.push(
        [
          escapeCsv(v.reference_code || `#${v.id}`),
          escapeCsv(v.name),
          escapeCsv(parentKey),
          escapeCsv(stRow?.title ?? ''),
          escapeCsv(userLabel(v.author_id)),
          escapeCsv(userLabel(v.reviewer_id)),
          escapeCsv(methodTitle),
          escapeCsv(v.source),
        ].join(','),
      );
    }
    const blob = new Blob([lines.join('\n')], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `verifications-project-${projectId}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }, [filtered, statusById, methods, rows, projectId, userLabel]);

  const canEditFields = Boolean(perms?.edit_requirements && (csrfToken ?? '').length);
  const canEditVerificationStatus = Boolean(
    perms?.edit_requirements && perms?.is_project_reviewer && (csrfToken ?? '').length,
  );

  const cellInput =
    'w-full min-w-[90px] max-w-[min(100%,280px)] text-xs bg-stitch-elevated border border-stitch-border rounded px-2 py-1.5 text-stitch-fg focus:border-stitch-accent outline-none disabled:opacity-50';
  const cellSelect = `${cellInput} cursor-pointer`;
  const displayCellBtn =
    'w-full text-left text-xs text-stitch-fg/90 leading-snug rounded-md px-1.5 py-1 hover:bg-stitch-higher border border-transparent hover:border-stitch-border/40 transition-colors min-h-[1.75rem]';

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

  function renderParentCell(v: Verification, allRows: Verification[]) {
    const parentRow = v.parent_id != null ? allRows.find((x) => x.id === v.parent_id) : null;
    const parentKeyLabel =
      parentRow != null
        ? (parentRow.reference_code ?? '').trim() || `#${parentRow.id}`
        : v.parent_id != null
          ? `Parent #${v.parent_id}`
          : '—';
    const parentTitle = parentRow?.name?.trim() || undefined;

    if (v.parent_id == null) {
      return <span className="text-stitch-muted text-xs px-0.5">—</span>;
    }
    if (parentRow) {
      return (
        <Link
          to={`/p/${projectId}/verifications/${v.parent_id}`}
          className="text-stitch-accent hover:underline font-mono text-xs line-clamp-2 min-w-0"
          title={parentTitle}
        >
          {parentKeyLabel}
        </Link>
      );
    }
    return (
      <span className="text-stitch-muted font-mono text-xs line-clamp-2" title={parentTitle}>
        {parentKeyLabel}
      </span>
    );
  }

  return (
    <div className="space-y-6">
      {saveErr ? (
        <div className="rounded-lg border border-red-500/30 bg-red-500/10 text-red-100 text-sm px-4 py-2">
          {saveErr}
        </div>
      ) : null}

      <div className="bg-stitch-elevated p-4 rounded-xl border border-stitch-border flex flex-wrap items-center justify-between gap-4">
        <div className="flex flex-wrap items-center gap-2 md:gap-3">
          <div className="flex items-center gap-2 px-3 py-1.5 bg-stitch-surface border border-stitch-border rounded text-xs text-stitch-muted">
            <span className="material-symbols-outlined text-sm">filter_list</span>
            <span>Status:</span>
            <select
              value={statusFilter === 'all' ? 'all' : String(statusFilter)}
              onChange={(e) => {
                const val = e.target.value;
                setStatusFilter(val === 'all' ? 'all' : Number(val));
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
        <div className="flex flex-wrap items-center gap-4">
          <p className="text-[10px] text-stitch-muted font-mono">
            {filtered.length.toLocaleString()} Verifications found
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
              href={`/p/${projectSlug}/verifications.xls`}
              title="Download Excel (classic)"
              className="p-2 text-stitch-muted hover:text-stitch-accent transition-colors"
            >
              <span className="material-symbols-outlined">table_chart</span>
            </a>
          ) : null}
        </div>
      </div>

      {viewMode === 'list' ? (
        filtered.length === 0 ? (
          <div className="rounded-xl border border-stitch-border bg-stitch-surface p-8 text-center text-stitch-muted text-sm">
            No verifications match filters.
            <Link
              to={`/p/${projectId}/verifications/new`}
              className="block mt-3 text-stitch-accent font-semibold hover:underline"
            >
              Create one
            </Link>
          </div>
        ) : (
        <ul className="space-y-3">
          {pageRows.map((v) => {
            const busy = savingId === v.id;
            const stRow = statusById.get(v.status_id);
            const statusTitle = stRow?.title ?? `Status #${v.status_id}`;
            const methodTitle =
              v.verification_method_id == null
                ? '—'
                : methods.find((m) => m.id === v.verification_method_id)?.title ??
                  `ID ${v.verification_method_id}`;
            return (
              <li
                key={v.id}
                className="rounded-xl border border-stitch-border bg-stitch-surface p-4 shadow-stitch hover:bg-white/[0.03] transition-colors"
              >
                <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                  <div className="min-w-0 flex-1 space-y-3">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-mono text-sm text-stitch-accent font-semibold">
                        {v.reference_code || `#${v.id}`}
                      </span>
                      {editCell?.verId === v.id && editCell.kind === 'status' && canEditVerificationStatus && !busy ? (
                        <div ref={inlineEditRef} className="min-w-[160px]">
                          <select
                            className={cellSelect}
                            value={v.status_id}
                            onChange={(e) => {
                              const n = Number(e.target.value);
                              setRows((prev) =>
                                prev.map((r) => (r.id === v.id ? { ...r, status_id: n } : r)),
                              );
                              void saveField(v.id, 'status_id', String(n));
                              closeCellEdit();
                            }}
                          >
                            {statusOptions.map((s) => (
                              <option key={s.id} value={s.id} className="bg-stitch-surface">
                                {s.title}
                              </option>
                            ))}
                            {!statusOptions.some((s) => s.id === v.status_id) && (
                              <option value={v.status_id} className="bg-stitch-surface">
                                Status #{v.status_id}
                              </option>
                            )}
                          </select>
                        </div>
                      ) : canEditVerificationStatus && !busy ? (
                        <button
                          type="button"
                          title="Click to edit status"
                          className="inline-flex"
                          onClick={() => setEditCell({ verId: v.id, kind: 'status' })}
                        >
                          <StatusBadge title={statusTitle} tagColor={stRow?.tag_color} />
                        </button>
                      ) : (
                        <StatusBadge title={statusTitle} tagColor={stRow?.tag_color} />
                      )}
                    </div>
                    <div>
                      <p className="text-[10px] uppercase text-stitch-muted font-bold tracking-wider mb-1">Title</p>
                      {editCell?.verId === v.id && editCell.kind === 'name' && canEditFields && !busy ? (
                        <div ref={inlineEditRef}>
                          <input
                            className={cellInput}
                            value={v.name}
                            onChange={(e) => {
                              const val = e.target.value;
                              setRows((prev) =>
                                prev.map((r) => (r.id === v.id ? { ...r, name: val } : r)),
                              );
                            }}
                            onBlur={(e) => {
                              void saveField(v.id, 'name', e.target.value.trim());
                              closeCellEdit();
                            }}
                          />
                        </div>
                      ) : canEditFields && !busy ? (
                        <button
                          type="button"
                          title="Click to edit title"
                          className={`${displayCellBtn} text-left`}
                          onClick={() => setEditCell({ verId: v.id, kind: 'name' })}
                        >
                          {v.name.trim() ? v.name : '—'}
                        </button>
                      ) : (
                        <p className="text-sm text-stitch-fg/90">{v.name.trim() ? v.name : '—'}</p>
                      )}
                    </div>
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs">
                      <div>
                        <p className="text-[10px] uppercase text-stitch-muted font-bold tracking-wider mb-1">
                          Category
                        </p>
                        <span className="text-stitch-muted">—</span>
                      </div>
                      <div>
                        <p className="text-[10px] uppercase text-stitch-muted font-bold tracking-wider mb-1">
                          Parents
                        </p>
                        <div>{renderParentCell(v, rows)}</div>
                      </div>
                      <div className="sm:col-span-2">
                        <p className="text-[10px] uppercase text-stitch-muted font-bold tracking-wider mb-1">
                          Verification
                        </p>
                        {editCell?.verId === v.id &&
                        editCell.kind === 'verification_method' &&
                        canEditFields &&
                        !busy ? (
                          <div ref={inlineEditRef}>
                            <select
                              className={cellSelect}
                              value={v.verification_method_id ?? ''}
                              onChange={(e) => {
                                const raw = e.target.value;
                                const n = raw === '' ? null : Number(raw);
                                setRows((prev) =>
                                  prev.map((r) =>
                                    r.id === v.id ? { ...r, verification_method_id: n } : r,
                                  ),
                                );
                                void saveField(v.id, 'verification_method_id', raw === '' ? '' : raw);
                                closeCellEdit();
                              }}
                            >
                              <option value="" className="bg-stitch-surface">
                                —
                              </option>
                              {methods.map((m) => (
                                <option key={m.id} value={m.id} className="bg-stitch-surface">
                                  {m.title}
                                </option>
                              ))}
                            </select>
                          </div>
                        ) : canEditFields && !busy ? (
                          <button
                            type="button"
                            className={displayCellBtn}
                            onClick={() => setEditCell({ verId: v.id, kind: 'verification_method' })}
                          >
                            {methodTitle}
                          </button>
                        ) : (
                          <span className="text-stitch-fg/90">{methodTitle}</span>
                        )}
                      </div>
                      <div>
                        <p className="text-[10px] uppercase text-stitch-muted font-bold tracking-wider mb-1">
                          Source
                        </p>
                        {editCell?.verId === v.id && editCell.kind === 'source' && canEditFields && !busy ? (
                          <div ref={inlineEditRef}>
                            <input
                              className={cellInput}
                              value={v.source}
                              onChange={(e) => {
                                const val = e.target.value;
                                setRows((prev) =>
                                  prev.map((r) => (r.id === v.id ? { ...r, source: val } : r)),
                                );
                              }}
                              onBlur={(e) => {
                                void saveField(v.id, 'source', e.target.value.trim());
                                closeCellEdit();
                              }}
                            />
                          </div>
                        ) : canEditFields && !busy ? (
                          <button type="button" className={displayCellBtn} onClick={() => setEditCell({ verId: v.id, kind: 'source' })}>
                            {v.source.trim() ? v.source : '—'}
                          </button>
                        ) : (
                          <span className="text-stitch-fg/90">{v.source.trim() ? v.source : '—'}</span>
                        )}
                      </div>
                      <div>
                        <p className="text-[10px] uppercase text-stitch-muted font-bold tracking-wider mb-1">
                          Author
                        </p>
                        <span className="text-stitch-fg/90">{userLabel(v.author_id)}</span>
                      </div>
                      <div>
                        <p className="text-[10px] uppercase text-stitch-muted font-bold tracking-wider mb-1">
                          Reviewer
                        </p>
                        <span className="text-stitch-fg/90">{userLabel(v.reviewer_id)}</span>
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-1 shrink-0 border-t sm:border-t-0 sm:border-l border-stitch-border/40 pt-3 sm:pt-0 sm:pl-3">
                    <Link
                      to={`/p/${projectId}/verifications/${v.id}`}
                      className="p-1.5 text-stitch-muted hover:text-stitch-accent"
                      title="View"
                    >
                      <span className="material-symbols-outlined text-lg">visibility</span>
                    </Link>
                    <Link
                      to={`/p/${projectId}/verifications/${v.id}/edit`}
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
        )
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
                  Parents
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[110px]">
                  Status
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[100px]">
                  Author
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[100px]">
                  Reviewer
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[140px]">
                  Verification
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[120px]">
                  Source
                </th>
                <th className="px-2 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider sticky right-0 bg-stitch-elevated min-w-[88px]">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-stitch-border">
              {pageRows.map((v) => {
                const busy = savingId === v.id;
                const stRow = statusById.get(v.status_id);
                const statusTitle = stRow?.title ?? `Status #${v.status_id}`;
                const methodTitle =
                  v.verification_method_id == null
                    ? '—'
                    : methods.find((m) => m.id === v.verification_method_id)?.title ??
                      `ID ${v.verification_method_id}`;
                return (
                  <tr key={v.id} className="hover:bg-white/[0.03] transition-colors">
                    <td className="px-2 py-2 align-top sticky left-0 z-[1] bg-stitch-surface border-r border-stitch-border/60 max-w-[min(140px,18vw)]">
                      {editCell?.verId === v.id &&
                      editCell.kind === 'reference_code' &&
                      canEditFields &&
                      !busy ? (
                        <div ref={inlineEditRef}>
                          <input
                            className={cellInput}
                            value={v.reference_code}
                            onChange={(e) => {
                              const val = e.target.value;
                              setRows((prev) =>
                                prev.map((r) => (r.id === v.id ? { ...r, reference_code: val } : r)),
                              );
                            }}
                            onBlur={(e) => {
                              void saveField(v.id, 'reference_code', e.target.value.trim());
                              closeCellEdit();
                            }}
                          />
                        </div>
                      ) : canEditFields && !busy ? (
                        <button
                          type="button"
                          title="Click to edit reference"
                          className={`${displayCellBtn} font-mono text-stitch-accent font-semibold`}
                          onClick={() => setEditCell({ verId: v.id, kind: 'reference_code' })}
                        >
                          {v.reference_code || `#${v.id}`}
                        </button>
                      ) : (
                        <span className="text-xs font-mono text-stitch-accent font-semibold whitespace-nowrap px-1.5 py-1 block">
                          {v.reference_code || `#${v.id}`}
                        </span>
                      )}
                    </td>
                    <td className="px-2 py-2 align-top max-w-[min(280px,26vw)]">
                      {editCell?.verId === v.id && editCell.kind === 'name' && canEditFields && !busy ? (
                        <div ref={inlineEditRef}>
                          <input
                            className={cellInput}
                            value={v.name}
                            onChange={(e) => {
                              const val = e.target.value;
                              setRows((prev) =>
                                prev.map((r) => (r.id === v.id ? { ...r, name: val } : r)),
                              );
                            }}
                            onBlur={(e) => {
                              void saveField(v.id, 'name', e.target.value.trim());
                              closeCellEdit();
                            }}
                          />
                        </div>
                      ) : canEditFields && !busy ? (
                        <button
                          type="button"
                          title="Click to edit title"
                          className={`${displayCellBtn} line-clamp-3`}
                          onClick={() => setEditCell({ verId: v.id, kind: 'name' })}
                        >
                          {v.name.trim() ? v.name : '—'}
                        </button>
                      ) : (
                        <span className="text-xs text-stitch-fg/90 px-1.5 py-1 block line-clamp-3">
                          {v.name.trim() ? v.name : '—'}
                        </span>
                      )}
                    </td>
                    <td className="px-2 py-2 align-top min-w-[120px] max-w-[200px]">
                      {renderParentCell(v, rows)}
                    </td>
                    <td className="px-2 py-2 align-top min-w-[110px]">
                      {editCell?.verId === v.id && editCell.kind === 'status' && canEditVerificationStatus && !busy ? (
                        <div ref={inlineEditRef}>
                          <select
                            className={cellSelect}
                            value={v.status_id}
                            onChange={(e) => {
                              const n = Number(e.target.value);
                              setRows((prev) =>
                                prev.map((r) => (r.id === v.id ? { ...r, status_id: n } : r)),
                              );
                              void saveField(v.id, 'status_id', String(n));
                              closeCellEdit();
                            }}
                          >
                            {statusOptions.map((s) => (
                              <option key={s.id} value={s.id} className="bg-stitch-surface">
                                {s.title}
                              </option>
                            ))}
                            {!statusOptions.some((s) => s.id === v.status_id) && (
                              <option value={v.status_id} className="bg-stitch-surface">
                                Status #{v.status_id}
                              </option>
                            )}
                          </select>
                        </div>
                      ) : canEditVerificationStatus && !busy ? (
                        <button
                          type="button"
                          title="Click to edit status"
                          className={`${displayCellBtn} inline-flex items-center`}
                          onClick={() => setEditCell({ verId: v.id, kind: 'status' })}
                        >
                          <StatusBadge title={statusTitle} tagColor={stRow?.tag_color} />
                        </button>
                      ) : (
                        <span className="text-xs px-1.5 py-1 block inline-flex">
                          <StatusBadge title={statusTitle} tagColor={stRow?.tag_color} />
                        </span>
                      )}
                    </td>
                    <td className="px-2 py-2 align-top text-xs text-stitch-fg/90 max-w-[140px]">
                      <span className="line-clamp-2" title={userLabel(v.author_id)}>
                        {userLabel(v.author_id)}
                      </span>
                    </td>
                    <td className="px-2 py-2 align-top text-xs text-stitch-fg/90 max-w-[140px]">
                      <span className="line-clamp-2" title={userLabel(v.reviewer_id)}>
                        {userLabel(v.reviewer_id)}
                      </span>
                    </td>
                    <td className="px-2 py-2 align-top min-w-[140px]">
                      {editCell?.verId === v.id &&
                      editCell.kind === 'verification_method' &&
                      canEditFields &&
                      !busy ? (
                        <div ref={inlineEditRef}>
                          <select
                            className={cellSelect}
                            value={v.verification_method_id ?? ''}
                            onChange={(e) => {
                              const raw = e.target.value;
                              const n = raw === '' ? null : Number(raw);
                              setRows((prev) =>
                                prev.map((r) =>
                                  r.id === v.id ? { ...r, verification_method_id: n } : r,
                                ),
                              );
                              void saveField(v.id, 'verification_method_id', raw === '' ? '' : raw);
                              closeCellEdit();
                            }}
                          >
                            <option value="" className="bg-stitch-surface">
                              —
                            </option>
                            {methods.map((m) => (
                              <option key={m.id} value={m.id} className="bg-stitch-surface">
                                {m.title}
                              </option>
                            ))}
                          </select>
                        </div>
                      ) : canEditFields && !busy ? (
                        <button
                          type="button"
                          title="Click to edit verification"
                          className={`${displayCellBtn} line-clamp-2`}
                          onClick={() => setEditCell({ verId: v.id, kind: 'verification_method' })}
                        >
                          {methodTitle}
                        </button>
                      ) : (
                        <span className="text-xs text-stitch-fg/90 px-1.5 py-1 block line-clamp-2">
                          {methodTitle}
                        </span>
                      )}
                    </td>
                    <td className="px-2 py-2 align-top max-w-[160px]">
                      {editCell?.verId === v.id && editCell.kind === 'source' && canEditFields && !busy ? (
                        <div ref={inlineEditRef}>
                          <input
                            className={cellInput}
                            value={v.source}
                            onChange={(e) => {
                              const val = e.target.value;
                              setRows((prev) =>
                                prev.map((r) => (r.id === v.id ? { ...r, source: val } : r)),
                              );
                            }}
                            onBlur={(e) => {
                              void saveField(v.id, 'source', e.target.value.trim());
                              closeCellEdit();
                            }}
                          />
                        </div>
                      ) : canEditFields && !busy ? (
                        <button
                          type="button"
                          title="Click to edit source"
                          className={`${displayCellBtn} line-clamp-2`}
                          onClick={() => setEditCell({ verId: v.id, kind: 'source' })}
                        >
                          {v.source.trim() ? v.source : '—'}
                        </button>
                      ) : (
                        <span className="text-xs text-stitch-fg/90 px-1.5 py-1 block line-clamp-2">
                          {v.source.trim() ? v.source : '—'}
                        </span>
                      )}
                    </td>
                    <td className="px-2 py-2 align-top sticky right-0 z-[1] bg-stitch-surface border-l border-stitch-border/60">
                      <div className="flex items-center gap-1">
                        <Link
                          to={`/p/${projectId}/verifications/${v.id}`}
                          className="p-1.5 text-stitch-muted hover:text-stitch-accent"
                          title="View"
                        >
                          <span className="material-symbols-outlined text-lg">visibility</span>
                        </Link>
                        <Link
                          to={`/p/${projectId}/verifications/${v.id}/edit`}
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
            <p className="p-8 text-center text-stitch-muted text-sm">
              No verifications match filters.
              <Link
                to={`/p/${projectId}/verifications/new`}
                className="block mt-3 text-stitch-accent font-semibold hover:underline"
              >
                Create one
              </Link>
            </p>
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
              className="bg-stitch-elevated border border-stitch-border rounded text-xs py-1 px-2 text-stitch-fg focus:ring-1 focus:ring-stitch-accent outline-none"
            >
              <option value={25}>25</option>
              <option value={50}>50</option>
              <option value={100}>100</option>
            </select>
          </div>
          <div className="flex flex-wrap items-center gap-4">
            <p className="text-xs text-stitch-muted font-medium">
              Showing{' '}
              <span className="text-stitch-fg font-bold">
                {filtered.length === 0 ? 0 : sliceStart + 1}-{Math.min(sliceStart + pageSize, filtered.length)}
              </span>{' '}
              of <span className="text-stitch-fg font-bold">{filtered.length}</span>
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
                          : 'hover:bg-stitch-elevated text-stitch-fg'
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

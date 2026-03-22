import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link, useOutletContext, useParams } from 'react-router-dom';
import {
  getMyPermissions,
  listVerificationMethodsByProject,
  listVerificationStatuses,
  listVerifications,
  updateVerificationField,
} from '@/api/client';
import { StatusBadge } from '@/components/StatusBadge';
import { useDashboard } from '@/context/DashboardContext';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import type {
  EffectivePermissions,
  Verification,
  VerificationMethod,
  VerificationStatus,
} from '@/api/types';

type VerificationTableEditCell = {
  verId: number;
  kind:
    | 'reference_code'
    | 'name'
    | 'status'
    | 'verification_method'
    | 'source'
    | 'parent';
};

export default function VerificationsPage() {
  const { globalSearch } = useOutletContext<ProjectOutletContext>();
  const { projectId: projectIdParam } = useParams();
  const pid = Number(projectIdParam);
  const { dashboard, csrfToken } = useDashboard();

  const [rows, setRows] = useState<Verification[]>([]);
  const [statuses, setStatuses] = useState<VerificationStatus[]>([]);
  const [methods, setMethods] = useState<VerificationMethod[]>([]);
  const [perms, setPerms] = useState<EffectivePermissions | null>(null);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [saveErr, setSaveErr] = useState<string | null>(null);
  const [savingId, setSavingId] = useState<number | null>(null);
  const [editCell, setEditCell] = useState<VerificationTableEditCell | null>(null);
  const inlineEditRef = useRef<HTMLDivElement | null>(null);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setLoading(true);
    setErr(null);
    try {
      const [ver, st, m, p] = await Promise.all([
        listVerifications(),
        listVerificationStatuses(),
        listVerificationMethodsByProject(pid),
        getMyPermissions(pid).catch(() => null),
      ]);
      setRows(ver.filter((v) => v.project_id === pid));
      setStatuses(st);
      setMethods(m);
      setPerms(p);
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load');
    } finally {
      setLoading(false);
    }
  }, [pid]);

  useEffect(() => {
    void load();
  }, [load]);

  const statusOptions = useMemo(() => {
    const forProject = statuses.filter((s) => s.project_id === pid);
    return forProject.length > 0 ? forProject : statuses;
  }, [statuses, pid]);

  const q = globalSearch.trim().toLowerCase();
  const filtered = useMemo(() => {
    return rows.filter((v) => {
      if (!q) return true;
      const blob = [v.reference_code, v.name, v.description, v.source, String(v.id)]
        .join(' ')
        .toLowerCase();
      return blob.includes(q);
    });
  }, [rows, q]);

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  const canEdit = Boolean(perms?.edit_requirements && (csrfToken ?? '').length);

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

  const saveField = useCallback(
    async (id: number, field: string, value: string) => {
      const token = csrfToken ?? '';
      if (!token || !perms?.edit_requirements) return;
      setSaveErr(null);
      setEditCell((prev) => (prev?.verId === id ? null : prev));
      setSavingId(id);
      try {
        await updateVerificationField(id, field, value, token);
      } catch (e) {
        setSaveErr(e instanceof Error ? e.message : 'Save failed');
        await load();
      } finally {
        setSavingId(null);
      }
    },
    [csrfToken, perms?.edit_requirements, load],
  );

  const cellInput =
    'w-full min-w-[90px] max-w-[min(100%,280px)] text-xs bg-stitch-elevated border border-stitch-border rounded px-2 py-1.5 text-white focus:border-stitch-accent outline-none disabled:opacity-50';
  const cellSelect = `${cellInput} cursor-pointer`;
  const displayCellBtn =
    'w-full text-left text-xs text-white/90 leading-snug rounded-md px-1.5 py-1 hover:bg-white/[0.06] border border-transparent hover:border-stitch-border/40 transition-colors min-h-[1.75rem]';

  const statusById = useMemo(() => {
    const m = new Map<number, VerificationStatus>();
    for (const s of statuses) m.set(s.id, s);
    return m;
  }, [statuses]);

  if (loading) {
    return (
      <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
        Loading verifications…
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

  return (
    <div>
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between mb-8">
        <div>
          <nav className="flex text-[10px] text-stitch-muted font-mono uppercase tracking-widest mb-1">
            <span>{projectName}</span>
            <span className="mx-2">/</span>
            <span className="text-stitch-accent font-bold">Verifications</span>
          </nav>
          <h2 className="text-2xl md:text-3xl font-extrabold text-white tracking-tight font-headline">
            Verification registry
          </h2>
          <p className="text-stitch-muted text-sm mt-2 max-w-2xl">
            Click any field to edit inline. Escape or click outside closes the editor (needs edit
            permission + CSRF session).
          </p>
        </div>
        <Link
          to={`/p/${pid}/verifications/new`}
          className="inline-flex items-center justify-center gap-2 bg-gradient-to-br from-[#000666] to-[#1a237e] text-white px-5 py-2.5 text-sm font-semibold rounded-md shadow-lg hover:opacity-95 transition-opacity shrink-0"
        >
          <span className="material-symbols-outlined text-sm">add</span>
          New verification
        </Link>
      </div>

      <div className="bg-stitch-elevated p-4 rounded-xl border border-stitch-border flex flex-wrap items-center justify-between gap-3 mb-6">
        <p className="text-[10px] text-stitch-muted font-mono">
          {filtered.length.toLocaleString()} verification{filtered.length === 1 ? '' : 's'} shown
        </p>
        <button
          type="button"
          onClick={() => void load()}
          className="text-xs text-stitch-accent font-bold flex items-center gap-1 hover:underline"
        >
          <span className="material-symbols-outlined text-sm">refresh</span>
          Refresh
        </button>
      </div>

      {saveErr ? (
        <div className="mb-4 rounded-lg border border-red-500/30 bg-red-500/10 text-red-100 text-sm px-4 py-2">
          {saveErr}
        </div>
      ) : null}

      <div className="bg-stitch-surface overflow-x-auto rounded-xl border border-stitch-border shadow-stitch">
        <table className="w-full text-left border-collapse min-w-[900px]">
          <thead>
            <tr className="border-b border-stitch-border bg-stitch-elevated">
              <th className="px-3 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider whitespace-nowrap">
                Reference
              </th>
              <th className="px-3 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[140px]">
                Name
              </th>
              <th className="px-3 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[120px]">
                Status
              </th>
              <th className="px-3 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[140px]">
                Verification type
              </th>
              <th className="px-3 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[120px]">
                Source
              </th>
              <th className="px-3 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[140px]">
                Parent
              </th>
              <th className="px-3 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider sticky right-0 bg-stitch-elevated">
                Actions
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-stitch-border">
            {filtered.map((v) => {
              const busy = savingId === v.id;
              const parentCandidates = rows.filter((x) => x.id !== v.id);
              const stRow = statusById.get(v.status_id);
              const statusTitle = stRow?.title ?? `Status #${v.status_id}`;
              const methodTitle =
                v.verification_method_id == null
                  ? '—'
                  : methods.find((m) => m.id === v.verification_method_id)?.title ??
                    `ID ${v.verification_method_id}`;
              const parentRow = v.parent_id != null ? rows.find((x) => x.id === v.parent_id) : null;
              const parentText = parentRow
                ? `${parentRow.reference_code || `#${parentRow.id}`} — ${parentRow.name}`
                : v.parent_id != null
                  ? `Parent #${v.parent_id}`
                  : '—';
              return (
                <tr key={v.id} className="hover:bg-white/[0.03]">
                  <td className="px-3 py-2 align-top max-w-[140px]">
                    {editCell?.verId === v.id &&
                    editCell.kind === 'reference_code' &&
                    canEdit &&
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
                    ) : canEdit && !busy ? (
                      <button
                        type="button"
                        title="Click to edit reference"
                        className={`${displayCellBtn} font-mono text-stitch-accent font-semibold`}
                        onClick={() => setEditCell({ verId: v.id, kind: 'reference_code' })}
                      >
                        {v.reference_code || `#${v.id}`}
                      </button>
                    ) : (
                      <span className="text-xs font-mono text-stitch-accent font-semibold px-1.5 py-1 block">
                        {v.reference_code || `#${v.id}`}
                      </span>
                    )}
                  </td>
                  <td className="px-3 py-2 align-top max-w-[200px]">
                    {editCell?.verId === v.id && editCell.kind === 'name' && canEdit && !busy ? (
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
                    ) : canEdit && !busy ? (
                      <button
                        type="button"
                        title="Click to edit name"
                        className={`${displayCellBtn} line-clamp-2`}
                        onClick={() => setEditCell({ verId: v.id, kind: 'name' })}
                      >
                        {v.name.trim() ? v.name : '—'}
                      </button>
                    ) : (
                      <span className="text-xs text-white/90 px-1.5 py-1 block line-clamp-2">
                        {v.name.trim() ? v.name : '—'}
                      </span>
                    )}
                  </td>
                  <td className="px-3 py-2 align-top min-w-[120px]">
                    {editCell?.verId === v.id && editCell.kind === 'status' && canEdit && !busy ? (
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
                    ) : canEdit && !busy ? (
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
                  <td className="px-3 py-2 align-top min-w-[140px]">
                    {editCell?.verId === v.id &&
                    editCell.kind === 'verification_method' &&
                    canEdit &&
                    !busy ? (
                      <div ref={inlineEditRef} className="space-y-1">
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
                    ) : canEdit && !busy ? (
                      <button
                        type="button"
                        title="Click to edit verification type"
                        className={`${displayCellBtn} line-clamp-2`}
                        onClick={() => setEditCell({ verId: v.id, kind: 'verification_method' })}
                      >
                        {methodTitle}
                      </button>
                    ) : (
                      <span className="text-xs text-white/90 px-1.5 py-1 block line-clamp-2">
                        {methodTitle}
                      </span>
                    )}
                  </td>
                  <td className="px-3 py-2 align-top max-w-[160px]">
                    {editCell?.verId === v.id && editCell.kind === 'source' && canEdit && !busy ? (
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
                    ) : canEdit && !busy ? (
                      <button
                        type="button"
                        title="Click to edit source"
                        className={`${displayCellBtn} line-clamp-2`}
                        onClick={() => setEditCell({ verId: v.id, kind: 'source' })}
                      >
                        {v.source.trim() ? v.source : '—'}
                      </button>
                    ) : (
                      <span className="text-xs text-white/90 px-1.5 py-1 block line-clamp-2">
                        {v.source.trim() ? v.source : '—'}
                      </span>
                    )}
                  </td>
                  <td className="px-3 py-2 align-top min-w-[160px]">
                    {editCell?.verId === v.id && editCell.kind === 'parent' && canEdit && !busy ? (
                      <div ref={inlineEditRef}>
                        <select
                          className={cellSelect}
                          value={v.parent_id ?? ''}
                          onChange={(e) => {
                            const raw = e.target.value;
                            const n = raw === '' ? null : Number(raw);
                            setRows((prev) =>
                              prev.map((r) => (r.id === v.id ? { ...r, parent_id: n } : r)),
                            );
                            void saveField(v.id, 'parent_id', raw === '' ? '' : raw);
                            closeCellEdit();
                          }}
                        >
                          <option value="" className="bg-stitch-surface">
                            —
                          </option>
                          {parentCandidates.map((p) => (
                            <option key={p.id} value={p.id} className="bg-stitch-surface">
                              {p.reference_code || `#${p.id}`} — {p.name}
                            </option>
                          ))}
                        </select>
                      </div>
                    ) : canEdit && !busy ? (
                      <button
                        type="button"
                        title="Click to edit parent"
                        className={`${displayCellBtn} line-clamp-2`}
                        onClick={() => setEditCell({ verId: v.id, kind: 'parent' })}
                      >
                        {parentText}
                      </button>
                    ) : (
                      <span className="text-xs text-white/90 px-1.5 py-1 block line-clamp-2">
                        {parentText}
                      </span>
                    )}
                  </td>
                  <td className="px-3 py-2 align-top sticky right-0 z-[1] bg-stitch-surface border-l border-stitch-border/60">
                    <div className="flex items-center gap-1">
                      <Link
                        to={`/p/${pid}/verifications/${v.id}`}
                        className="p-1.5 text-stitch-muted hover:text-stitch-accent"
                        title="View"
                      >
                        <span className="material-symbols-outlined text-lg">visibility</span>
                      </Link>
                      <Link
                        to={`/p/${pid}/verifications/${v.id}/edit`}
                        className="p-1.5 text-stitch-muted hover:text-stitch-accent"
                        title="SPA editor"
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
            No verifications in this project yet.
            <Link
              to={`/p/${pid}/verifications/new`}
              className="block mt-3 text-stitch-accent font-semibold hover:underline"
            >
              Create one
            </Link>
          </p>
        )}
      </div>
    </div>
  );
}

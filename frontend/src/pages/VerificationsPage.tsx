import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useOutletContext, useParams } from 'react-router-dom';
import {
  getMyPermissions,
  listVerificationMethodsByProject,
  listVerificationStatuses,
  listVerifications,
  updateVerificationField,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import type {
  EffectivePermissions,
  Verification,
  VerificationMethod,
  VerificationStatus,
} from '@/api/types';

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
  const projectSlug = dashboard?.projects?.find((p) => p.id === pid)?.slug;

  const canEdit = Boolean(perms?.edit_requirements && (csrfToken ?? '').length);

  const saveField = useCallback(
    async (id: number, field: string, value: string) => {
      const token = csrfToken ?? '';
      if (!token || !perms?.edit_requirements) return;
      setSaveErr(null);
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
            Same columns as the classic tests table. Edit cells inline (needs edit permission + CSRF
            session).
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
        <table className="w-full text-left border-collapse min-w-[1100px]">
          <thead>
            <tr className="border-b border-stitch-border bg-stitch-elevated">
              <th className="px-3 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider whitespace-nowrap">
                Reference
              </th>
              <th className="px-3 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[140px]">
                Name
              </th>
              <th className="px-3 py-3 text-[10px] font-bold text-stitch-muted uppercase tracking-wider min-w-[200px]">
                Description
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
              return (
                <tr key={v.id} className="hover:bg-white/[0.03]">
                  <td className="px-3 py-2 align-top">
                    <input
                      className={cellInput}
                      value={v.reference_code}
                      disabled={!canEdit || busy}
                      onChange={(e) => {
                        const val = e.target.value;
                        setRows((prev) =>
                          prev.map((r) => (r.id === v.id ? { ...r, reference_code: val } : r)),
                        );
                      }}
                      onBlur={(e) => void saveField(v.id, 'reference_code', e.target.value.trim())}
                    />
                  </td>
                  <td className="px-3 py-2 align-top">
                    <input
                      className={cellInput}
                      value={v.name}
                      disabled={!canEdit || busy}
                      onChange={(e) => {
                        const val = e.target.value;
                        setRows((prev) =>
                          prev.map((r) => (r.id === v.id ? { ...r, name: val } : r)),
                        );
                      }}
                      onBlur={(e) => void saveField(v.id, 'name', e.target.value.trim())}
                    />
                  </td>
                  <td className="px-3 py-2 align-top">
                    <textarea
                      className={`${cellInput} min-h-[56px] resize-y`}
                      rows={2}
                      value={v.description}
                      disabled={!canEdit || busy}
                      onChange={(e) => {
                        const val = e.target.value;
                        setRows((prev) =>
                          prev.map((r) => (r.id === v.id ? { ...r, description: val } : r)),
                        );
                      }}
                      onBlur={(e) => void saveField(v.id, 'description', e.target.value.trim())}
                    />
                  </td>
                  <td className="px-3 py-2 align-top">
                    <select
                      className={cellSelect}
                      value={v.status_id}
                      disabled={!canEdit || busy}
                      onChange={(e) => {
                        const n = Number(e.target.value);
                        setRows((prev) =>
                          prev.map((r) => (r.id === v.id ? { ...r, status_id: n } : r)),
                        );
                        void saveField(v.id, 'status_id', String(n));
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
                  </td>
                  <td className="px-3 py-2 align-top">
                    <select
                      className={cellSelect}
                      value={v.verification_method_id ?? ''}
                      disabled={!canEdit || busy}
                      onChange={(e) => {
                        const raw = e.target.value;
                        const n = raw === '' ? null : Number(raw);
                        setRows((prev) =>
                          prev.map((r) =>
                            r.id === v.id ? { ...r, verification_method_id: n } : r,
                          ),
                        );
                        void saveField(v.id, 'verification_method_id', raw === '' ? '' : raw);
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
                    {v.verification_method_id != null &&
                    !methods.some((m) => m.id === v.verification_method_id) ? (
                      <p className="text-[10px] text-stitch-muted mt-0.5">
                        ID {v.verification_method_id}
                      </p>
                    ) : null}
                  </td>
                  <td className="px-3 py-2 align-top">
                    <input
                      className={cellInput}
                      value={v.source}
                      disabled={!canEdit || busy}
                      onChange={(e) => {
                        const val = e.target.value;
                        setRows((prev) =>
                          prev.map((r) => (r.id === v.id ? { ...r, source: val } : r)),
                        );
                      }}
                      onBlur={(e) => void saveField(v.id, 'source', e.target.value.trim())}
                    />
                  </td>
                  <td className="px-3 py-2 align-top">
                    <select
                      className={cellSelect}
                      value={v.parent_id ?? ''}
                      disabled={!canEdit || busy}
                      onChange={(e) => {
                        const raw = e.target.value;
                        const n = raw === '' ? null : Number(raw);
                        setRows((prev) =>
                          prev.map((r) => (r.id === v.id ? { ...r, parent_id: n } : r)),
                        );
                        void saveField(v.id, 'parent_id', raw === '' ? '' : raw);
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
                  </td>
                  <td className="px-3 py-2 align-top sticky right-0 z-[1] bg-stitch-surface border-l border-stitch-border/60">
                    <div className="flex items-center gap-1">
                      {projectSlug ? (
                        <a
                          href={`/p/${projectSlug}/verifications/show/${v.id}`}
                          className="p-1.5 text-stitch-muted hover:text-stitch-accent"
                          title="Classic view"
                        >
                          <span className="material-symbols-outlined text-lg">visibility</span>
                        </a>
                      ) : null}
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

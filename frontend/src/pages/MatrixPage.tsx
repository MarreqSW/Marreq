import { useCallback, useEffect, useMemo, useState } from 'react';
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

const PAGE_SIZE = 50;

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

  const q = globalSearch.trim().toLowerCase();
  const filtered = useMemo(() => {
    let rows = matrix;
    if (suspectOnly) rows = rows.filter((m) => m.suspect);
    if (!q) return rows;
    return rows.filter((m) => {
      const r = reqById.get(m.req_id);
      const t = verById.get(m.verification_id);
      const st = t ? statusById.get(t.status_id) : undefined;
      const meth = t?.verification_method_id
        ? methodById.get(t.verification_method_id)
        : undefined;
      const blob = [
        r?.reference_code,
        r?.title,
        t?.reference_code,
        t?.name,
        st?.title,
        st?.tag,
        meth?.title,
        String(m.req_id),
        String(m.verification_id),
      ]
        .join(' ')
        .toLowerCase();
      return blob.includes(q);
    });
  }, [matrix, suspectOnly, q, reqById, verById, statusById, methodById]);

  /** Count matrix rows by linked verification status (test outcome / workflow state). */
  const statusRollup = useMemo(() => {
    const counts = new Map<string, { n: number; tagColor: string | null }>();
    for (const m of filtered) {
      const t = verById.get(m.verification_id);
      const st = t ? statusById.get(t.status_id) : undefined;
      const label = st?.title ?? (t ? `Status #${t.status_id}` : 'Unknown verification');
      const prev = counts.get(label);
      counts.set(label, {
        n: (prev?.n ?? 0) + 1,
        tagColor: st?.tag_color ?? prev?.tagColor ?? null,
      });
    }
    return [...counts.entries()].sort((a, b) => b[1].n - a[1].n);
  }, [filtered, verById, statusById]);

  const pageCount = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const safePage = Math.min(page, pageCount);
  const slice = filtered.slice((safePage - 1) * PAGE_SIZE, safePage * PAGE_SIZE);

  useEffect(() => {
    setPage(1);
  }, [suspectOnly, q]);

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

  return (
    <div>
      <StitchPageHeader
        projectName={projectName}
        section="Matrix"
        title="Traceability matrix"
        subtitle="Each row is one traceability link. Verification status reflects the test record (passed, failed, pending, etc., per your project catalog)."
      >
        <a
          href={`/p/${pid}/matrix`}
          className="text-xs font-bold uppercase tracking-wider text-stitch-accent border border-stitch-border rounded-md px-3 py-2 hover:bg-stitch-higher"
        >
          Classic matrix
        </a>
        <button
          type="button"
          onClick={() => void load()}
          className="text-xs font-bold uppercase tracking-wider text-stitch-muted border border-stitch-border rounded-md px-3 py-2 hover:bg-stitch-higher"
        >
          Refresh
        </button>
      </StitchPageHeader>

      <div className="flex flex-wrap items-center justify-between gap-4 mb-6">
        <label className="flex items-center gap-2 text-sm text-stitch-muted cursor-pointer">
          <input
            type="checkbox"
            checked={suspectOnly}
            onChange={(e) => setSuspectOnly(e.target.checked)}
            className="rounded border-stitch-border text-stitch-accent"
          />
          Suspect links only
        </label>
        <p className="text-[10px] text-stitch-muted font-mono">
          {filtered.length} link{filtered.length === 1 ? '' : 's'}
        </p>
      </div>

      {statusRollup.length > 0 && (
        <div className="mb-4 flex flex-wrap items-center gap-2">
          <span className="text-[10px] uppercase tracking-widest text-stitch-muted font-bold">
            By verification status
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
        <div className="overflow-x-auto">
          <table className="w-full text-left text-sm min-w-[900px]">
            <thead>
              <tr className="border-b border-stitch-border bg-stitch-elevated text-[10px] text-stitch-muted uppercase tracking-widest">
                <th className="px-4 py-3">Requirement</th>
                <th className="px-4 py-3">Verification</th>
                <th className="px-4 py-3">Test status</th>
                <th className="px-4 py-3">Method</th>
                <th className="px-4 py-3 text-center">Suspect</th>
                <th className="px-4 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-stitch-border">
              {slice.length === 0 ? (
                <tr>
                  <td colSpan={6} className="px-4 py-10 text-center text-stitch-muted">
                    No links match filters.
                  </td>
                </tr>
              ) : (
                slice.map((m) => {
                  const r = reqById.get(m.req_id);
                  const t = verById.get(m.verification_id);
                  const vst = t ? statusById.get(t.status_id) : undefined;
                  const meth =
                    t?.verification_method_id != null
                      ? methodById.get(t.verification_method_id)
                      : undefined;
                  const bkey = `${m.req_id}-${m.verification_id}`;
                  const statusTitle = vst?.title ?? (t ? `Status #${t.status_id}` : 'Unknown');
                  return (
                    <tr key={`${m.req_id}-${m.verification_id}-${m.creation_date}`} className="hover:bg-white/[0.03]">
                      <td className="px-4 py-3">
                        <Link
                          to={`/p/${pid}/requirements/${m.req_id}/edit`}
                          className="font-mono text-stitch-accent hover:underline"
                        >
                          {r?.reference_code ?? `#${m.req_id}`}
                        </Link>
                        <p className="text-xs text-stitch-muted line-clamp-1">{r?.title ?? '—'}</p>
                      </td>
                      <td className="px-4 py-3">
                        <Link
                          to={`/p/${pid}/verifications/${m.verification_id}/edit`}
                          className="font-mono text-stitch-accent hover:underline"
                        >
                          {t?.reference_code ?? `#${m.verification_id}`}
                        </Link>
                        <p className="text-xs text-stitch-muted line-clamp-1">{t?.name ?? '—'}</p>
                      </td>
                      <td className="px-4 py-3 align-middle">
                        <StatusBadge title={statusTitle} tagColor={vst?.tag_color} />
                      </td>
                      <td className="px-4 py-3 text-stitch-muted text-xs">
                        {meth?.title ?? '—'}
                      </td>
                      <td className="px-4 py-3 text-center">
                        {m.suspect ? (
                          <span className="text-[10px] font-bold uppercase text-red-300">Yes</span>
                        ) : (
                          <span className="text-stitch-muted">—</span>
                        )}
                      </td>
                      <td className="px-4 py-3 text-right">
                        {m.suspect ? (
                          <button
                            type="button"
                            disabled={busyKey === bkey}
                            onClick={() => void onClearSuspect(m)}
                            className="text-xs font-bold text-stitch-accent hover:underline disabled:opacity-40"
                          >
                            {busyKey === bkey ? '…' : 'Clear suspect'}
                          </button>
                        ) : null}
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </div>

      {pageCount > 1 && (
        <div className="flex items-center justify-between mt-4 text-xs text-stitch-muted">
          <span>
            Page {safePage} / {pageCount}
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

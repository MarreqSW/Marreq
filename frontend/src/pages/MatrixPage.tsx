import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useOutletContext, useParams } from 'react-router-dom';
import {
  clearTraceabilitySuspect,
  listMatrix,
  listRequirements,
  listVerifications,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';
import type { MatrixLink, Requirement, Verification } from '@/api/types';
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
      const [mx, r, allV] = await Promise.all([
        listMatrix(pid),
        listRequirements(pid),
        listVerifications(),
      ]);
      setMatrix(mx);
      setReqs(r);
      setVers(allV.filter((v) => v.project_id === pid));
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

  const q = globalSearch.trim().toLowerCase();
  const filtered = useMemo(() => {
    let rows = matrix;
    if (suspectOnly) rows = rows.filter((m) => m.suspect);
    if (!q) return rows;
    return rows.filter((m) => {
      const r = reqById.get(m.req_id);
      const t = verById.get(m.verification_id);
      const blob = [
        r?.reference_code,
        r?.title,
        t?.reference_code,
        t?.name,
        String(m.req_id),
        String(m.verification_id),
      ]
        .join(' ')
        .toLowerCase();
      return blob.includes(q);
    });
  }, [matrix, suspectOnly, q, reqById, verById]);

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
        subtitle="All requirement ↔ verification links for this project. Matches legacy matrix data; use Classic UI for bulk Excel/CSV export with extra filters."
      >
        <a
          href={`/p/${pid}/matrix`}
          className="text-xs font-bold uppercase tracking-wider text-stitch-accent border border-stitch-border rounded-md px-3 py-2 hover:bg-white/[0.06]"
        >
          Classic matrix
        </a>
        <button
          type="button"
          onClick={() => void load()}
          className="text-xs font-bold uppercase tracking-wider text-stitch-muted border border-stitch-border rounded-md px-3 py-2 hover:bg-white/[0.06]"
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

      <div className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-sm min-w-[640px]">
            <thead>
              <tr className="border-b border-stitch-border bg-stitch-elevated text-[10px] text-stitch-muted uppercase tracking-widest">
                <th className="px-4 py-3">Requirement</th>
                <th className="px-4 py-3">Verification</th>
                <th className="px-4 py-3 text-center">Suspect</th>
                <th className="px-4 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-stitch-border">
              {slice.length === 0 ? (
                <tr>
                  <td colSpan={4} className="px-4 py-10 text-center text-stitch-muted">
                    No links match filters.
                  </td>
                </tr>
              ) : (
                slice.map((m) => {
                  const r = reqById.get(m.req_id);
                  const t = verById.get(m.verification_id);
                  const bkey = `${m.req_id}-${m.verification_id}`;
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

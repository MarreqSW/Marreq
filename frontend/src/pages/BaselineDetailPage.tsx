import { useCallback, useEffect, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import {
  getBaseline,
  getBaselineRequirements,
  getBaselineTraceability,
  getBaselineVerifications,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';
import type {
  Baseline,
  BaselineTraceabilityRow,
  BaselineVerificationSnapshot,
  Requirement,
} from '@/api/types';

export default function BaselineDetailPage() {
  const { projectId: projectIdParam, baselineId: baselineIdParam } = useParams();
  const pid = Number(projectIdParam);
  const bid = Number(baselineIdParam);
  const { dashboard } = useDashboard();

  const [meta, setMeta] = useState<Baseline | null>(null);
  const [reqs, setReqs] = useState<Requirement[]>([]);
  const [vers, setVers] = useState<BaselineVerificationSnapshot[]>([]);
  const [trace, setTrace] = useState<BaselineTraceabilityRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid) || !Number.isFinite(bid)) return;
    setLoading(true);
    setErr(null);
    try {
      const [b, r, v, t] = await Promise.all([
        getBaseline(pid, bid),
        getBaselineRequirements(pid, bid),
        getBaselineVerifications(pid, bid),
        getBaselineTraceability(pid, bid),
      ]);
      setMeta(b);
      setReqs(r);
      setVers(v);
      setTrace(t);
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load baseline');
    } finally {
      setLoading(false);
    }
  }, [pid, bid]);

  useEffect(() => {
    void load();
  }, [load]);

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  if (loading) {
    return (
      <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
        Loading baseline…
      </div>
    );
  }

  if (err || !meta) {
    return (
      <div className="space-y-4">
        <div className="p-4 rounded-xl bg-red-500/10 border border-red-500/25 text-red-200 text-sm">
          {err ?? 'Not found'}
        </div>
        <Link to={`/p/${pid}/baselines`} className="text-stitch-accent text-sm font-semibold">
          ← Back to baselines
        </Link>
      </div>
    );
  }

  const traceSample = trace.slice(0, 80);

  return (
    <div>
      <nav className="flex items-center gap-2 text-[10px] font-semibold text-stitch-muted mb-4 uppercase tracking-widest">
        <Link to={`/p/${pid}/baselines`} className="hover:text-stitch-accent">
          Baselines
        </Link>
        <span className="material-symbols-outlined text-sm">chevron_right</span>
        <span className="text-stitch-accent font-bold">{meta.name}</span>
      </nav>

      <StitchPageHeader
        projectName={projectName}
        section="Baseline"
        title={meta.name}
        subtitle={meta.description ?? 'Snapshot contents from the API.'}
      >
        <a
          href={`/p/${pid}/baselines/${bid}`}
          className="text-xs font-bold uppercase tracking-wider text-stitch-accent border border-stitch-border rounded-md px-3 py-2 hover:bg-stitch-higher"
        >
          Classic view
        </a>
      </StitchPageHeader>

      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-8">
        <div className="rounded-xl border border-stitch-border bg-stitch-surface p-4 text-center">
          <p className="text-2xl font-extrabold text-stitch-fg">{reqs.length}</p>
          <p className="text-[10px] text-stitch-muted uppercase tracking-widest mt-1">
            Requirements in snapshot
          </p>
        </div>
        <div className="rounded-xl border border-stitch-border bg-stitch-surface p-4 text-center">
          <p className="text-2xl font-extrabold text-stitch-fg">{vers.length}</p>
          <p className="text-[10px] text-stitch-muted uppercase tracking-widest mt-1">
            Verifications in snapshot
          </p>
        </div>
        <div className="rounded-xl border border-stitch-border bg-stitch-surface p-4 text-center">
          <p className="text-2xl font-extrabold text-stitch-fg">{trace.length}</p>
          <p className="text-[10px] text-stitch-muted uppercase tracking-widest mt-1">
            Traceability rows
          </p>
        </div>
      </div>

      <section className="mb-8">
        <h3 className="text-sm font-bold text-stitch-fg uppercase tracking-widest mb-3">
          Sample traceability (first {traceSample.length} of {trace.length})
        </h3>
        <div className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden max-h-96 overflow-y-auto">
          <table className="w-full text-left text-xs">
            <thead className="sticky top-0 bg-stitch-elevated border-b border-stitch-border">
              <tr className="text-stitch-muted uppercase">
                <th className="px-3 py-2">Req #</th>
                <th className="px-3 py-2">Ver #</th>
                <th className="px-3 py-2">Suspect</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-stitch-border">
              {traceSample.length === 0 ? (
                <tr>
                  <td colSpan={3} className="px-3 py-6 text-stitch-muted text-center">
                    No links in baseline
                  </td>
                </tr>
              ) : (
                traceSample.map((row, i) => (
                  <tr key={i} className="hover:bg-white/[0.03]">
                    <td className="px-3 py-2 font-mono text-stitch-accent">{row.requirement_id}</td>
                    <td className="px-3 py-2 font-mono text-stitch-muted">{row.verification_id}</td>
                    <td className="px-3 py-2">{row.suspect ? 'yes' : '—'}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}

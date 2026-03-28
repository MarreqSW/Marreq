import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import {
  getCoverageReport,
  listMatrix,
  listRequirements,
  listVerifications,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';

function StatCard({
  label,
  value,
  hint,
}: {
  label: string;
  value: string | number;
  hint?: string;
}) {
  return (
    <div className="rounded-xl border border-stitch-border bg-stitch-surface p-5 shadow-stitch">
      <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-widest">{label}</p>
      <p className="text-3xl font-extrabold text-stitch-fg mt-2 tabular-nums">{value}</p>
      {hint ? <p className="text-xs text-stitch-muted mt-2">{hint}</p> : null}
    </div>
  );
}

export default function DashboardPage() {
  const { projectId: projectIdParam } = useParams();
  const pid = Number(projectIdParam);
  const { dashboard } = useDashboard();

  const [reqCount, setReqCount] = useState(0);
  const [verCount, setVerCount] = useState(0);
  const [linkCount, setLinkCount] = useState(0);
  const [coverage, setCoverage] = useState<{
    reqNoTest: number;
    verNoReq: number;
    suspect: number;
  } | null>(null);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setLoading(true);
    setErr(null);
    try {
      const [reqs, ver, mx, rep] = await Promise.all([
        listRequirements(pid),
        listVerifications(),
        listMatrix(pid),
        getCoverageReport(pid),
      ]);
      setReqCount(reqs.length);
      setVerCount(ver.filter((v) => v.project_id === pid).length);
      setLinkCount(mx.length);
      setCoverage({
        reqNoTest: rep.requirements_without_tests.length,
        verNoReq: rep.tests_without_requirements.length,
        suspect: rep.suspect_links.length,
      });
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load dashboard');
    } finally {
      setLoading(false);
    }
  }, [pid]);

  useEffect(() => {
    void load();
  }, [load]);

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  const linkedReqRatio = useMemo(() => {
    if (reqCount === 0) return null;
    if (!coverage) return null;
    const withTests = reqCount - coverage.reqNoTest;
    return Math.round((withTests / reqCount) * 100);
  }, [reqCount, coverage]);

  if (loading) {
    return (
      <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
        Loading dashboard…
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
        section="Dashboard"
        title="Project overview"
        subtitle="Counts and traceability health at a glance. Data is live from the API."
      />

      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4 mb-8">
        <StatCard label="Requirements" value={reqCount} />
        <StatCard label="Verifications" value={verCount} />
        <StatCard label="Matrix links" value={linkCount} hint="Requirement ↔ verification ties" />
        <StatCard
          label="Req. with tests"
          value={linkedReqRatio != null ? `${linkedReqRatio}%` : '—'}
          hint={
            coverage
              ? `${coverage.reqNoTest} requirement(s) without any linked test`
              : undefined
          }
        />
      </div>

      {coverage ? (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-10">
          <Link
            to={`/p/${pid}/reports#gaps`}
            className="rounded-xl border border-amber-500/25 bg-amber-500/10 p-4 block hover:bg-amber-500/15 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-stitch-accent"
          >
            <p className="text-xs font-bold text-amber-200 uppercase tracking-wide">Gaps</p>
            <p className="text-2xl font-bold text-stitch-fg mt-1">{coverage.reqNoTest}</p>
            <p className="text-xs text-stitch-muted mt-1">Requirements without tests</p>
            <p className="text-[10px] text-stitch-accent font-bold mt-2 uppercase tracking-wider">
              View in reports →
            </p>
          </Link>
          <Link
            to={`/p/${pid}/reports#orphans`}
            className="rounded-xl border border-stitch-accent/25 bg-stitch-accent/10 p-4 block hover:bg-stitch-accent/20 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-stitch-accent"
          >
            <p className="text-xs font-bold text-stitch-accent uppercase tracking-wide">Orphans</p>
            <p className="text-2xl font-bold text-stitch-fg mt-1">{coverage.verNoReq}</p>
            <p className="text-xs text-stitch-muted mt-1">Tests without requirements</p>
            <p className="text-[10px] text-stitch-accent font-bold mt-2 uppercase tracking-wider">
              View in reports →
            </p>
          </Link>
          <Link
            to={`/p/${pid}/reports#suspect`}
            className="rounded-xl border border-red-500/25 bg-red-500/10 p-4 block hover:bg-red-500/15 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-stitch-accent"
          >
            <p className="text-xs font-bold text-red-200 uppercase tracking-wide">Suspect</p>
            <p className="text-2xl font-bold text-stitch-fg mt-1">{coverage.suspect}</p>
            <p className="text-xs text-stitch-muted mt-1">Links flagged as suspect</p>
            <p className="text-[10px] text-stitch-accent font-bold mt-2 uppercase tracking-wider">
              View in reports →
            </p>
          </Link>
        </div>
      ) : null}

      <h3 className="text-sm font-bold text-stitch-fg uppercase tracking-widest mb-4">Quick links</h3>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
        <Link
          to={`/p/${pid}/requirements`}
          className="rounded-xl border border-stitch-border bg-stitch-elevated px-4 py-4 text-sm font-semibold text-stitch-fg hover:bg-stitch-higher transition-colors flex items-center gap-2"
        >
          <span className="material-symbols-outlined text-stitch-accent">list_alt</span>
          Requirements
        </Link>
        <Link
          to={`/p/${pid}/verifications`}
          className="rounded-xl border border-stitch-border bg-stitch-elevated px-4 py-4 text-sm font-semibold text-stitch-fg hover:bg-stitch-higher transition-colors flex items-center gap-2"
        >
          <span className="material-symbols-outlined text-stitch-accent">verified</span>
          Verifications
        </Link>
        <Link
          to={`/p/${pid}/traceability`}
          className="rounded-xl border border-stitch-border bg-stitch-elevated px-4 py-4 text-sm font-semibold text-stitch-fg hover:bg-stitch-higher transition-colors flex items-center gap-2"
        >
          <span className="material-symbols-outlined text-stitch-accent">account_tree</span>
          Traceability
        </Link>
        <Link
          to={`/p/${pid}/reports`}
          className="rounded-xl border border-stitch-border bg-stitch-elevated px-4 py-4 text-sm font-semibold text-stitch-fg hover:bg-stitch-higher transition-colors flex items-center gap-2"
        >
          <span className="material-symbols-outlined text-stitch-accent">description</span>
          Reports
        </Link>
      </div>
    </div>
  );
}

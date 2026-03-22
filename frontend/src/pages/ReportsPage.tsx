import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { getCoverageReport, listRequirements, listVerifications } from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';

export default function ReportsPage() {
  const { projectId: projectIdParam } = useParams();
  const pid = Number(projectIdParam);
  const { dashboard } = useDashboard();

  const [report, setReport] = useState<Awaited<ReturnType<typeof getCoverageReport>> | null>(
    null,
  );
  const [reqById, setReqById] = useState<Map<number, string>>(new Map());
  const [verById, setVerById] = useState<Map<number, string>>(new Map());
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setLoading(true);
    setErr(null);
    try {
      const [rep, reqs, vers] = await Promise.all([
        getCoverageReport(pid),
        listRequirements(pid),
        listVerifications(),
      ]);
      setReport(rep);
      setReqById(new Map(reqs.map((r) => [r.id, r.reference_code || r.title])));
      setVerById(
        new Map(
          vers.filter((v) => v.project_id === pid).map((v) => [v.id, v.reference_code || v.name]),
        ),
      );
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load report');
    } finally {
      setLoading(false);
    }
  }, [pid]);

  useEffect(() => {
    void load();
  }, [load]);

  const project = dashboard?.projects?.find((p) => p.id === pid);
  const projectName = project?.name ?? 'Project';
  const projectSlug = project?.slug;

  const summary = useMemo(() => {
    if (!report) return null;
    return {
      r: report.requirements_without_tests.length,
      v: report.tests_without_requirements.length,
      s: report.suspect_links.length,
    };
  }, [report]);

  if (loading) {
    return (
      <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
        Loading reports…
      </div>
    );
  }

  if (err || !report) {
    return (
      <div className="p-4 rounded-xl bg-red-500/10 border border-red-500/25 text-red-200 text-sm">
        {err ?? 'No data'}
      </div>
    );
  }

  return (
    <div>
      <StitchPageHeader
        projectName={projectName}
        section="Reports"
        title="Coverage & gaps"
        subtitle="Derived from the traceability matrix: untested requirements, unlinked verifications, and suspect links."
      >
        <button
          type="button"
          onClick={() => void load()}
          className="text-xs font-bold uppercase tracking-wider text-stitch-accent border border-stitch-border rounded-md px-3 py-2 hover:bg-white/[0.06]"
        >
          Refresh
        </button>
      </StitchPageHeader>

      {summary ? (
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-8">
          <div className="rounded-xl border border-stitch-border bg-stitch-surface p-4 text-center">
            <p className="text-3xl font-extrabold text-white">{summary.r}</p>
            <p className="text-[10px] text-stitch-muted uppercase tracking-widest mt-1">
              Req. without tests
            </p>
          </div>
          <div className="rounded-xl border border-stitch-border bg-stitch-surface p-4 text-center">
            <p className="text-3xl font-extrabold text-white">{summary.v}</p>
            <p className="text-[10px] text-stitch-muted uppercase tracking-widest mt-1">
              Tests without req.
            </p>
          </div>
          <div className="rounded-xl border border-stitch-border bg-stitch-surface p-4 text-center">
            <p className="text-3xl font-extrabold text-stitch-accent">{summary.s}</p>
            <p className="text-[10px] text-stitch-muted uppercase tracking-widest mt-1">
              Suspect links
            </p>
          </div>
        </div>
      ) : null}

      <div className="space-y-8">
        <section className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
          <div className="px-4 py-3 border-b border-stitch-border bg-stitch-elevated">
            <h3 className="text-sm font-bold text-white">Requirements without linked tests</h3>
          </div>
          <div className="max-h-72 overflow-auto">
            <table className="w-full text-left text-sm">
              <tbody className="divide-y divide-stitch-border">
                {report.requirements_without_tests.length === 0 ? (
                  <tr>
                    <td className="px-4 py-6 text-stitch-muted text-center">None — full coverage</td>
                  </tr>
                ) : (
                  report.requirements_without_tests.map((id) => (
                    <tr key={id} className="hover:bg-white/[0.03]">
                      <td className="px-4 py-2 font-mono text-stitch-accent">#{id}</td>
                      <td className="px-4 py-2 text-stitch-muted">{reqById.get(id) ?? '—'}</td>
                      <td className="px-4 py-2 text-right">
                        <Link
                          to={`/p/${pid}/requirements/${id}/edit`}
                          className="text-xs font-bold text-stitch-accent hover:underline"
                        >
                          Open
                        </Link>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </section>

        <section className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
          <div className="px-4 py-3 border-b border-stitch-border bg-stitch-elevated">
            <h3 className="text-sm font-bold text-white">Verifications without requirements</h3>
          </div>
          <div className="max-h-72 overflow-auto">
            <table className="w-full text-left text-sm">
              <tbody className="divide-y divide-stitch-border">
                {report.tests_without_requirements.length === 0 ? (
                  <tr>
                    <td className="px-4 py-6 text-stitch-muted text-center">None</td>
                  </tr>
                ) : (
                  report.tests_without_requirements.map((id) => (
                    <tr key={id} className="hover:bg-white/[0.03]">
                      <td className="px-4 py-2 font-mono text-stitch-accent">#{id}</td>
                      <td className="px-4 py-2 text-stitch-muted">{verById.get(id) ?? '—'}</td>
                      <td className="px-4 py-2 text-right">
                        <Link
                          to={`/p/${pid}/verifications/${id}/edit`}
                          className="text-xs font-bold text-stitch-accent hover:underline"
                        >
                          Open
                        </Link>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </section>

        <section className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
          <div className="px-4 py-3 border-b border-stitch-border bg-stitch-elevated">
            <h3 className="text-sm font-bold text-white">Suspect matrix links</h3>
          </div>
          <div className="max-h-72 overflow-auto">
            <table className="w-full text-left text-sm">
              <thead>
                <tr className="border-b border-stitch-border text-[10px] text-stitch-muted uppercase">
                  <th className="px-4 py-2">Requirement</th>
                  <th className="px-4 py-2">Verification</th>
                  <th className="px-4 py-2 text-right">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-stitch-border">
                {report.suspect_links.length === 0 ? (
                  <tr>
                    <td colSpan={3} className="px-4 py-6 text-stitch-muted text-center">
                      No suspect links
                    </td>
                  </tr>
                ) : (
                  report.suspect_links.map((l, i) => (
                    <tr key={`${l.req_id}-${l.verification_id}-${i}`} className="hover:bg-white/[0.03]">
                      <td className="px-4 py-2 font-mono text-stitch-accent">
                        {reqById.get(l.req_id) ?? `REQ #${l.req_id}`}
                      </td>
                      <td className="px-4 py-2 font-mono text-stitch-muted">
                        {verById.get(l.verification_id) ?? `VER #${l.verification_id}`}
                      </td>
                      <td className="px-4 py-2 text-right">
                        <Link
                          to={`/p/${pid}/traceability`}
                          className="text-xs font-bold text-stitch-accent hover:underline"
                        >
                          Graph
                        </Link>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </section>

        {projectSlug ? (
          <section className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
            <div className="px-4 py-3 border-b border-stitch-border bg-stitch-elevated">
              <h3 className="text-sm font-bold text-white">Classic exports (same session)</h3>
              <p className="text-[10px] text-stitch-muted mt-1 uppercase tracking-wide">
                Excel / PDF downloads served by the legacy HTML routes
              </p>
            </div>
            <div className="p-4 flex flex-wrap gap-3 text-sm">
              <a
                href={`/p/${projectSlug}/requirements.xls`}
                className="text-stitch-accent font-bold hover:underline"
              >
                Requirements (.xls)
              </a>
              <a
                href={`/p/${projectSlug}/verifications.xls`}
                className="text-stitch-accent font-bold hover:underline"
              >
                Verifications (.xls)
              </a>
              <a
                href={`/p/${projectSlug}/matrix.xls`}
                className="text-stitch-accent font-bold hover:underline"
              >
                Matrix (.xls)
              </a>
              <a
                href={`/p/${projectSlug}/reports/requirements-pdf`}
                className="text-stitch-accent font-bold hover:underline"
              >
                Requirements PDF
              </a>
              <a
                href={`/p/${projectSlug}/reports/pdf`}
                className="text-stitch-accent font-bold hover:underline"
              >
                Report PDF
              </a>
            </div>
          </section>
        ) : null}
      </div>
    </div>
  );
}

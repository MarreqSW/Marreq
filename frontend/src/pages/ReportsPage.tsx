import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useLocation, useParams } from 'react-router-dom';
import {
  getBaselineTraceability,
  getCoverageReport,
  listBaselines,
  listCategories,
  listMatrix,
  listRequirements,
  listUsersOptional,
  listVerifications,
} from '@/api/client';
import type {
  Baseline,
  BaselineTraceabilityRow,
  Category,
  MatrixLink,
  Requirement,
  User,
  Verification,
} from '@/api/types';
import { ReportSection } from '@/components/reports/ReportSection';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';

const NAV_SECTIONS = [
  { scrollId: 'report-coverage', label: 'Coverage' },
  { scrollId: 'report-matrix', label: 'Matrix' },
  { scrollId: 'report-quality', label: 'Data quality' },
  { scrollId: 'report-workflow', label: 'Workflow' },
  { scrollId: 'report-baseline', label: 'Baseline diff' },
] as const;

function scrollToSection(id: string) {
  document.getElementById(id)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
}

function pairKey(reqId: number, verId: number) {
  return `${reqId}-${verId}`;
}

export default function ReportsPage() {
  const { projectId: projectIdParam } = useParams();
  const pid = Number(projectIdParam);
  const location = useLocation();
  const { dashboard } = useDashboard();

  const [report, setReport] = useState<Awaited<ReturnType<typeof getCoverageReport>> | null>(null);
  const [matrix, setMatrix] = useState<MatrixLink[]>([]);
  const [requirements, setRequirements] = useState<Requirement[]>([]);
  const [verifications, setVerifications] = useState<Verification[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [users, setUsers] = useState<User[] | null>(null);
  const [baselines, setBaselines] = useState<Baseline[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);

  const [baselineId, setBaselineId] = useState<number | ''>('');
  const [baselineRows, setBaselineRows] = useState<BaselineTraceabilityRow[] | null>(null);
  const [baselineLoading, setBaselineLoading] = useState(false);
  const [baselineErr, setBaselineErr] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setLoading(true);
    setErr(null);
    try {
      const [rep, mx, reqs, vers, cats, u, bl] = await Promise.all([
        getCoverageReport(pid),
        listMatrix(pid),
        listRequirements(pid),
        listVerifications(),
        listCategories(),
        listUsersOptional(),
        listBaselines(pid),
      ]);
      setReport(rep);
      setMatrix(mx);
      setRequirements(reqs);
      setVerifications(vers.filter((v) => v.project_id === pid));
      setCategories(cats.filter((c) => c.project_id === pid));
      setUsers(u);
      setBaselines(bl);
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load report');
    } finally {
      setLoading(false);
    }
  }, [pid]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (loading) return;
    const h = location.hash.replace(/^#/, '');
    if (!h) return;
    const t = window.setTimeout(() => {
      document.getElementById(h)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }, 100);
    return () => window.clearTimeout(t);
  }, [loading, location.hash]);

  useEffect(() => {
    if (baselineId === '' || !Number.isFinite(pid)) {
      setBaselineRows(null);
      setBaselineErr(null);
      return;
    }
    let cancelled = false;
    setBaselineLoading(true);
    setBaselineErr(null);
    void getBaselineTraceability(pid, baselineId)
      .then((rows) => {
        if (!cancelled) setBaselineRows(rows);
      })
      .catch((e) => {
        if (!cancelled) setBaselineErr(e instanceof Error ? e.message : 'Failed to load baseline');
      })
      .finally(() => {
        if (!cancelled) setBaselineLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [pid, baselineId]);

  const project = dashboard?.projects?.find((p) => p.id === pid);
  const projectName = project?.name ?? 'Project';
  const projectSlug = project?.slug;

  const reqById = useMemo(
    () => new Map(requirements.map((r) => [r.id, r.reference_code?.trim() || r.title])),
    [requirements],
  );

  const verById = useMemo(
    () =>
      new Map(verifications.map((v) => [v.id, v.reference_code?.trim() || v.name])),
    [verifications],
  );

  const verIdSet = useMemo(() => new Set(verifications.map((v) => v.id)), [verifications]);

  const categoryById = useMemo(() => {
    const m = new Map<number, string>();
    for (const c of categories) m.set(c.id, c.title);
    return m;
  }, [categories]);

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

  const summary = useMemo(() => {
    if (!report) return null;
    return {
      r: report.requirements_without_tests.length,
      v: report.tests_without_requirements.length,
      s: report.suspect_links.length,
    };
  }, [report]);

  const matrixAnalytics = useMemo(() => {
    const byReq = new Map<number, MatrixLink[]>();
    const byVer = new Map<number, Set<number>>();
    for (const m of matrix) {
      const ra = byReq.get(m.req_id) ?? [];
      ra.push(m);
      byReq.set(m.req_id, ra);
      if (!byVer.has(m.verification_id)) byVer.set(m.verification_id, new Set());
      byVer.get(m.verification_id)!.add(m.req_id);
    }

    const reqOnlySuspectIds: number[] = [];
    for (const [rid, links] of byReq) {
      if (links.length > 0 && links.every((l) => l.suspect)) reqOnlySuspectIds.push(rid);
    }
    reqOnlySuspectIds.sort((a, b) => a - b);

    const verHubs = [...byVer.entries()]
      .map(([verification_id, set]) => ({ verification_id, count: set.size }))
      .sort((a, b) => b.count - a.count);

    const reqIdsInMatrix = new Set(matrix.map((m) => m.req_id));
    const linkCountByReq = new Map<number, number>();
    for (const m of matrix) {
      linkCountByReq.set(m.req_id, (linkCountByReq.get(m.req_id) ?? 0) + 1);
    }
    let dist0 = 0;
    let dist1 = 0;
    let dist2 = 0;
    for (const r of requirements) {
      const n = linkCountByReq.get(r.id) ?? 0;
      if (n === 0) dist0 += 1;
      else if (n === 1) dist1 += 1;
      else dist2 += 1;
    }

    const categoryStats: Array<{
      categoryId: number;
      title: string;
      total: number;
      withLink: number;
      pct: number | null;
    }> = [];
    const reqsByCat = new Map<number, Requirement[]>();
    for (const r of requirements) {
      const arr = reqsByCat.get(r.category_id) ?? [];
      arr.push(r);
      reqsByCat.set(r.category_id, arr);
    }
    for (const [categoryId, reqs] of reqsByCat) {
      const total = reqs.length;
      const withLink = reqs.filter((r) => reqIdsInMatrix.has(r.id)).length;
      const pct = total > 0 ? Math.round((withLink / total) * 100) : null;
      categoryStats.push({
        categoryId,
        title: categoryById.get(categoryId) ?? `Category #${categoryId}`,
        total,
        withLink,
        pct,
      });
    }
    categoryStats.sort((a, b) => a.title.localeCompare(b.title));

    return {
      reqOnlySuspectIds,
      verHubs,
      dist0,
      dist1,
      dist2,
      categoryStats,
    };
  }, [matrix, requirements, categoryById]);

  const qualityReqs = useMemo(() => {
    const catIds = new Set(categories.map((c) => c.id));
    const issues: Array<{ id: number; label: string; reasons: string[] }> = [];
    for (const r of requirements) {
      const reasons: string[] = [];
      if (!(r.reference_code ?? '').trim()) reasons.push('Missing key (reference code)');
      if (!catIds.has(r.category_id)) reasons.push('Unknown or invalid category');
      if (!(r.description ?? '').trim()) reasons.push('Empty description');
      if (reasons.length) issues.push({ id: r.id, label: reqById.get(r.id) ?? `REQ #${r.id}`, reasons });
    }
    return issues.sort((a, b) => a.id - b.id);
  }, [requirements, categories, reqById]);

  const qualityVers = useMemo(() => {
    const issues: Array<{ id: number; label: string; reasons: string[] }> = [];
    for (const v of verifications) {
      const reasons: string[] = [];
      if (v.verification_method_id == null) reasons.push('No verification type');
      if (!(v.reference_code ?? '').trim()) reasons.push('Missing key (reference code)');
      if (v.parent_id != null && !verIdSet.has(v.parent_id)) reasons.push('Invalid parent verification');
      if (reasons.length) issues.push({ id: v.id, label: verById.get(v.id) ?? `VER #${v.id}`, reasons });
    }
    return issues.sort((a, b) => a.id - b.id);
  }, [verifications, verIdSet, verById]);

  const approvalGroups = useMemo(() => {
    const m = new Map<string, Requirement[]>();
    for (const r of requirements) {
      const k = r.approval_state || 'unknown';
      const arr = m.get(k) ?? [];
      arr.push(r);
      m.set(k, arr);
    }
    return [...m.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  }, [requirements]);

  const authorGroups = useMemo(() => {
    const m = new Map<number, Requirement[]>();
    for (const r of requirements) {
      const arr = m.get(r.author_id) ?? [];
      arr.push(r);
      m.set(r.author_id, arr);
    }
    return [...m.entries()].sort((a, b) => a[0] - b[0]);
  }, [requirements]);

  const reviewerGroups = useMemo(() => {
    const m = new Map<number, Requirement[]>();
    for (const r of requirements) {
      const arr = m.get(r.reviewer_id) ?? [];
      arr.push(r);
      m.set(r.reviewer_id, arr);
    }
    return [...m.entries()].sort((a, b) => a[0] - b[0]);
  }, [requirements]);

  const baselineDiff = useMemo(() => {
    if (baselineRows == null) return null;
    const basePairs = new Map<string, boolean>();
    for (const row of baselineRows) {
      basePairs.set(pairKey(row.requirement_id, row.verification_id), row.suspect);
    }
    const curPairs = new Map<string, boolean>();
    for (const m of matrix) {
      curPairs.set(pairKey(m.req_id, m.verification_id), m.suspect);
    }

    const added: Array<{ req_id: number; verification_id: number; suspect: boolean }> = [];
    const removed: Array<{ req_id: number; verification_id: number }> = [];
    const suspectChanged: Array<{
      req_id: number;
      verification_id: number;
      baselineSuspect: boolean;
      currentSuspect: boolean;
    }> = [];

    for (const [k, curS] of curPairs) {
      if (!basePairs.has(k)) {
        const [rs, vs] = k.split('-').map(Number);
        added.push({ req_id: rs, verification_id: vs, suspect: curS });
      }
    }
    for (const [k, baseS] of basePairs) {
      if (!curPairs.has(k)) {
        const [rs, vs] = k.split('-').map(Number);
        removed.push({ req_id: rs, verification_id: vs });
      } else {
        const curS = curPairs.get(k)!;
        if (curS !== baseS) {
          const [rs, vs] = k.split('-').map(Number);
          suspectChanged.push({
            req_id: rs,
            verification_id: vs,
            baselineSuspect: baseS,
            currentSuspect: curS,
          });
        }
      }
    }

    added.sort((a, b) => a.req_id - b.req_id || a.verification_id - b.verification_id);
    removed.sort((a, b) => a.req_id - b.req_id || a.verification_id - b.verification_id);
    suspectChanged.sort((a, b) => a.req_id - b.req_id || a.verification_id - b.verification_id);

    return { added, removed, suspectChanged };
  }, [baselineRows, matrix]);

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
        subtitle="Traceability health, matrix analytics, data quality, workflow, and baseline comparison."
      >
        <button
          type="button"
          onClick={() => void load()}
          className="text-xs font-bold uppercase tracking-wider text-stitch-accent border border-stitch-border rounded-md px-3 py-2 hover:bg-stitch-higher"
        >
          Refresh
        </button>
      </StitchPageHeader>

      <nav
        className="sticky top-0 z-20 flex flex-wrap gap-2 py-3 mb-6 -mx-1 px-1 bg-stitch-canvas/90 backdrop-blur-md border-b border-stitch-border/60"
        aria-label="Report sections"
      >
        {NAV_SECTIONS.map(({ scrollId, label }) => (
          <button
            key={scrollId}
            type="button"
            onClick={() => scrollToSection(scrollId)}
            className="text-xs font-bold uppercase tracking-wider px-3 py-1.5 rounded-md border border-stitch-border text-stitch-muted hover:text-stitch-accent hover:bg-stitch-elevated transition-colors"
          >
            {label}
          </button>
        ))}
      </nav>

      {summary ? (
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-8">
          <a
            href={`#gaps`}
            className="rounded-xl border border-stitch-border bg-stitch-surface p-4 text-center hover:bg-stitch-higher transition-colors"
          >
            <p className="text-3xl font-extrabold text-stitch-fg">{summary.r}</p>
            <p className="text-[10px] text-stitch-muted uppercase tracking-widest mt-1">
              Req. without tests
            </p>
          </a>
          <a
            href={`#orphans`}
            className="rounded-xl border border-stitch-border bg-stitch-surface p-4 text-center hover:bg-stitch-higher transition-colors"
          >
            <p className="text-3xl font-extrabold text-stitch-fg">{summary.v}</p>
            <p className="text-[10px] text-stitch-muted uppercase tracking-widest mt-1">
              Tests without req.
            </p>
          </a>
          <a
            href={`#suspect`}
            className="rounded-xl border border-stitch-border bg-stitch-surface p-4 text-center hover:bg-stitch-higher transition-colors"
          >
            <p className="text-3xl font-extrabold text-stitch-accent">{summary.s}</p>
            <p className="text-[10px] text-stitch-muted uppercase tracking-widest mt-1">
              Suspect links
            </p>
          </a>
        </div>
      ) : null}

      <div id="report-coverage" className="space-y-8 scroll-mt-24 mb-16">
        <ReportSection id="gaps" title="Requirements without linked tests">
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
        </ReportSection>

        <ReportSection id="orphans" title="Verifications without requirements">
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
        </ReportSection>

        <ReportSection id="suspect" title="Suspect matrix links">
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
                          to={`/p/${pid}/matrix`}
                          className="text-xs font-bold text-stitch-accent hover:underline mr-2"
                        >
                          Matrix
                        </Link>
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
        </ReportSection>

        {projectSlug ? (
          <ReportSection
            title="Classic exports (same session)"
            subtitle="Excel / PDF downloads served by the legacy HTML routes"
          >
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
          </ReportSection>
        ) : null}
      </div>

      <div id="report-matrix" className="space-y-8 scroll-mt-24 mb-16">
        <ReportSection
          title="Requirements linked only via suspect ties"
          subtitle="Has matrix links, but every link is flagged suspect"
        >
          <div className="max-h-60 overflow-auto">
            {matrixAnalytics.reqOnlySuspectIds.length === 0 ? (
              <p className="px-4 py-6 text-stitch-muted text-sm text-center">None</p>
            ) : (
              <table className="w-full text-left text-sm">
                <tbody className="divide-y divide-stitch-border">
                  {matrixAnalytics.reqOnlySuspectIds.map((id) => (
                    <tr key={id} className="hover:bg-white/[0.03]">
                      <td className="px-4 py-2 font-mono text-stitch-accent">{reqById.get(id) ?? `#${id}`}</td>
                      <td className="px-4 py-2 text-right">
                        <Link
                          to={`/p/${pid}/requirements/${id}/edit`}
                          className="text-xs font-bold text-stitch-accent hover:underline"
                        >
                          Open
                        </Link>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </ReportSection>

        <ReportSection title="Verification hubs" subtitle="Verifications linked to the most requirements">
          <div className="max-h-72 overflow-auto">
            <table className="w-full text-left text-sm">
              <thead>
                <tr className="border-b border-stitch-border text-[10px] text-stitch-muted uppercase">
                  <th className="px-4 py-2">Verification</th>
                  <th className="px-4 py-2 text-right">Linked reqs</th>
                  <th className="px-4 py-2 text-right">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-stitch-border">
                {matrixAnalytics.verHubs.length === 0 ? (
                  <tr>
                    <td colSpan={3} className="px-4 py-6 text-stitch-muted text-center">
                      No matrix data
                    </td>
                  </tr>
                ) : (
                  matrixAnalytics.verHubs.map(({ verification_id, count }) => (
                    <tr key={verification_id} className="hover:bg-white/[0.03]">
                      <td className="px-4 py-2 font-mono text-stitch-accent">
                        {verById.get(verification_id) ?? `#${verification_id}`}
                      </td>
                      <td className="px-4 py-2 text-right tabular-nums">{count}</td>
                      <td className="px-4 py-2 text-right">
                        <Link
                          to={`/p/${pid}/verifications/${verification_id}`}
                          className="text-xs font-bold text-stitch-accent hover:underline"
                        >
                          View
                        </Link>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </ReportSection>

        <ReportSection title="Requirements by number of matrix links" subtitle="Counts across this project">
          <div className="p-4 grid grid-cols-1 sm:grid-cols-3 gap-4 text-sm">
            <div className="rounded-lg border border-stitch-border bg-stitch-elevated/50 p-4 text-center">
              <p className="text-2xl font-bold text-stitch-fg">{matrixAnalytics.dist0}</p>
              <p className="text-[10px] text-stitch-muted uppercase mt-1">No links</p>
            </div>
            <div className="rounded-lg border border-stitch-border bg-stitch-elevated/50 p-4 text-center">
              <p className="text-2xl font-bold text-stitch-fg">{matrixAnalytics.dist1}</p>
              <p className="text-[10px] text-stitch-muted uppercase mt-1">Exactly one</p>
            </div>
            <div className="rounded-lg border border-stitch-border bg-stitch-elevated/50 p-4 text-center">
              <p className="text-2xl font-bold text-stitch-fg">{matrixAnalytics.dist2}</p>
              <p className="text-[10px] text-stitch-muted uppercase mt-1">Two or more</p>
            </div>
          </div>
        </ReportSection>

        <ReportSection title="Coverage by category" subtitle="Requirements with at least one matrix link">
          <div className="max-h-72 overflow-auto">
            <table className="w-full text-left text-sm">
              <thead>
                <tr className="border-b border-stitch-border text-[10px] text-stitch-muted uppercase">
                  <th className="px-4 py-2">Category</th>
                  <th className="px-4 py-2 text-right">With link</th>
                  <th className="px-4 py-2 text-right">Total</th>
                  <th className="px-4 py-2 text-right">%</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-stitch-border">
                {matrixAnalytics.categoryStats.length === 0 ? (
                  <tr>
                    <td colSpan={4} className="px-4 py-6 text-stitch-muted text-center">
                      No categories
                    </td>
                  </tr>
                ) : (
                  matrixAnalytics.categoryStats.map((c) => (
                    <tr key={c.categoryId} className="hover:bg-white/[0.03]">
                      <td className="px-4 py-2">{c.title}</td>
                      <td className="px-4 py-2 text-right tabular-nums">{c.withLink}</td>
                      <td className="px-4 py-2 text-right tabular-nums">{c.total}</td>
                      <td className="px-4 py-2 text-right tabular-nums">
                        {c.pct != null ? `${c.pct}%` : '—'}
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </ReportSection>
      </div>

      <div id="report-quality" className="space-y-8 scroll-mt-24 mb-16">
        <ReportSection
          title="Requirement data quality"
          subtitle="Missing key, invalid category, or empty description"
        >
          <div className="max-h-72 overflow-auto">
            {qualityReqs.length === 0 ? (
              <p className="px-4 py-6 text-stitch-muted text-sm text-center">No issues detected</p>
            ) : (
              <table className="w-full text-left text-sm">
                <thead>
                  <tr className="border-b border-stitch-border text-[10px] text-stitch-muted uppercase">
                    <th className="px-4 py-2">Requirement</th>
                    <th className="px-4 py-2">Issues</th>
                    <th className="px-4 py-2 text-right">Open</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-stitch-border">
                  {qualityReqs.map((row) => (
                    <tr key={row.id} className="hover:bg-white/[0.03]">
                      <td className="px-4 py-2 font-mono text-stitch-accent">{row.label}</td>
                      <td className="px-4 py-2 text-stitch-muted text-xs">{row.reasons.join(' · ')}</td>
                      <td className="px-4 py-2 text-right">
                        <Link
                          to={`/p/${pid}/requirements/${row.id}/edit`}
                          className="text-xs font-bold text-stitch-accent hover:underline"
                        >
                          Edit
                        </Link>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </ReportSection>

        <ReportSection
          title="Verification data quality"
          subtitle="Missing type or key, or broken parent reference"
        >
          <div className="max-h-72 overflow-auto">
            {qualityVers.length === 0 ? (
              <p className="px-4 py-6 text-stitch-muted text-sm text-center">No issues detected</p>
            ) : (
              <table className="w-full text-left text-sm">
                <thead>
                  <tr className="border-b border-stitch-border text-[10px] text-stitch-muted uppercase">
                    <th className="px-4 py-2">Verification</th>
                    <th className="px-4 py-2">Issues</th>
                    <th className="px-4 py-2 text-right">Open</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-stitch-border">
                  {qualityVers.map((row) => (
                    <tr key={row.id} className="hover:bg-white/[0.03]">
                      <td className="px-4 py-2 font-mono text-stitch-accent">{row.label}</td>
                      <td className="px-4 py-2 text-stitch-muted text-xs">{row.reasons.join(' · ')}</td>
                      <td className="px-4 py-2 text-right">
                        <Link
                          to={`/p/${pid}/verifications/${row.id}/edit`}
                          className="text-xs font-bold text-stitch-accent hover:underline"
                        >
                          Edit
                        </Link>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </ReportSection>
      </div>

      <div id="report-workflow" className="space-y-8 scroll-mt-24 mb-16">
        <ReportSection title="Requirements by approval state">
          <div className="space-y-4 p-4">
            {approvalGroups.map(([state, reqs]) => (
              <div key={state} className="border border-stitch-border rounded-lg overflow-hidden">
                <div className="px-3 py-2 bg-stitch-elevated text-xs font-bold text-stitch-fg">
                  {state.replace(/_/g, ' ')} <span className="text-stitch-muted">({reqs.length})</span>
                </div>
                <ul className="max-h-40 overflow-auto divide-y divide-stitch-border text-sm">
                  {reqs.map((r) => (
                    <li key={r.id} className="px-3 py-1.5 flex justify-between gap-2">
                      <span className="font-mono text-stitch-accent truncate">{reqById.get(r.id)}</span>
                      <Link
                        to={`/p/${pid}/requirements/${r.id}/edit`}
                        className="text-xs font-bold text-stitch-accent hover:underline shrink-0"
                      >
                        Edit
                      </Link>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </ReportSection>

        <ReportSection title="Requirements by author">
          <div className="space-y-4 p-4">
            {authorGroups.map(([uid, reqs]) => (
              <div key={uid} className="border border-stitch-border rounded-lg overflow-hidden">
                <div className="px-3 py-2 bg-stitch-elevated text-xs font-bold text-stitch-fg">
                  {userLabel(uid)} <span className="text-stitch-muted">({reqs.length})</span>
                </div>
                <ul className="max-h-36 overflow-auto divide-y divide-stitch-border text-sm">
                  {reqs.map((r) => (
                    <li key={r.id} className="px-3 py-1.5 flex justify-between gap-2">
                      <span className="font-mono text-stitch-accent truncate">{reqById.get(r.id)}</span>
                      <Link
                        to={`/p/${pid}/requirements/${r.id}`}
                        className="text-xs font-bold text-stitch-accent hover:underline shrink-0"
                      >
                        View
                      </Link>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </ReportSection>

        <ReportSection title="Requirements by reviewer">
          <div className="space-y-4 p-4">
            {reviewerGroups.map(([uid, reqs]) => (
              <div key={uid} className="border border-stitch-border rounded-lg overflow-hidden">
                <div className="px-3 py-2 bg-stitch-elevated text-xs font-bold text-stitch-fg">
                  {userLabel(uid)} <span className="text-stitch-muted">({reqs.length})</span>
                </div>
                <ul className="max-h-36 overflow-auto divide-y divide-stitch-border text-sm">
                  {reqs.map((r) => (
                    <li key={r.id} className="px-3 py-1.5 flex justify-between gap-2">
                      <span className="font-mono text-stitch-accent truncate">{reqById.get(r.id)}</span>
                      <Link
                        to={`/p/${pid}/requirements/${r.id}`}
                        className="text-xs font-bold text-stitch-accent hover:underline shrink-0"
                      >
                        View
                      </Link>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        </ReportSection>
      </div>

      <div id="report-baseline" className="space-y-8 scroll-mt-24 mb-8">
        <ReportSection
          title="Traceability vs baseline"
          subtitle="Compare current matrix to a saved baseline snapshot"
        >
          <div className="p-4 space-y-4">
            {baselines.length === 0 ? (
              <p className="text-sm text-stitch-muted">
                No baselines for this project. Create one from the{' '}
                <Link to={`/p/${pid}/baselines`} className="text-stitch-accent font-bold hover:underline">
                  Baselines
                </Link>{' '}
                page.
              </p>
            ) : (
              <>
                <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider">
                  Baseline
                  <select
                    className="mt-1 block w-full max-w-md text-sm bg-stitch-elevated border border-stitch-border rounded-md px-2 py-2 text-stitch-fg"
                    value={baselineId === '' ? '' : String(baselineId)}
                    onChange={(e) =>
                      setBaselineId(e.target.value === '' ? '' : Number(e.target.value))
                    }
                  >
                    <option value="">Select baseline…</option>
                    {baselines.map((b) => (
                      <option key={b.id} value={b.id}>
                        {b.name}
                      </option>
                    ))}
                  </select>
                </label>
                {baselineLoading ? (
                  <p className="text-sm text-stitch-muted">Loading baseline traceability…</p>
                ) : baselineErr ? (
                  <p className="text-sm text-red-300">{baselineErr}</p>
                ) : baselineDiff ? (
                  <div className="space-y-6">
                    <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 text-center text-sm">
                      <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 p-3">
                        <p className="text-xl font-bold text-stitch-fg">{baselineDiff.added.length}</p>
                        <p className="text-[10px] text-stitch-muted uppercase">Links added</p>
                      </div>
                      <div className="rounded-lg border border-amber-500/30 bg-amber-500/10 p-3">
                        <p className="text-xl font-bold text-stitch-fg">{baselineDiff.removed.length}</p>
                        <p className="text-[10px] text-stitch-muted uppercase">Links removed</p>
                      </div>
                      <div className="rounded-lg border border-stitch-accent/30 bg-stitch-accent/10 p-3">
                        <p className="text-xl font-bold text-stitch-fg">
                          {baselineDiff.suspectChanged.length}
                        </p>
                        <p className="text-[10px] text-stitch-muted uppercase">Suspect flag changed</p>
                      </div>
                    </div>

                    {baselineDiff.added.length > 0 ? (
                      <div>
                        <h4 className="text-xs font-bold text-stitch-fg mb-2">Added links</h4>
                        <div className="max-h-48 overflow-auto border border-stitch-border rounded-lg">
                          <table className="w-full text-left text-xs">
                            <tbody className="divide-y divide-stitch-border">
                              {baselineDiff.added.map((l) => (
                                <tr key={pairKey(l.req_id, l.verification_id)}>
                                  <td className="px-3 py-1.5 font-mono text-stitch-accent">
                                    {reqById.get(l.req_id) ?? `#${l.req_id}`}
                                  </td>
                                  <td className="px-3 py-1.5 font-mono text-stitch-muted">
                                    {verById.get(l.verification_id) ?? `#${l.verification_id}`}
                                  </td>
                                  <td className="px-3 py-1.5 text-stitch-muted">
                                    {l.suspect ? 'suspect' : 'ok'}
                                  </td>
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        </div>
                      </div>
                    ) : null}

                    {baselineDiff.removed.length > 0 ? (
                      <div>
                        <h4 className="text-xs font-bold text-stitch-fg mb-2">Removed links</h4>
                        <div className="max-h-48 overflow-auto border border-stitch-border rounded-lg">
                          <table className="w-full text-left text-xs">
                            <tbody className="divide-y divide-stitch-border">
                              {baselineDiff.removed.map((l) => (
                                <tr key={pairKey(l.req_id, l.verification_id)}>
                                  <td className="px-3 py-1.5 font-mono text-stitch-accent">
                                    {reqById.get(l.req_id) ?? `#${l.req_id}`}
                                  </td>
                                  <td className="px-3 py-1.5 font-mono text-stitch-muted">
                                    {verById.get(l.verification_id) ?? `#${l.verification_id}`}
                                  </td>
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        </div>
                      </div>
                    ) : null}

                    {baselineDiff.suspectChanged.length > 0 ? (
                      <div>
                        <h4 className="text-xs font-bold text-stitch-fg mb-2">Suspect flag changes</h4>
                        <div className="max-h-48 overflow-auto border border-stitch-border rounded-lg">
                          <table className="w-full text-left text-xs">
                            <thead>
                              <tr className="border-b border-stitch-border text-[10px] text-stitch-muted uppercase">
                                <th className="px-3 py-1.5">Requirement</th>
                                <th className="px-3 py-1.5">Verification</th>
                                <th className="px-3 py-1.5">Was</th>
                                <th className="px-3 py-1.5">Now</th>
                              </tr>
                            </thead>
                            <tbody className="divide-y divide-stitch-border">
                              {baselineDiff.suspectChanged.map((l) => (
                                <tr key={pairKey(l.req_id, l.verification_id)}>
                                  <td className="px-3 py-1.5 font-mono text-stitch-accent">
                                    {reqById.get(l.req_id) ?? `#${l.req_id}`}
                                  </td>
                                  <td className="px-3 py-1.5 font-mono text-stitch-muted">
                                    {verById.get(l.verification_id) ?? `#${l.verification_id}`}
                                  </td>
                                  <td className="px-3 py-1.5">
                                    {l.baselineSuspect ? 'suspect' : 'clear'}
                                  </td>
                                  <td className="px-3 py-1.5">
                                    {l.currentSuspect ? 'suspect' : 'clear'}
                                  </td>
                                </tr>
                              ))}
                            </tbody>
                          </table>
                        </div>
                      </div>
                    ) : null}

                    {baselineId !== '' &&
                    baselineDiff.added.length === 0 &&
                    baselineDiff.removed.length === 0 &&
                    baselineDiff.suspectChanged.length === 0 ? (
                      <p className="text-sm text-stitch-muted">Matrix matches baseline traceability.</p>
                    ) : null}
                  </div>
                ) : null}
              </>
            )}
          </div>
        </ReportSection>
      </div>
    </div>
  );
}

import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useOutletContext, useParams } from 'react-router-dom';
import {
  compareBaselineRequirementWithCurrent,
  compareBaselineVerificationWithCurrent,
  compareRequirementVersionsByProject,
  getBaseline,
  getBaselineRequirements,
  getBaselineTraceability,
  getBaselineVerifications,
  listBaselines,
  listRequirements,
  listVerificationMethodsByProject,
  listVerificationStatuses,
  listVerifications,
} from '@/api/client';
import { downloadBaselineReqif } from '@/api/exports';
import { useDashboard } from '@/context/DashboardContext';
import AsyncDiffDialog from '@/components/AsyncDiffDialog';
import { RequirementDiffContent } from '@/components/RequirementVersionDiffDialog';
import StitchPageHeader from '@/components/StitchPageHeader';
import { VerificationDiffContent } from '@/components/VerificationVersionDiffDialog';
import type {
  Baseline,
  BaselineTraceabilityRow,
  BaselineVerificationSnapshot,
  Requirement,
  RequirementDiff,
  Verification,
  VerificationMethod,
  VerificationStatus,
  VerificationVersionDiff,
} from '@/api/types';
import type { ProjectOutletContext } from '@/types/projectOutlet';

export default function BaselineDetailPage() {
  const { basePath, projectId } = useOutletContext<ProjectOutletContext>();
  const { baselineId: baselineIdParam } = useParams();
  const pid = projectId;
  const bid = Number(baselineIdParam);
  const { dashboard } = useDashboard();

  const [meta, setMeta] = useState<Baseline | null>(null);
  const [reqs, setReqs] = useState<Requirement[]>([]);
  const [vers, setVers] = useState<BaselineVerificationSnapshot[]>([]);
  const [trace, setTrace] = useState<BaselineTraceabilityRow[]>([]);
  const [baselines, setBaselines] = useState<Baseline[]>([]);
  const [currentReqs, setCurrentReqs] = useState<Requirement[]>([]);
  const [currentVers, setCurrentVers] = useState<Verification[]>([]);
  const [verificationStatuses, setVerificationStatuses] = useState<VerificationStatus[]>([]);
  const [verificationMethods, setVerificationMethods] = useState<VerificationMethod[]>([]);
  const [compareBaselineId, setCompareBaselineId] = useState<number | ''>('');
  const [compareReqs, setCompareReqs] = useState<Requirement[]>([]);
  const [reqDiff, setReqDiff] = useState<{
    requirementId: number;
    title: string;
    load: () => Promise<RequirementDiff>;
  } | null>(null);
  const [verificationDiff, setVerificationDiff] = useState<{
    verificationId: number;
    title: string;
    load: () => Promise<VerificationVersionDiff>;
  } | null>(null);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [reqifBusy, setReqifBusy] = useState(false);
  const [reqifErr, setReqifErr] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid) || !Number.isFinite(bid)) return;
    setLoading(true);
    setErr(null);
    try {
      const [b, r, v, t, allBaselines, liveReqs, liveVers, statuses, methods] = await Promise.all([
        getBaseline(pid, bid),
        getBaselineRequirements(pid, bid),
        getBaselineVerifications(pid, bid),
        getBaselineTraceability(pid, bid),
        listBaselines(pid),
        listRequirements(pid),
        listVerifications(),
        listVerificationStatuses(),
        listVerificationMethodsByProject(pid),
      ]);
      setMeta(b);
      setReqs(r);
      setVers(v);
      setTrace(t);
      setBaselines(allBaselines.filter((candidate) => candidate.id !== bid));
      setCurrentReqs(liveReqs);
      setCurrentVers(liveVers.filter((verification) => verification.project_id === pid));
      setVerificationStatuses(statuses);
      setVerificationMethods(methods);
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load baseline');
    } finally {
      setLoading(false);
    }
  }, [pid, bid]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (compareBaselineId === '') {
      setCompareReqs([]);
      return;
    }
    let alive = true;
    getBaselineRequirements(pid, compareBaselineId)
      .then((rows) => {
        if (alive) setCompareReqs(rows);
      })
      .catch(() => {
        if (alive) setCompareReqs([]);
      });
    return () => {
      alive = false;
    };
  }, [compareBaselineId, pid]);

  const currentReqById = useMemo(
    () => new Map(currentReqs.map((requirement) => [requirement.id, requirement])),
    [currentReqs],
  );
  const compareReqById = useMemo(
    () => new Map(compareReqs.map((requirement) => [requirement.id, requirement])),
    [compareReqs],
  );
  const currentVerById = useMemo(
    () => new Map(currentVers.map((verification) => [verification.id, verification])),
    [currentVers],
  );
  const statusById = useMemo(
    () => new Map(verificationStatuses.map((status) => [status.id, status.title])),
    [verificationStatuses],
  );
  const methodById = useMemo(
    () => new Map(verificationMethods.map((method) => [method.id, method.title])),
    [verificationMethods],
  );

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
        <Link to={`${basePath}/baselines`} className="text-stitch-accent text-sm font-semibold">
          ← Back to baselines
        </Link>
      </div>
    );
  }

  const traceSample = trace.slice(0, 80);

  return (
    <div>
      <nav className="flex items-center gap-2 text-[10px] font-semibold text-stitch-muted mb-4 uppercase tracking-widest">
        <Link to={`${basePath}/baselines`} className="hover:text-stitch-accent">
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
        <button
          type="button"
          disabled={reqifBusy}
          onClick={() => {
            setReqifErr(null);
            setReqifBusy(true);
            void downloadBaselineReqif(pid, bid)
              .catch((e) =>
                setReqifErr(e instanceof Error ? e.message : 'ReqIF export failed'),
              )
              .finally(() => setReqifBusy(false));
          }}
          className="text-xs font-bold uppercase tracking-wider text-stitch-accent border border-stitch-border rounded-md px-3 py-2 hover:bg-stitch-higher disabled:opacity-50"
        >
          {reqifBusy ? 'Exporting…' : 'Export ReqIF'}
        </button>
      </StitchPageHeader>
      {reqifErr ? (
        <div className="mb-4 rounded-xl bg-red-500/10 border border-red-500/25 text-red-200 text-sm p-4">
          {reqifErr}
        </div>
      ) : null}

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

      <section className="mb-8 rounded-xl border border-stitch-border bg-stitch-surface p-5">
        <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
          <div>
            <h3 className="text-sm font-bold uppercase tracking-widest text-stitch-fg">
              Requirement snapshots
            </h3>
            <p className="mt-1 text-xs text-stitch-muted">
              Compare each frozen requirement with current HEAD or with another baseline.
            </p>
          </div>
          <label className="text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
            Compare with baseline
            <select
              value={compareBaselineId}
              onChange={(event) =>
                setCompareBaselineId(event.target.value ? Number(event.target.value) : '')
              }
              className="mt-1 block min-w-64 rounded-md border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm font-normal normal-case tracking-normal text-stitch-fg"
            >
              <option value="">None</option>
              {baselines.map((baseline) => (
                <option key={baseline.id} value={baseline.id}>
                  {baseline.name}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="max-h-[32rem] overflow-y-auto rounded-lg border border-stitch-border">
          <table className="w-full text-left text-xs">
            <thead className="sticky top-0 bg-stitch-elevated text-stitch-muted">
              <tr>
                <th className="px-3 py-2">Reference</th>
                <th className="px-3 py-2">Title</th>
                <th className="px-3 py-2 text-right">Comparison</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-stitch-border">
              {reqs.map((snapshot) => {
                const current = currentReqById.get(snapshot.id);
                const other = compareReqById.get(snapshot.id);
                const changedFromCurrent =
                  current?.current_version_id != null &&
                  snapshot.current_version_id !== current.current_version_id;
                const changedBetweenBaselines =
                  compareBaselineId !== '' &&
                  other?.current_version_id != null &&
                  snapshot.current_version_id != null &&
                  other.current_version_id !== snapshot.current_version_id;
                return (
                  <tr key={snapshot.id} className="hover:bg-white/[0.03]">
                    <td className="px-3 py-2 font-mono text-stitch-accent">
                      {snapshot.reference_code || `#${snapshot.id}`}
                    </td>
                    <td className="px-3 py-2 text-stitch-fg">{snapshot.title}</td>
                    <td className="px-3 py-2">
                      <div className="flex flex-wrap justify-end gap-2">
                        {changedFromCurrent ? (
                          <button
                            type="button"
                            onClick={() =>
                              setReqDiff({
                                requirementId: snapshot.id,
                                title: `${snapshot.reference_code || `#${snapshot.id}`} — baseline vs current`,
                                load: () =>
                                  compareBaselineRequirementWithCurrent(pid, bid, snapshot.id),
                              })
                            }
                            className="font-bold text-stitch-accent hover:underline"
                          >
                            Diff vs current
                          </button>
                        ) : (
                          <span className="text-stitch-muted">Current unchanged</span>
                        )}
                        {changedBetweenBaselines && other?.current_version_id != null && snapshot.current_version_id != null ? (
                          <button
                            type="button"
                            onClick={() => {
                              const otherBaseline = baselines.find(
                                (baseline) => baseline.id === compareBaselineId,
                              );
                              const leftIsOlder =
                                (otherBaseline?.created_at ?? '') < (meta.created_at ?? '');
                              const oldVersionId = leftIsOlder
                                ? other.current_version_id!
                                : snapshot.current_version_id!;
                              const newVersionId = leftIsOlder
                                ? snapshot.current_version_id!
                                : other.current_version_id!;
                              setReqDiff({
                                requirementId: snapshot.id,
                                title: `${snapshot.reference_code || `#${snapshot.id}`} — baseline comparison`,
                                load: () =>
                                  compareRequirementVersionsByProject(
                                    pid,
                                    snapshot.id,
                                    oldVersionId,
                                    newVersionId,
                                  ),
                              });
                            }}
                            className="font-bold text-stitch-accent hover:underline"
                          >
                            Diff baselines
                          </button>
                        ) : null}
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </section>

      <section className="mb-8 rounded-xl border border-stitch-border bg-stitch-surface p-5">
        <h3 className="text-sm font-bold uppercase tracking-widest text-stitch-fg">
          Verification snapshots
        </h3>
        <p className="mb-4 mt-1 text-xs text-stitch-muted">
          Compare the verification captured by this baseline against its current definition and status.
        </p>
        <div className="max-h-[28rem] overflow-y-auto rounded-lg border border-stitch-border">
          <table className="w-full text-left text-xs">
            <thead className="sticky top-0 bg-stitch-elevated text-stitch-muted">
              <tr>
                <th className="px-3 py-2">Reference</th>
                <th className="px-3 py-2">Baseline status/type</th>
                <th className="px-3 py-2">Current status/type</th>
                <th className="px-3 py-2 text-right">Comparison</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-stitch-border">
              {vers.map((snapshot) => {
                const current = currentVerById.get(snapshot.verification_id);
                const changed =
                  current != null &&
                  (snapshot.name !== current.name ||
                    snapshot.description !== current.description ||
                    snapshot.source !== current.source ||
                    snapshot.reference_code !== current.reference_code ||
                    snapshot.status_id !== current.status_id ||
                    snapshot.parent_id !== current.parent_id ||
                    snapshot.verification_method_id !== current.verification_method_id);
                return (
                  <tr key={snapshot.verification_id} className="hover:bg-white/[0.03]">
                    <td className="px-3 py-2 font-mono text-stitch-accent">
                      {snapshot.reference_code || `#${snapshot.verification_id}`}
                    </td>
                    <td className="px-3 py-2 text-stitch-muted">
                      {statusById.get(snapshot.status_id) ?? `Status #${snapshot.status_id}`} ·{' '}
                      {snapshot.verification_method_id == null
                        ? '—'
                        : methodById.get(snapshot.verification_method_id) ??
                          `Method #${snapshot.verification_method_id}`}
                    </td>
                    <td className="px-3 py-2 text-stitch-fg">
                      {current
                        ? `${statusById.get(current.status_id) ?? `Status #${current.status_id}`} · ${
                            current.verification_method_id == null
                              ? '—'
                              : methodById.get(current.verification_method_id) ??
                                `Method #${current.verification_method_id}`
                          }`
                        : 'Deleted'}
                    </td>
                    <td className="px-3 py-2 text-right">
                      {changed ? (
                        <button
                          type="button"
                          onClick={() =>
                            setVerificationDiff({
                              verificationId: snapshot.verification_id,
                              title: `${snapshot.reference_code || `#${snapshot.verification_id}`} — baseline vs current`,
                              load: () =>
                                compareBaselineVerificationWithCurrent(
                                  pid,
                                  bid,
                                  snapshot.verification_id,
                                ),
                            })
                          }
                          className="font-bold text-stitch-accent hover:underline"
                        >
                          Diff vs current
                        </button>
                      ) : (
                        <span className="text-stitch-muted">{current ? 'Unchanged' : 'Unavailable'}</span>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </section>

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

      {reqDiff ? (
        <AsyncDiffDialog
          open
          onClose={() => setReqDiff(null)}
          title={reqDiff.title}
          subtitle="Removed values are red; additions are green."
          load={reqDiff.load}
          render={(diff: RequirementDiff) => <RequirementDiffContent diff={diff} />}
        />
      ) : null}
      {verificationDiff ? (
        <AsyncDiffDialog
          open
          onClose={() => setVerificationDiff(null)}
          title={verificationDiff.title}
          subtitle="The frozen baseline snapshot is compared with the current verification."
          load={verificationDiff.load}
          render={(diff: VerificationVersionDiff) => <VerificationDiffContent diff={diff} />}
        />
      ) : null}
    </div>
  );
}

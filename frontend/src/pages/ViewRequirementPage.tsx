import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useOutletContext, useParams } from 'react-router-dom';
import RequirementCommentComposer, {
  commentsLockedForApproval,
} from '@/components/RequirementCommentComposer';
import { useDashboard } from '@/context/DashboardContext';
import {
  getMyPermissions,
  getRequirementByProject,
  getRequirementVersionByProject,
  listApplicability,
  listCategories,
  listRequirementActivityByProject,
  listRequirementComments,
  listRequirementStatuses,
  setRequirementVersionApproval,
  listRequirementVersionLinks,
  listRequirementVersionsByProject,
  listRequirements,
  listProjectMembers,
  listUsersOptional,
  listVerificationMethodsByProject,
  listVerificationStatuses,
  listVerifications,
} from '@/api/client';
import RequirementVersionDiffDialog from '@/components/RequirementVersionDiffDialog';
import { StatusBadge } from '@/components/StatusBadge';
import { formatUserLabel } from '@/utils/userLabel';
import type {
  Applicability,
  Category,
  EffectivePermissions,
  EntityActivityItem,
  Requirement,
  RequirementCommentItem,
  RequirementDetailPayload,
  RequirementStatus,
  RequirementVersion,
  RequirementVersionLink,
  ProjectMember,
  User,
  Verification,
  VerificationMethod,
  VerificationStatus,
} from '@/api/types';
import type { ProjectOutletContext } from '@/types/projectOutlet';

function approvalLabel(state: string): string {
  return state.replace(/_/g, ' ').toUpperCase();
}

function priorityFromCustomFields(
  fields: Requirement['custom_fields'] | undefined,
): string {
  if (!fields?.length) return '—';
  const p = fields.find((f) => f.label && /priority/i.test(f.label));
  return p?.value?.trim() || '—';
}

function formatTs(iso: string): string {
  try {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    return d.toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' });
  } catch {
    return iso;
  }
}

function formatActivityChangeValue(
  field: string,
  raw: string,
  statusById: Map<number, RequirementStatus>,
  categoryById: Map<number, string>,
  applicabilityById: Map<number, string>,
): string {
  const t = raw.trim();
  if (t === '—' || t === '') return t || '—';
  if (field === 'Status') {
    const id = Number(t);
    if (Number.isFinite(id)) return statusById.get(id)?.title ?? raw;
  }
  if (field === 'Category') {
    const id = Number(t);
    if (Number.isFinite(id)) return categoryById.get(id) ?? raw;
  }
  if (field === 'Applicability') {
    const id = Number(t);
    if (Number.isFinite(id)) return applicabilityById.get(id) ?? raw;
  }
  return raw;
}

function formatRelativeTime(iso: string): string {
  try {
    const d = new Date(iso);
    const ms = Date.now() - d.getTime();
    if (Number.isNaN(d.getTime()) || ms < 0) return '';
    const minutes = Math.floor(ms / 60000);
    if (minutes < 1) return 'just now';
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 7) return `${days}d ago`;
    return d.toLocaleDateString(undefined, { dateStyle: 'short' });
  } catch {
    return '';
  }
}

export default function ViewRequirementPage() {
  const { basePath, projectId: pid } = useOutletContext<ProjectOutletContext>();
  const { csrfToken } = useDashboard();
  const { requirementId: requirementIdParam, versionId: versionIdParam } = useParams();
  const rid = Number(requirementIdParam);
  const requestedVersionId = versionIdParam != null ? Number(versionIdParam) : NaN;
  const viewingVersionParam = Number.isFinite(requestedVersionId);

  const [detail, setDetail] = useState<RequirementDetailPayload | null>(null);
  const [snapshot, setSnapshot] = useState<RequirementVersion | null>(null);
  const [snapshotParents, setSnapshotParents] = useState<RequirementVersionLink[]>([]);
  const [methods, setMethods] = useState<VerificationMethod[]>([]);
  const [versions, setVersions] = useState<RequirementVersion[]>([]);
  const [comments, setComments] = useState<RequirementCommentItem[]>([]);
  const [statuses, setStatuses] = useState<RequirementStatus[]>([]);
  const [verifStatuses, setVerifStatuses] = useState<VerificationStatus[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [applicability, setApplicability] = useState<Applicability[]>([]);
  const [projectReqs, setProjectReqs] = useState<Requirement[]>([]);
  const [verifications, setVerifications] = useState<Verification[]>([]);
  const [users, setUsers] = useState<User[] | null>(null);
  const [members, setMembers] = useState<ProjectMember[]>([]);
  const [perms, setPerms] = useState<EffectivePermissions | null>(null);
  const [activityLog, setActivityLog] = useState<EntityActivityItem[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [approvalError, setApprovalError] = useState<string | null>(null);
  const [approvalBusy, setApprovalBusy] = useState(false);
  const [diffOpen, setDiffOpen] = useState(false);
  const [diffInitialPair, setDiffInitialPair] = useState<{
    oldVersionId?: number;
    newVersionId?: number;
  } | null>(null);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid) || !Number.isFinite(rid)) return;
    setLoadError(null);
    setApprovalError(null);
    try {
      const [
        d,
        v,
        st,
        vst,
        cat,
        app,
        reqs,
        ver,
        u,
        mem,
        p,
        act,
        methodList,
        snap,
        snapParents,
        cmts,
      ] = await Promise.all([
        getRequirementByProject(pid, rid),
        listRequirementVersionsByProject(pid, rid),
        listRequirementStatuses(),
        listVerificationStatuses(),
        listCategories(),
        listApplicability(),
        listRequirements(pid),
        listVerifications(),
        listUsersOptional(),
        listProjectMembers(pid),
        getMyPermissions(pid).catch(() => null),
        listRequirementActivityByProject(pid, rid).catch(() => [] as EntityActivityItem[]),
        listVerificationMethodsByProject(pid).catch(() => [] as VerificationMethod[]),
        viewingVersionParam
          ? getRequirementVersionByProject(pid, rid, requestedVersionId)
          : Promise.resolve(null),
        viewingVersionParam
          ? listRequirementVersionLinks(pid, { source_version_id: requestedVersionId })
          : Promise.resolve([] as RequirementVersionLink[]),
        listRequirementComments(rid, viewingVersionParam ? requestedVersionId : undefined),
      ]);
      if (d.project_id !== pid) {
        setLoadError('This requirement belongs to another project.');
        return;
      }
      setDetail(d);
      setSnapshot(snap);
      setSnapshotParents(snapParents);
      setVersions(v);
      setStatuses(st);
      setVerifStatuses(vst);
      setCategories(cat.filter((c) => c.project_id === pid));
      setApplicability(app.filter((a) => a.project_id === pid));
      setProjectReqs(reqs);
      setVerifications(ver.filter((x) => x.project_id === pid));
      setUsers(u);
      setMembers(mem);
      setComments(cmts);
      setPerms(p);
      setActivityLog(act);
      setMethods(methodList);
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : 'Failed to load requirement');
    }
  }, [pid, rid, requestedVersionId, viewingVersionParam]);

  useEffect(() => {
    void load();
  }, [load]);

  const userLabel = useCallback(
    (id: number) => formatUserLabel(id, { users, members }),
    [users, members],
  );

  const statusById = useMemo(() => {
    const m = new Map<number, RequirementStatus>();
    for (const s of statuses) m.set(s.id, s);
    return m;
  }, [statuses]);

  const categoryById = useMemo(() => {
    const m = new Map<number, string>();
    for (const c of categories) m.set(c.id, c.title);
    return m;
  }, [categories]);

  const applicabilityById = useMemo(() => {
    const m = new Map<number, string>();
    for (const a of applicability) m.set(a.id, a.title);
    return m;
  }, [applicability]);

  const verById = useMemo(() => {
    const m = new Map<number, Verification>();
    for (const v of verifications) m.set(v.id, v);
    return m;
  }, [verifications]);

  const verifStatusById = useMemo(() => {
    const m = new Map<number, VerificationStatus>();
    for (const s of verifStatuses) m.set(s.id, s);
    return m;
  }, [verifStatuses]);

  const reqTitleById = useMemo(() => {
    const m = new Map<number, string>();
    for (const r of projectReqs) m.set(r.id, r.title);
    return m;
  }, [projectReqs]);

  const reqById = useMemo(() => {
    const m = new Map<number, Requirement>();
    for (const r of projectReqs) m.set(r.id, r);
    return m;
  }, [projectReqs]);

  const versionIdToReqId = useMemo(() => {
    const m = new Map<number, number>();
    for (const r of projectReqs) {
      if (r.current_version_id != null) {
        m.set(r.current_version_id, r.id);
      }
    }
    return m;
  }, [projectReqs]);

  const resolveParentReq = useCallback(
    (link: RequirementVersionLink) => {
      const prid = versionIdToReqId.get(link.target_version_id);
      if (prid == null) return null;
      return reqById.get(prid) ?? null;
    },
    [versionIdToReqId, reqById],
  );

  const versionsNewestFirst = useMemo(
    () =>
      [...versions].sort(
        (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
      ),
    [versions],
  );
  const latestVersionLabel = versions.length > 0 ? `v${versions.length}` : '—';
  const lastApprovedVersion = useMemo(
    () =>
      versionsNewestFirst.find(
        (version) =>
          version.approval_state.toLowerCase() === 'approved' &&
          version.id !== detail?.current_version_id,
      ) ?? null,
    [detail?.current_version_id, versionsNewestFirst],
  );

  const changelogEntries = useMemo(() => {
    return versionsNewestFirst.map((ver, i) => {
      const older = versionsNewestFirst[i + 1];
      const revNum = versionsNewestFirst.length - i;
      const changes: string[] = [];
      if (older) {
        if (older.title !== ver.title) changes.push('Title');
        if (older.description !== ver.description) changes.push('Statement');
        if (older.status_id !== ver.status_id) changes.push('Status');
        if (older.approval_state !== ver.approval_state) changes.push('Approval');
        if (older.author_id !== ver.author_id) changes.push('Author');
        if (older.reviewer_id !== ver.reviewer_id) changes.push('Reviewer');
        if (older.category_id !== ver.category_id) changes.push('Category');
        if (older.applicability_id !== ver.applicability_id) changes.push('Applicability');
        if ((older.justification ?? '') !== (ver.justification ?? '')) changes.push('Rationale');
        if ((older.deadline_date ?? '') !== (ver.deadline_date ?? '')) changes.push('Deadline');
      }
      return { ver, revNum, older, changes, isLatest: i === 0 };
    });
  }, [versionsNewestFirst]);

  const isHistorical =
    viewingVersionParam &&
    snapshot != null &&
    detail != null &&
    snapshot.id !== detail.current_version_id;

  const snapshotRevNum = useMemo(() => {
    if (!snapshot) return null;
    const i = versionsNewestFirst.findIndex((v) => v.id === snapshot.id);
    if (i < 0) return null;
    return versionsNewestFirst.length - i;
  }, [snapshot, versionsNewestFirst]);

  const methodById = useMemo(() => {
    const m = new Map<number, string>();
    for (const x of methods) m.set(x.id, x.title);
    return m;
  }, [methods]);

  const view = useMemo(() => {
    if (!detail) return null;
    if (isHistorical && snapshot) {
      return {
        title: snapshot.title,
        description: snapshot.description,
        justification: snapshot.justification,
        status_id: snapshot.status_id,
        author_id: snapshot.author_id,
        reviewer_id: snapshot.reviewer_id,
        category_id: snapshot.category_id,
        applicability_id: snapshot.applicability_id,
        approval_state: snapshot.approval_state,
        approved_by: snapshot.approved_by,
        approved_at: snapshot.approved_at,
        custom_fields: snapshot.custom_fields,
        verification_method_ids: snapshot.verification_method_ids ?? [],
        update_date: snapshot.created_at,
        parent_links: snapshotParents,
        versionLabel: snapshotRevNum != null ? `v${snapshotRevNum}` : `version #${snapshot.id}`,
      };
    }
    return {
      title: detail.title,
      description: detail.description,
      justification: detail.justification,
      status_id: detail.status_id,
      author_id: detail.author_id,
      reviewer_id: detail.reviewer_id,
      category_id: detail.category_id,
      applicability_id: detail.applicability_id,
      approval_state: detail.approval_state,
      approved_by: detail.approved_by,
      approved_at: detail.approved_at,
      custom_fields: detail.custom_fields,
      verification_method_ids: detail.verification_method_ids ?? [],
      update_date: detail.update_date,
      parent_links: detail.trace_summary.parent_links,
      versionLabel: latestVersionLabel,
    };
  }, [
    detail,
    isHistorical,
    snapshot,
    snapshotParents,
    snapshotRevNum,
    latestVersionLabel,
  ]);

  const canMutate = Boolean(perms?.edit_requirements) && !isHistorical;
  const currentApproval = (view?.approval_state ?? '').toLowerCase();
  const canChangeApproval =
    !isHistorical && Boolean(perms?.is_project_reviewer) && detail?.current_version_id != null;

  async function setApproval(state: 'reviewed' | 'approved') {
    const versionId = detail?.current_version_id;
    const token = csrfToken ?? '';
    if (versionId == null || !token) {
      setApprovalError('Missing CSRF token; refresh the page.');
      return;
    }
    const ok = window.confirm(
      state === 'reviewed'
        ? 'Mark this requirement version as reviewed?'
        : 'Approve this requirement version? Comments on approved versions may be locked.',
    );
    if (!ok) return;
    setApprovalBusy(true);
    setApprovalError(null);
    try {
      await setRequirementVersionApproval(pid, rid, versionId, state, token);
      await load();
    } catch (e) {
      setApprovalError(e instanceof Error ? e.message : 'Failed to update approval');
    } finally {
      setApprovalBusy(false);
    }
  }

  const openVersionDiff = (pair?: { oldVersionId: number; newVersionId: number }) => {
    setDiffInitialPair(pair ?? null);
    setDiffOpen(true);
  };

  if (loadError) {
    return (
      <div className="rounded-xl border border-red-500/40 bg-red-500/10 p-4 text-sm text-red-800 dark:text-red-100">
        {loadError}
        <div className="mt-3">
          <Link to={`${basePath}/requirements`} className="font-semibold text-stitch-accent underline">
            Back to requirements
          </Link>
        </div>
      </div>
    );
  }

  if (!detail || !view) {
    return (
      <div className="text-stitch-muted text-sm py-12 text-center bg-stitch-canvas rounded-lg">
        Loading requirement…
      </div>
    );
  }

  const st = statusById.get(view.status_id);
  const ts = detail.trace_summary;
  const parentLinks = view.parent_links;

  return (
    <div className="max-w-7xl mx-auto pb-12">
      <nav className="flex flex-wrap items-center justify-between gap-3 text-xs font-semibold text-stitch-muted mb-6 uppercase tracking-widest">
        <div className="flex items-center gap-2 min-w-0">
          <Link to={`${basePath}/requirements`} className="hover:text-stitch-accent transition-colors shrink-0">
            Requirements
          </Link>
          <span className="material-symbols-outlined text-sm">chevron_right</span>
          <span className="text-stitch-accent font-bold font-headline truncate">
            {detail.reference_code || `REQ-${detail.id}`}
          </span>
          <span className="text-stitch-muted font-normal normal-case tracking-normal">
            {isHistorical ? '· Historical snapshot' : '· View'}
          </span>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            disabled={versions.length < 2}
            onClick={() => openVersionDiff()}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-stitch-border text-stitch-muted hover:text-stitch-accent hover:border-stitch-accent/40 text-[10px] font-bold uppercase tracking-wider transition-colors disabled:cursor-not-allowed disabled:opacity-40"
          >
            <span className="material-symbols-outlined text-sm">difference</span>
            Compare versions
          </button>
          {canMutate ? (
            <>
              <Link
                to={`${basePath}/requirements/new?from=${rid}`}
                className="inline-flex items-center gap-1.5 rounded-md border border-stitch-border px-3 py-1.5 text-[10px] font-bold uppercase tracking-wider text-stitch-muted transition-colors hover:border-stitch-accent/40 hover:text-stitch-accent"
              >
                <span className="material-symbols-outlined text-sm">content_copy</span>
                Duplicate
              </Link>
              <Link
                to={`${basePath}/requirements/${rid}/edit`}
                className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-gradient-to-br from-[#000666] to-[#1a237e] text-white text-[10px] font-bold uppercase tracking-wider shadow-lg hover:opacity-95 transition-opacity"
              >
                <span className="material-symbols-outlined text-sm">edit</span>
                Edit
              </Link>
            </>
          ) : null}
        </div>
      </nav>

      {isHistorical ? (
        <div
          role="status"
          className="mb-6 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-amber-500/40 bg-amber-500/10 px-4 py-3 text-sm text-stitch-fg"
        >
          <p>
            Viewing historical snapshot <span className="font-mono font-bold">{view.versionLabel}</span>
            {' · '}not the current version.
          </p>
          <Link
            to={`${basePath}/requirements/${rid}`}
            className="text-[10px] font-bold uppercase tracking-wider text-stitch-accent hover:underline"
          >
            View current
          </Link>
        </div>
      ) : null}

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-8">
        <div className="lg:col-span-8 space-y-8">
          <section className="bg-stitch-surface p-6 md:p-8 rounded-xl border border-stitch-border shadow-stitch">
            <div className="flex flex-wrap items-center gap-3 mb-4">
              <span className="font-mono text-xs font-bold text-stitch-muted bg-stitch-elevated px-2 py-1 rounded border border-stitch-border">
                {detail.reference_code || `#${detail.id}`}
              </span>
              <span className="text-xs font-medium text-stitch-accent-dim bg-stitch-elevated px-2 py-1 rounded border border-stitch-border uppercase tracking-wide">
                {approvalLabel(view.approval_state)}
              </span>
              {view.approved_at ? (
                <span className="text-[10px] text-stitch-muted">
                  Approved {formatTs(view.approved_at)}
                  {view.approved_by != null ? ` · ${userLabel(view.approved_by)}` : ''}
                </span>
              ) : null}
              {canChangeApproval && currentApproval === 'draft' ? (
                <button
                  type="button"
                  disabled={approvalBusy || !(csrfToken ?? '').length}
                  onClick={() => void setApproval('reviewed')}
                  className="text-[10px] font-bold uppercase tracking-wider text-stitch-accent hover:underline disabled:opacity-40"
                >
                  {approvalBusy ? 'Updating…' : 'Mark as Reviewed'}
                </button>
              ) : null}
              {canChangeApproval && currentApproval === 'reviewed' ? (
                <button
                  type="button"
                  disabled={approvalBusy || !(csrfToken ?? '').length}
                  onClick={() => void setApproval('approved')}
                  className="text-[10px] font-bold uppercase tracking-wider text-stitch-accent hover:underline disabled:opacity-40"
                >
                  {approvalBusy ? 'Updating…' : 'Approve Requirement'}
                </button>
              ) : null}
              {!isHistorical && lastApprovedVersion && detail.current_version_id != null ? (
                <button
                  type="button"
                  onClick={() =>
                    openVersionDiff({
                      oldVersionId: lastApprovedVersion.id,
                      newVersionId: detail.current_version_id!,
                    })
                  }
                  className="text-[10px] font-bold uppercase tracking-wider text-stitch-accent hover:underline"
                >
                  Compare with last approved
                </button>
              ) : null}
              {st ? <StatusBadge title={st.title} tagColor={st.tag_color} /> : null}
            </div>
            {approvalError ? (
              <p role="alert" className="mb-4 text-sm text-red-200">
                {approvalError}
              </p>
            ) : null}
            <h1 className="text-2xl md:text-3xl font-bold font-headline text-stitch-fg mb-6">
              {view.title.trim() || '—'}
            </h1>

            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 p-4 bg-stitch-elevated rounded-lg border border-stitch-border text-sm">
              <div>
                <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                  Priority
                </span>
                <span className="font-semibold text-stitch-fg">{priorityFromCustomFields(view.custom_fields)}</span>
              </div>
              <div>
                <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                  Version
                </span>
                <span className="font-mono font-medium text-stitch-fg">{view.versionLabel}</span>
              </div>
              <div>
                <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                  Author
                </span>
                <span className="font-semibold text-stitch-fg line-clamp-2">{userLabel(view.author_id)}</span>
              </div>
              <div>
                <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                  Reviewer
                </span>
                <span className="font-semibold text-stitch-fg line-clamp-2">{userLabel(view.reviewer_id)}</span>
              </div>
            </div>

            <dl className="mt-6 grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm">
              <div>
                <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">Category</dt>
                <dd className="text-stitch-fg">{categoryById.get(view.category_id) ?? `Category #${view.category_id}`}</dd>
              </div>
              <div>
                <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">Applicability</dt>
                <dd className="text-stitch-fg">
                  {applicabilityById.get(view.applicability_id) ??
                    `Applicability #${view.applicability_id}`}
                </dd>
              </div>
              <div className="sm:col-span-2">
                <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                  Verification methods
                </dt>
                <dd className="text-stitch-fg">
                  {view.verification_method_ids.length === 0
                    ? '—'
                    : view.verification_method_ids
                        .map((id) => methodById.get(id) ?? `Method #${id}`)
                        .join(', ')}
                </dd>
              </div>
              <div className="sm:col-span-2">
                <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-2">Parents</dt>
                <dd>
                  {parentLinks.length === 0 ? (
                    <span className="text-stitch-muted text-sm">None</span>
                  ) : (
                    <ul className="space-y-2 list-none m-0 p-0">
                      {parentLinks.map((l) => {
                        const parentReq = resolveParentReq(l);
                        return (
                          <li
                            key={l.id}
                            className="flex flex-wrap items-baseline gap-x-2 gap-y-1 text-sm border border-stitch-border rounded-lg px-3 py-2 bg-stitch-elevated/80"
                          >
                            {parentReq ? (
                              <Link
                                to={`${basePath}/requirements/${parentReq.id}`}
                                className="font-mono font-semibold text-stitch-accent hover:underline min-w-0"
                                title={parentReq.title?.trim() || undefined}
                              >
                                {(parentReq.reference_code ?? '').trim() || `#${parentReq.id}`}
                              </Link>
                            ) : (
                              <span className="text-stitch-muted font-mono text-xs">
                                Version #{l.target_version_id}
                              </span>
                            )}
                            <span className="text-[10px] font-bold text-stitch-muted uppercase tracking-wide shrink-0">
                              {l.link_type}
                            </span>
                          </li>
                        );
                      })}
                    </ul>
                  )}
                </dd>
              </div>
              <div>
                <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">Modified</dt>
                <dd className="text-stitch-fg font-mono text-xs">{formatTs(view.update_date)}</dd>
              </div>
              <div>
                <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">Created</dt>
                <dd className="text-stitch-fg font-mono text-xs">{formatTs(detail.creation_date)}</dd>
              </div>
            </dl>
          </section>

          <section className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
            <div className="px-6 py-3 border-b border-stitch-border bg-stitch-elevated">
              <h2 className="text-xs font-bold uppercase tracking-widest text-stitch-muted font-headline">
                Requirement statement
              </h2>
            </div>
            <div className="p-6 md:p-8">
              <div className="text-sm leading-relaxed text-stitch-fg whitespace-pre-wrap">
                {view.description.trim() ? view.description : '—'}
              </div>
            </div>
          </section>

          <section className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
            <div className="px-6 py-3 border-b border-stitch-border bg-stitch-elevated">
              <h2 className="text-xs font-bold uppercase tracking-widest text-stitch-muted font-headline">
                Rationale
              </h2>
            </div>
            <div className="p-6 md:p-8">
              <div className="text-sm leading-relaxed text-stitch-fg whitespace-pre-wrap">
                {view.justification?.trim() ? view.justification : '—'}
              </div>
            </div>
          </section>

          {view.custom_fields && view.custom_fields.length > 0 ? (
            <section className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
              <div className="px-6 py-3 border-b border-stitch-border bg-stitch-elevated">
                <h2 className="text-xs font-bold uppercase tracking-widest text-stitch-muted font-headline">
                  Custom metadata
                </h2>
              </div>
              <dl className="p-6 md:p-8 grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm">
                {view.custom_fields.map((f) => (
                  <div key={f.field_id}>
                    <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                      {f.label}
                    </dt>
                    <dd className="text-stitch-fg">{f.value?.trim() || '—'}</dd>
                  </div>
                ))}
              </dl>
            </section>
          ) : null}
        </div>

        <aside className="lg:col-span-4 space-y-6">
          <div className="bg-stitch-surface rounded-xl border border-stitch-border p-6 shadow-stitch">
            <div className="flex items-center gap-2 mb-4">
              <span className="material-symbols-outlined text-stitch-accent text-xl">account_tree</span>
              <h2 className="text-sm font-bold font-headline text-stitch-accent">Traceability</h2>
            </div>
            <div className="space-y-6">
              <div>
                <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-2">Upstream (parents)</p>
                {parentLinks.length === 0 ? (
                  <p className="text-xs text-stitch-muted">None</p>
                ) : (
                  <ul className="space-y-2">
                    {parentLinks.map((l) => {
                      const parentReq = resolveParentReq(l);
                      return (
                        <li key={l.id}>
                          {parentReq ? (
                            <Link
                              to={`${basePath}/requirements/${parentReq.id}`}
                              className="block p-3 rounded-lg border border-stitch-border bg-stitch-elevated hover:bg-stitch-higher transition-colors"
                              title={parentReq.title?.trim() || undefined}
                            >
                              <span className="font-mono text-xs font-bold text-stitch-accent block">
                                {(parentReq.reference_code ?? '').trim() || `#${parentReq.id}`}
                              </span>
                              <span className="text-[10px] text-stitch-muted">{l.link_type}</span>
                            </Link>
                          ) : (
                            <div className="p-3 rounded-lg border border-stitch-border bg-stitch-elevated text-xs text-stitch-muted">
                              Version #{l.target_version_id} · {l.link_type}
                            </div>
                          )}
                        </li>
                      );
                    })}
                  </ul>
                )}
              </div>

              {!isHistorical && ts.child_ids.length > 0 ? (
                <div>
                  <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-2">
                    Child requirements
                  </p>
                  <ul className="space-y-2">
                    {ts.child_ids.map((cid) => (
                      <li key={cid}>
                        <Link
                          to={`${basePath}/requirements/${cid}`}
                          className="flex items-center justify-between p-3 rounded-lg border border-stitch-border bg-stitch-elevated hover:bg-stitch-higher transition-colors"
                        >
                          <div className="min-w-0">
                            <span className="font-mono text-[10px] font-bold text-stitch-muted">#{cid}</span>
                            <span className="block text-xs text-stitch-fg truncate">
                              {reqTitleById.get(cid) ?? '—'}
                            </span>
                          </div>
                          <span className="material-symbols-outlined text-stitch-muted text-sm">chevron_right</span>
                        </Link>
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}

              {!isHistorical ? (
              <div>
                <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-2">
                  Downstream (verifications)
                </p>
                {ts.linked_test_ids.length === 0 ? (
                  <p className="text-xs text-stitch-muted">None</p>
                ) : (
                  <ul className="space-y-2">
                    {ts.linked_test_ids.map((vid) => {
                      const v = verById.get(vid);
                      const vst = v ? verifStatusById.get(v.status_id) : undefined;
                      const borderColor = vst?.tag_color || undefined;
                      return (
                        <li key={vid}>
                          <Link
                            to={`${basePath}/verifications/${vid}`}
                            className="flex items-center justify-between gap-2 p-3 rounded-lg border border-stitch-border border-l-2 bg-stitch-elevated hover:bg-stitch-higher transition-colors"
                            style={borderColor ? { borderLeftColor: borderColor } : undefined}
                          >
                            <div className="min-w-0 flex-1">
                              <span
                                className="font-mono text-[10px] font-bold text-stitch-accent block"
                                style={borderColor ? { color: borderColor } : undefined}
                              >
                                {v?.reference_code ?? `VER-${vid}`}
                              </span>
                              <span className="text-xs text-stitch-fg truncate block">{v?.name ?? '—'}</span>
                            </div>
                            {vst?.title ? (
                              <StatusBadge title={vst.title} tagColor={vst.tag_color} />
                            ) : null}
                          </Link>
                        </li>
                      );
                    })}
                  </ul>
                )}
              </div>
              ) : (
                <p className="text-xs text-stitch-muted">
                  Child requirements and linked tests are not frozen on this snapshot. Open the current version to see live traceability.
                </p>
              )}
            </div>
          </div>
        </aside>
      </div>

      <section className="mt-10 bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch overflow-hidden">
        <div className="px-6 py-3 border-b border-stitch-border bg-stitch-elevated">
          <div className="flex items-center gap-2">
            <span className="material-symbols-outlined text-stitch-accent text-xl">forum</span>
            <h2 className="text-sm font-bold font-headline text-stitch-accent">Discussion</h2>
          </div>
          <p className="text-[10px] text-stitch-muted mt-1">
            Comments are tied to requirement versions. Newest activity first.
          </p>
        </div>
        <div className="p-4 md:p-6 max-h-[min(400px,45vh)] overflow-y-auto space-y-4">
          {comments.length === 0 ? (
            <p className="text-xs text-stitch-muted">No comments yet.</p>
          ) : (
            [...comments]
              .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
              .map((c) => (
                <div key={c.id} className="flex gap-3">
                  <div className="h-9 w-9 rounded-full bg-stitch-accent flex items-center justify-center text-[11px] text-stitch-on-accent font-bold shrink-0">
                    {c.author_name
                      ? c.author_name
                          .split(/\s+/)
                          .map((p) => p[0])
                          .join('')
                          .slice(0, 2)
                          .toUpperCase()
                      : '?'}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2 mb-0.5">
                      <span className="text-sm font-bold text-stitch-fg">{c.author_name}</span>
                      <span className="text-[10px] text-stitch-muted">
                        {formatRelativeTime(c.created_at) || formatTs(c.created_at)}
                      </span>
                      {c.requirement_version_id != null ? (
                        <span className="text-[10px] font-mono text-stitch-muted">
                          · version #{c.requirement_version_id}
                        </span>
                      ) : null}
                    </div>
                    <p className="text-sm text-stitch-fg/90 leading-relaxed whitespace-pre-wrap">{c.body}</p>
                  </div>
                </div>
              ))
          )}
        </div>
        {isHistorical ? null : (
          <div className="px-4 py-3 border-t border-stitch-border bg-stitch-elevated">
            <RequirementCommentComposer
              requirementId={rid}
              versionId={detail.current_version_id}
              csrfToken={csrfToken}
              locked={commentsLockedForApproval(view.approval_state)}
              onPosted={(comment) => setComments((prev) => [comment, ...prev])}
            />
          </div>
        )}
      </section>

      <section className="mt-8 bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch overflow-hidden">
        <div className="px-6 py-3 border-b border-stitch-border bg-stitch-elevated flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-2 min-w-0">
            <span className="material-symbols-outlined text-stitch-accent text-xl shrink-0">history</span>
            <div>
              <h2 className="text-sm font-bold font-headline text-stitch-accent">Changelog</h2>
              <p className="text-[10px] text-stitch-muted mt-0.5">
                Version history and saved snapshots, plus the audit log of create/update actions (newest first).
              </p>
            </div>
          </div>
          <button
            type="button"
            disabled={versions.length < 2}
            onClick={() => openVersionDiff()}
            className="text-[10px] font-bold uppercase tracking-wide text-stitch-accent hover:underline shrink-0 disabled:cursor-not-allowed disabled:opacity-40"
          >
            Compare versions →
          </button>
        </div>
        <div className="p-4 md:p-6 max-h-[min(640px,70vh)] overflow-y-auto space-y-8">
          <div>
            <h3 className="text-[10px] font-bold uppercase tracking-widest text-stitch-muted mb-3">
              Version snapshots
            </h3>
            {versions.length === 0 ? (
              <p className="text-xs text-stitch-muted">No version snapshots yet.</p>
            ) : (
            <ul className="space-y-0 divide-y divide-stitch-border">
              {changelogEntries.map(({ ver, revNum, changes, isLatest, older }) => {
                const vstRow = statusById.get(ver.status_id);
                const isOpenSnapshot = viewingVersionParam && ver.id === requestedVersionId;
                const snapshotHref = `${basePath}/requirements/${rid}/versions/${ver.id}`;
                return (
                  <li
                    key={ver.id}
                    className={`py-4 first:pt-0 ${isOpenSnapshot ? 'bg-stitch-elevated/60 -mx-2 px-2 rounded-lg' : ''}`}
                  >
                    <div className="flex flex-wrap items-start justify-between gap-3 mb-2">
                      <div className="flex flex-wrap items-center gap-2">
                        <Link
                          to={snapshotHref}
                          className="font-mono text-xs font-bold text-stitch-accent bg-stitch-elevated px-2 py-0.5 rounded border border-stitch-border hover:border-stitch-accent/50"
                        >
                          v{revNum}
                        </Link>
                        {isLatest ? (
                          <span className="text-[10px] font-bold uppercase tracking-wider text-emerald-700 dark:text-emerald-400 bg-emerald-500/15 px-2 py-0.5 rounded">
                            Latest
                          </span>
                        ) : null}
                        <span className="text-[10px] text-stitch-muted font-mono">{formatTs(ver.created_at)}</span>
                      </div>
                      <div className="flex flex-wrap items-center gap-2">
                        {vstRow ? <StatusBadge title={vstRow.title} tagColor={vstRow.tag_color} /> : null}
                        <span className="text-[10px] font-semibold text-stitch-muted uppercase tracking-wide">
                          {approvalLabel(ver.approval_state)}
                        </span>
                      </div>
                    </div>
                    <p className="text-sm font-semibold text-stitch-fg mb-1 line-clamp-2">{ver.title.trim() || '—'}</p>
                    <p className="text-[10px] text-stitch-muted mb-2">
                      Author {userLabel(ver.author_id)}
                      {ver.approved_at ? (
                        <>
                          {' · '}
                          Approved {formatTs(ver.approved_at)}
                          {ver.approved_by != null ? ` · ${userLabel(ver.approved_by)}` : ''}
                        </>
                      ) : null}
                    </p>
                    {changes.length > 0 ? (
                      <div className="flex flex-wrap gap-1.5 mt-2">
                        <span className="text-[10px] text-stitch-muted font-bold uppercase tracking-wider mr-1">
                          Changed vs previous:
                        </span>
                        {changes.map((c) => (
                          <span
                            key={c}
                            className="text-[10px] font-semibold px-2 py-0.5 rounded-md bg-stitch-elevated border border-stitch-border text-stitch-fg"
                          >
                            {c}
                          </span>
                        ))}
                      </div>
                    ) : older == null && versions.length === 1 ? (
                      <p className="text-[10px] text-stitch-muted mt-1">Initial snapshot.</p>
                    ) : older == null ? (
                      <p className="text-[10px] text-stitch-muted mt-1">Earliest snapshot in history.</p>
                    ) : (
                      <p className="text-[10px] text-stitch-muted mt-1">No field changes vs the next newer snapshot.</p>
                    )}
                    {older ? (
                      <button
                        type="button"
                        onClick={() =>
                          openVersionDiff({
                            oldVersionId: older.id,
                            newVersionId: ver.id,
                          })
                        }
                        className="mt-3 inline-flex items-center gap-1 text-[10px] font-bold uppercase tracking-wider text-stitch-accent hover:underline"
                      >
                        <span className="material-symbols-outlined text-sm">difference</span>
                        Compare with previous
                      </button>
                    ) : null}
                  </li>
                );
              })}
            </ul>
            )}
          </div>

          <div className="border-t border-stitch-border pt-6">
            <h3 className="text-[10px] font-bold uppercase tracking-widest text-stitch-muted mb-3">
              Historic activity (audit)
            </h3>
            <p className="text-[10px] text-stitch-muted mb-4">
              Field-level changes recorded when this requirement is created or updated through the API.
            </p>
            {activityLog.length === 0 ? (
              <p className="text-xs text-stitch-muted">No audit entries yet.</p>
            ) : (
              <ul className="space-y-0 divide-y divide-stitch-border">
                {activityLog.map((entry) => (
                  <li key={entry.log_id} className="py-4 first:pt-0">
                    <div className="flex flex-wrap items-baseline justify-between gap-2 mb-1">
                      <span className="text-sm font-semibold text-stitch-fg">{entry.summary}</span>
                      <span className="text-[10px] font-mono text-stitch-muted">
                        {formatTs(entry.created_at)}
                        {formatRelativeTime(entry.created_at) ? ` · ${formatRelativeTime(entry.created_at)}` : ''}
                      </span>
                    </div>
                    <p className="text-[10px] text-stitch-muted mb-2">
                      <span className="font-semibold text-stitch-fg/90">{entry.username}</span>
                      <span className="mx-1">·</span>
                      <span className="uppercase tracking-wide">{entry.action_type}</span>
                    </p>
                    {entry.description ? (
                      <p className="text-xs text-stitch-fg/80 mb-2 whitespace-pre-wrap">{entry.description}</p>
                    ) : null}
                    {entry.changes.length > 0 ? (
                      <ul className="mt-2 space-y-1.5 text-[11px]">
                        {entry.changes.map((ch, idx) => (
                          <li
                            key={`${entry.log_id}-${idx}-${ch.field}`}
                            className="grid grid-cols-1 sm:grid-cols-3 gap-1 sm:gap-2 text-stitch-fg/90"
                          >
                            <span className="font-bold text-stitch-muted">{ch.field}</span>
                            <span className="text-stitch-muted line-clamp-3 sm:col-span-2">
                              <span className="text-red-300/90 line-through decoration-stitch-border">
                                {formatActivityChangeValue(
                                  ch.field,
                                  ch.old_value,
                                  statusById,
                                  categoryById,
                                  applicabilityById,
                                )}
                              </span>
                              <span className="mx-1 text-stitch-border">→</span>
                              <span className="text-emerald-200/90">
                                {formatActivityChangeValue(
                                  ch.field,
                                  ch.new_value,
                                  statusById,
                                  categoryById,
                                  applicabilityById,
                                )}
                              </span>
                            </span>
                          </li>
                        ))}
                      </ul>
                    ) : null}
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      </section>
      <RequirementVersionDiffDialog
        open={diffOpen}
        onClose={() => setDiffOpen(false)}
        projectId={pid}
        requirementId={rid}
        versions={versions}
        initialPair={diffInitialPair}
      />
    </div>
  );
}

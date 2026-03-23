import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import {
  getMyPermissions,
  getRequirementByProject,
  listApplicability,
  listCategories,
  listRequirementComments,
  listRequirementStatuses,
  listRequirementVersionsByProject,
  listRequirements,
  listUsersOptional,
  listVerificationStatuses,
  listVerifications,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import { StatusBadge } from '@/components/StatusBadge';
import type {
  Applicability,
  Category,
  EffectivePermissions,
  Requirement,
  RequirementCommentItem,
  RequirementDetailPayload,
  RequirementStatus,
  RequirementVersion,
  RequirementVersionLink,
  User,
  Verification,
  VerificationStatus,
} from '@/api/types';

function approvalLabel(state: string): string {
  return state.replace(/_/g, ' ').toUpperCase();
}

function priorityFromCustomFields(req: Requirement): string {
  const fields = req.custom_fields;
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
  const { projectId: projectIdParam, requirementId: requirementIdParam } = useParams();
  const pid = Number(projectIdParam);
  const rid = Number(requirementIdParam);
  const { dashboard } = useDashboard();

  const projectSlug = useMemo(
    () => dashboard?.projects?.find((p) => p.id === pid)?.slug,
    [dashboard?.projects, pid],
  );

  const [detail, setDetail] = useState<RequirementDetailPayload | null>(null);
  const [versions, setVersions] = useState<RequirementVersion[]>([]);
  const [comments, setComments] = useState<RequirementCommentItem[]>([]);
  const [statuses, setStatuses] = useState<RequirementStatus[]>([]);
  const [verifStatuses, setVerifStatuses] = useState<VerificationStatus[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [applicability, setApplicability] = useState<Applicability[]>([]);
  const [projectReqs, setProjectReqs] = useState<Requirement[]>([]);
  const [verifications, setVerifications] = useState<Verification[]>([]);
  const [users, setUsers] = useState<User[] | null>(null);
  const [perms, setPerms] = useState<EffectivePermissions | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid) || !Number.isFinite(rid)) return;
    setLoadError(null);
    try {
      const [d, v, st, vst, cat, app, reqs, ver, u, cmts, p] = await Promise.all([
        getRequirementByProject(pid, rid),
        listRequirementVersionsByProject(pid, rid),
        listRequirementStatuses(),
        listVerificationStatuses(),
        listCategories(),
        listApplicability(),
        listRequirements(pid),
        listVerifications(),
        listUsersOptional(),
        listRequirementComments(rid),
        getMyPermissions(pid).catch(() => null),
      ]);
      if (d.project_id !== pid) {
        setLoadError('This requirement belongs to another project.');
        return;
      }
      setDetail(d);
      setVersions(v);
      setStatuses(st);
      setVerifStatuses(vst);
      setCategories(cat.filter((c) => c.project_id === pid));
      setApplicability(app.filter((a) => a.project_id === pid));
      setProjectReqs(reqs);
      setVerifications(ver.filter((x) => x.project_id === pid));
      setUsers(u);
      setComments(cmts);
      setPerms(p);
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : 'Failed to load requirement');
    }
  }, [pid, rid]);

  useEffect(() => {
    void load();
  }, [load]);

  const userLabel = useCallback(
    (id: number) => {
      if (users?.length) {
        const u = users.find((x) => x.id === id);
        if (u) return `${u.name} (${u.username})`;
      }
      return `User #${id}`;
    },
    [users],
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
        if ((older.justification ?? '') !== (ver.justification ?? '')) changes.push('Justification');
        if ((older.deadline_date ?? '') !== (ver.deadline_date ?? '')) changes.push('Deadline');
      }
      return { ver, revNum, older, changes, isLatest: i === 0 };
    });
  }, [versionsNewestFirst]);

  const canEdit = Boolean(perms?.edit_requirements);

  if (loadError) {
    return (
      <div className="rounded-xl border border-red-500/40 bg-red-500/10 p-4 text-sm text-red-800 dark:text-red-100">
        {loadError}
        <div className="mt-3">
          <Link to={`/p/${pid}/requirements`} className="font-semibold text-stitch-accent underline">
            Back to requirements
          </Link>
        </div>
      </div>
    );
  }

  if (!detail) {
    return (
      <div className="text-stitch-muted text-sm py-12 text-center bg-stitch-canvas rounded-lg">
        Loading requirement…
      </div>
    );
  }

  const st = statusById.get(detail.status_id);
  const ts = detail.trace_summary;

  return (
    <div className="max-w-7xl mx-auto pb-12">
      <nav className="flex flex-wrap items-center justify-between gap-3 text-xs font-semibold text-stitch-muted mb-6 uppercase tracking-widest">
        <div className="flex items-center gap-2 min-w-0">
          <Link to={`/p/${pid}/requirements`} className="hover:text-stitch-accent transition-colors shrink-0">
            Requirements
          </Link>
          <span className="material-symbols-outlined text-sm">chevron_right</span>
          <span className="text-stitch-accent font-bold font-headline truncate">
            {detail.reference_code || `REQ-${detail.id}`}
          </span>
          <span className="text-stitch-muted font-normal normal-case tracking-normal">· View</span>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {projectSlug ? (
            <a
              href={`/p/${projectSlug}/requirements/show/${rid}`}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-stitch-border text-stitch-muted hover:text-stitch-accent hover:border-stitch-accent/40 text-[10px] font-bold uppercase tracking-wider transition-colors"
            >
              <span className="material-symbols-outlined text-sm">open_in_new</span>
              Classic
            </a>
          ) : null}
          {canEdit ? (
            <Link
              to={`/p/${pid}/requirements/${rid}/edit`}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-gradient-to-br from-[#000666] to-[#1a237e] text-white text-[10px] font-bold uppercase tracking-wider shadow-lg hover:opacity-95 transition-opacity"
            >
              <span className="material-symbols-outlined text-sm">edit</span>
              Edit
            </Link>
          ) : null}
        </div>
      </nav>

      <div className="grid grid-cols-1 lg:grid-cols-12 gap-8">
        <div className="lg:col-span-8 space-y-8">
          <section className="bg-stitch-surface p-6 md:p-8 rounded-xl border border-stitch-border shadow-stitch">
            <div className="flex flex-wrap items-center gap-3 mb-4">
              <span className="font-mono text-xs font-bold text-stitch-muted bg-stitch-elevated px-2 py-1 rounded border border-stitch-border">
                {detail.reference_code || `#${detail.id}`}
              </span>
              <span className="text-xs font-medium text-stitch-accent-dim bg-stitch-elevated px-2 py-1 rounded border border-stitch-border uppercase tracking-wide">
                {approvalLabel(detail.approval_state)}
              </span>
              {st ? <StatusBadge title={st.title} tagColor={st.tag_color} /> : null}
            </div>
            <h1 className="text-2xl md:text-3xl font-bold font-headline text-stitch-fg mb-6">
              {detail.title.trim() || '—'}
            </h1>

            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 p-4 bg-stitch-elevated rounded-lg border border-stitch-border text-sm">
              <div>
                <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                  Priority
                </span>
                <span className="font-semibold text-stitch-fg">{priorityFromCustomFields(detail)}</span>
              </div>
              <div>
                <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                  Version
                </span>
                <span className="font-mono font-medium text-stitch-fg">{latestVersionLabel}</span>
              </div>
              <div>
                <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                  Author
                </span>
                <span className="font-semibold text-stitch-fg line-clamp-2">{userLabel(detail.author_id)}</span>
              </div>
              <div>
                <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                  Reviewer
                </span>
                <span className="font-semibold text-stitch-fg line-clamp-2">{userLabel(detail.reviewer_id)}</span>
              </div>
            </div>

            <dl className="mt-6 grid grid-cols-1 sm:grid-cols-2 gap-4 text-sm">
              <div>
                <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">Category</dt>
                <dd className="text-stitch-fg">{categoryById.get(detail.category_id) ?? `Category #${detail.category_id}`}</dd>
              </div>
              <div>
                <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">Applicability</dt>
                <dd className="text-stitch-fg">
                  {applicabilityById.get(detail.applicability_id) ??
                    `Applicability #${detail.applicability_id}`}
                </dd>
              </div>
              <div className="sm:col-span-2">
                <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-2">Parents</dt>
                <dd>
                  {ts.parent_links.length === 0 ? (
                    <span className="text-stitch-muted text-sm">None</span>
                  ) : (
                    <ul className="space-y-2 list-none m-0 p-0">
                      {ts.parent_links.map((l) => {
                        const parentReq = resolveParentReq(l);
                        return (
                          <li
                            key={l.id}
                            className="flex flex-wrap items-baseline gap-x-2 gap-y-1 text-sm border border-stitch-border rounded-lg px-3 py-2 bg-stitch-elevated/80"
                          >
                            {parentReq ? (
                              <Link
                                to={`/p/${pid}/requirements/${parentReq.id}`}
                                className="font-mono font-semibold text-stitch-accent hover:underline shrink-0"
                              >
                                {parentReq.reference_code || `#${parentReq.id}`}
                              </Link>
                            ) : (
                              <span className="text-stitch-muted font-mono text-xs">
                                Version #{l.target_version_id}
                              </span>
                            )}
                            <span className="text-stitch-fg line-clamp-2 min-w-0">
                              {parentReq?.title?.trim() ? parentReq.title : '—'}
                            </span>
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
                <dd className="text-stitch-fg font-mono text-xs">{formatTs(detail.update_date)}</dd>
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
                {detail.description.trim() ? detail.description : '—'}
              </div>
            </div>
          </section>
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
                {ts.parent_links.length === 0 ? (
                  <p className="text-xs text-stitch-muted">None</p>
                ) : (
                  <ul className="space-y-2">
                    {ts.parent_links.map((l) => {
                      const parentReq = resolveParentReq(l);
                      return (
                        <li key={l.id}>
                          {parentReq ? (
                            <Link
                              to={`/p/${pid}/requirements/${parentReq.id}`}
                              className="block p-3 rounded-lg border border-stitch-border bg-stitch-elevated hover:bg-stitch-higher transition-colors"
                            >
                              <span className="font-mono text-[10px] font-bold text-stitch-accent block">
                                {parentReq.reference_code || `#${parentReq.id}`}
                              </span>
                              <span className="text-xs text-stitch-fg line-clamp-2">{parentReq.title}</span>
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

              {ts.child_ids.length > 0 ? (
                <div>
                  <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-2">
                    Child requirements
                  </p>
                  <ul className="space-y-2">
                    {ts.child_ids.map((cid) => (
                      <li key={cid}>
                        <Link
                          to={`/p/${pid}/requirements/${cid}`}
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
                            to={`/p/${pid}/verifications/${vid}`}
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
        {canEdit ? (
          <div className="px-4 py-3 border-t border-stitch-border bg-stitch-elevated">
            <Link
              to={`/p/${pid}/requirements/${rid}/edit`}
              className="text-xs font-bold text-stitch-accent hover:underline inline-flex items-center gap-1"
            >
              <span className="material-symbols-outlined text-sm">add_comment</span>
              Add a comment in the editor
            </Link>
          </div>
        ) : null}
      </section>

      <section className="mt-8 bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch overflow-hidden">
        <div className="px-6 py-3 border-b border-stitch-border bg-stitch-elevated flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-2 min-w-0">
            <span className="material-symbols-outlined text-stitch-accent text-xl shrink-0">history</span>
            <div>
              <h2 className="text-sm font-bold font-headline text-stitch-accent">Changelog</h2>
              <p className="text-[10px] text-stitch-muted mt-0.5">
                Requirement version snapshots (newest first). Compare fields between consecutive revisions.
              </p>
            </div>
          </div>
          {projectSlug ? (
            <a
              href={`/p/${projectSlug}/requirements/show/${rid}`}
              className="text-[10px] font-bold uppercase tracking-wide text-stitch-accent hover:underline shrink-0"
            >
              Full diffs in classic →
            </a>
          ) : null}
        </div>
        <div className="p-4 md:p-6 max-h-[min(520px,55vh)] overflow-y-auto">
          {versions.length === 0 ? (
            <p className="text-xs text-stitch-muted">No version snapshots yet.</p>
          ) : (
            <ul className="space-y-0 divide-y divide-stitch-border">
              {changelogEntries.map(({ ver, revNum, changes, isLatest, older }) => {
                const vstRow = statusById.get(ver.status_id);
                return (
                  <li key={ver.id} className="py-4 first:pt-0">
                    <div className="flex flex-wrap items-start justify-between gap-3 mb-2">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="font-mono text-xs font-bold text-stitch-accent bg-stitch-elevated px-2 py-0.5 rounded border border-stitch-border">
                          v{revNum}
                        </span>
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
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </section>
    </div>
  );
}

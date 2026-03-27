import { FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import {
  createRequirementComment,
  createRequirementVersionLink,
  deleteRequirementGlobally,
  deleteRequirementVersionLink,
  getRequirementByProject,
  listApplicability,
  listCategories,
  listProjectMembers,
  listRequirementComments,
  listRequirementStatuses,
  listRequirementVersionLinkTypes,
  listRequirementVersionsByProject,
  listRequirements,
  listUsersOptional,
  listVerificationStatuses,
  listVerifications,
  patchRequirementByProject,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type {
  Applicability,
  Category,
  ProjectMember,
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
import { StatusBadge, statusTagColorSwatchStyle } from '@/components/StatusBadge';

function approvalLabel(state: string): string {
  return state.replace(/_/g, ' ').toUpperCase();
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

export default function EditRequirementPage() {
  const { projectId: projectIdParam, requirementId: requirementIdParam } = useParams();
  const pid = Number(projectIdParam);
  const rid = Number(requirementIdParam);
  const navigate = useNavigate();
  const { csrfToken, dashboard, refresh: refreshDashboard } = useDashboard();

  const projectSlug = useMemo(
    () => dashboard?.projects?.find((p) => p.id === pid)?.slug,
    [dashboard?.projects, pid],
  );

  const [detail, setDetail] = useState<RequirementDetailPayload | null>(null);
  const [versions, setVersions] = useState<RequirementVersion[]>([]);
  const [statuses, setStatuses] = useState<RequirementStatus[]>([]);
  const [verifStatuses, setVerifStatuses] = useState<VerificationStatus[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [applicability, setApplicability] = useState<Applicability[]>([]);
  const [members, setMembers] = useState<ProjectMember[]>([]);
  const [users, setUsers] = useState<User[] | null>(null);
  const [projectReqs, setProjectReqs] = useState<Requirement[]>([]);
  const [verifications, setVerifications] = useState<Verification[]>([]);
  const [linkTypes, setLinkTypes] = useState<string[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [comments, setComments] = useState<RequirementCommentItem[]>([]);
  const [commentBody, setCommentBody] = useState('');
  const [commentPosting, setCommentPosting] = useState(false);
  const [deleteBusy, setDeleteBusy] = useState(false);
  const [linkBusy, setLinkBusy] = useState(false);
  const [newParentId, setNewParentId] = useState<number | ''>('');
  const [newLinkType, setNewLinkType] = useState('');

  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [statusId, setStatusId] = useState(0);
  const [categoryId, setCategoryId] = useState(0);
  const [applicabilityId, setApplicabilityId] = useState(0);
  const [authorId, setAuthorId] = useState(0);
  const [reviewerId, setReviewerId] = useState(0);

  const [baseline, setBaseline] = useState({
    title: '',
    description: '',
    status_id: 0,
    category_id: 0,
    applicability_id: 0,
    author_id: 0,
    reviewer_id: 0,
  });

  const load = useCallback(async () => {
    if (!Number.isFinite(pid) || !Number.isFinite(rid)) return;
    setLoadError(null);
    try {
      const [
        d,
        v,
        st,
        vst,
        cat,
        app,
        mem,
        reqs,
        ver,
        u,
        cmts,
        lt,
      ] = await Promise.all([
        getRequirementByProject(pid, rid),
        listRequirementVersionsByProject(pid, rid),
        listRequirementStatuses(),
        listVerificationStatuses(),
        listCategories(),
        listApplicability(),
        listProjectMembers(pid),
        listRequirements(pid),
        listVerifications(),
        listUsersOptional(),
        listRequirementComments(rid),
        listRequirementVersionLinkTypes(pid),
      ]);
      setDetail(d);
      setComments(cmts);
      setVersions(v);
      setStatuses(st);
      setVerifStatuses(vst);
      setCategories(cat.filter((c) => c.project_id === pid));
      setApplicability(app.filter((a) => a.project_id === pid));
      setMembers(mem);
      setProjectReqs(reqs);
      setVerifications(ver.filter((x) => x.project_id === pid));
      setUsers(u);
      setLinkTypes(lt);

      setTitle(d.title);
      setDescription(d.description);
      setStatusId(d.status_id);
      setCategoryId(d.category_id);
      setApplicabilityId(d.applicability_id);
      setAuthorId(d.author_id);
      setReviewerId(d.reviewer_id);
      setBaseline({
        title: d.title,
        description: d.description,
        status_id: d.status_id,
        category_id: d.category_id,
        applicability_id: d.applicability_id,
        author_id: d.author_id,
        reviewer_id: d.reviewer_id,
      });
      setNewLinkType((t) => (t && lt.includes(t) ? t : lt[0] ?? ''));
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

  const memberOptionIds = useMemo(() => {
    const ids = new Set(members.map((m) => m.user_id));
    ids.add(authorId);
    ids.add(reviewerId);
    return [...ids].sort((a, b) => a - b);
  }, [members, authorId, reviewerId]);

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

  const versionsNewestFirst = useMemo(
    () =>
      [...versions].sort(
        (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
      ),
    [versions],
  );
  const latestVersionCreatedAt = versionsNewestFirst[0]?.created_at;

  const parentCandidates = useMemo(
    () =>
      projectReqs.filter(
        (r) => r.id !== rid && r.current_version_id != null && r.current_version_id > 0,
      ),
    [projectReqs, rid],
  );

  const resolveParentReq = useCallback(
    (link: RequirementVersionLink) => {
      const prid = versionIdToReqId.get(link.target_version_id);
      if (prid == null) return null;
      return reqById.get(prid) ?? null;
    },
    [versionIdToReqId, reqById],
  );

  const dirty = useMemo(() => {
    return (
      title !== baseline.title ||
      description !== baseline.description ||
      statusId !== baseline.status_id ||
      categoryId !== baseline.category_id ||
      applicabilityId !== baseline.applicability_id ||
      authorId !== baseline.author_id ||
      reviewerId !== baseline.reviewer_id
    );
  }, [title, description, statusId, categoryId, applicabilityId, authorId, reviewerId, baseline]);

  function revert() {
    setTitle(baseline.title);
    setDescription(baseline.description);
    setStatusId(baseline.status_id);
    setCategoryId(baseline.category_id);
    setApplicabilityId(baseline.applicability_id);
    setAuthorId(baseline.author_id);
    setReviewerId(baseline.reviewer_id);
    setSaveError(null);
  }

  function formatTs(iso: string): string {
    try {
      const d = new Date(iso);
      if (Number.isNaN(d.getTime())) return iso;
      return d.toLocaleString(undefined, {
        dateStyle: 'short',
        timeStyle: 'short',
      });
    } catch {
      return iso;
    }
  }

  async function postComment() {
    const token = csrfToken ?? '';
    if (!token || !commentBody.trim()) return;
    setCommentPosting(true);
    setSaveError(null);
    try {
      const sorted = [...versions].sort(
        (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
      );
      const latestVid = sorted[0]?.id ?? null;
      const c = await createRequirementComment(
        rid,
        { body: commentBody.trim(), requirement_version_id: latestVid },
        token,
      );
      setComments((prev) => [c, ...prev]);
      setCommentBody('');
    } catch (e) {
      setSaveError(e instanceof Error ? e.message : 'Failed to post comment');
    } finally {
      setCommentPosting(false);
    }
  }

  async function deleteRequirement() {
    if (
      !window.confirm(
        'Delete this requirement permanently? Linked traceability and data may be affected. This cannot be undone.',
      )
    ) {
      return;
    }
    const token = csrfToken ?? '';
    if (!token) {
      setSaveError('Missing CSRF token; refresh the page.');
      return;
    }
    setDeleteBusy(true);
    setSaveError(null);
    try {
      await deleteRequirementGlobally(rid, token);
      await refreshDashboard();
      navigate(`/p/${pid}/requirements`);
    } catch (e) {
      setSaveError(e instanceof Error ? e.message : 'Delete failed');
    } finally {
      setDeleteBusy(false);
    }
  }

  async function onSave(e: FormEvent) {
    e.preventDefault();
    const token = csrfToken ?? '';
    if (!token) {
      setSaveError('Missing CSRF token; refresh the page.');
      return;
    }
    setSaveError(null);
    setSaving(true);
    try {
      await patchRequirementByProject(
        pid,
        rid,
        {
          title: title.trim(),
          description: description.trim(),
          status_id: statusId,
          category_id: categoryId,
          applicability_id: applicabilityId,
          author_id: authorId,
          reviewer_id: reviewerId,
        },
        token,
      );
      await refreshDashboard();
      await load();
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : 'Save failed');
    } finally {
      setSaving(false);
    }
  }

  async function removeParentLink(linkId: number) {
    const token = csrfToken ?? '';
    if (!token) {
      setSaveError('Missing CSRF token; refresh the page.');
      return;
    }
    setLinkBusy(true);
    setSaveError(null);
    try {
      await deleteRequirementVersionLink(pid, linkId, token);
      await load();
    } catch (e) {
      setSaveError(e instanceof Error ? e.message : 'Failed to remove parent link');
    } finally {
      setLinkBusy(false);
    }
  }

  async function addParentLink() {
    const token = csrfToken ?? '';
    if (!token || newParentId === '' || !newLinkType.trim()) {
      setSaveError('Pick a parent requirement and link type.');
      return;
    }
    const srcVid = detail?.current_version_id;
    if (srcVid == null) {
      setSaveError('This requirement has no current version; cannot add parent links.');
      return;
    }
    const parentReq = reqById.get(Number(newParentId));
    const tgtVid = parentReq?.current_version_id;
    if (parentReq == null || tgtVid == null) {
      setSaveError('Parent requirement must have a current version.');
      return;
    }
    setLinkBusy(true);
    setSaveError(null);
    try {
      await createRequirementVersionLink(
        pid,
        {
          source_version_id: srcVid,
          target_version_id: tgtVid,
          link_type: newLinkType.trim(),
        },
        token,
      );
      setNewParentId('');
      await load();
    } catch (e) {
      setSaveError(e instanceof Error ? e.message : 'Failed to add parent link');
    } finally {
      setLinkBusy(false);
    }
  }

  const statusOptions = useMemo(() => {
    const forProject = statuses.filter((s) => s.project_id === pid);
    return forProject.length > 0 ? forProject : statuses;
  }, [statuses, pid]);

  const priorityLabel = useMemo(() => {
    const fields = detail?.custom_fields;
    if (!fields?.length) return '—';
    const p = fields.find((f) => f.label && /priority/i.test(f.label));
    return p?.value?.trim() || '—';
  }, [detail?.custom_fields]);

  const statusTitle = useMemo(() => {
    const s = statuses.find((x) => x.id === statusId);
    return s?.title ?? '—';
  }, [statuses, statusId]);

  const statusMeta = useMemo(() => {
    const s = statuses.find((x) => x.id === statusId);
    return s;
  }, [statuses, statusId]);

  const selectStitch =
    'w-full text-sm font-medium bg-stitch-elevated border border-stitch-border rounded-lg px-3 py-2 text-stitch-fg focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/30 outline-none transition-colors';

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
      <div className="text-stitch-muted text-sm py-12 text-center font-body bg-stitch-elevated -mx-6 md:-mx-8 px-6 rounded-lg">
        Loading requirement…
      </div>
    );
  }

  const { trace_summary: ts } = detail;
  const latestVersionLabel = versions.length > 0 ? `v${versions.length}` : '—';
  const canEditParents = detail.current_version_id != null && detail.current_version_id > 0;

  return (
    <div className="font-body text-stitch-fg -mx-6 md:-mx-8 -mt-6 md:-mt-8 px-6 md:px-8 pt-6 md:pt-8 pb-36 min-h-full bg-stitch-canvas">
      <nav className="flex items-center gap-2 text-xs font-semibold text-stitch-muted mb-6 uppercase tracking-widest max-w-7xl mx-auto">
        <Link to={`/p/${pid}/requirements`} className="hover:text-stitch-accent transition-colors">
          Requirements
        </Link>
        <span className="material-symbols-outlined text-sm text-stitch-muted">chevron_right</span>
        <span className="text-stitch-accent font-bold font-headline">
          {detail.reference_code || `REQ-${detail.id}`}
        </span>
      </nav>

      <form onSubmit={onSave} className="max-w-7xl mx-auto">
        <div className="grid grid-cols-12 gap-8">
          {/* Main editor */}
          <div className="col-span-12 lg:col-span-8 space-y-8">
            <section className="bg-stitch-surface p-6 md:p-8 rounded-xl shadow-sm border border-stitch-border">
              <div className="flex flex-col gap-2 mb-6">
                <div className="flex flex-wrap items-center gap-3">
                  <span className="font-mono text-xs font-bold text-stitch-muted bg-stitch-higher px-2 py-1 rounded border border-stitch-border">
                    {detail.reference_code || `#${detail.id}`}
                  </span>
                  <div className="h-1 w-1 bg-stitch-border rounded-full" />
                  <span className="text-xs font-medium text-stitch-accent-dim bg-stitch-higher px-2 py-1 rounded border border-stitch-border uppercase tracking-wide">
                    {approvalLabel(detail.approval_state)}
                  </span>
                </div>
                <input
                  className="text-3xl font-bold font-headline bg-transparent border-none focus:ring-0 w-full p-0 text-stitch-fg placeholder:text-stitch-muted"
                  placeholder="Enter requirement title…"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  required
                />
              </div>

              <div className="grid grid-cols-2 md:grid-cols-4 gap-6 p-4 bg-stitch-elevated rounded-lg border border-stitch-border">
                <div>
                  <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Priority
                  </span>
                  <div className="flex items-center gap-2 text-sm font-semibold text-stitch-fg">
                    <span className="material-symbols-outlined text-stitch-accent-dim text-lg">
                      priority_high
                    </span>
                    {priorityLabel}
                  </div>
                </div>
                <div>
                  <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Status
                  </span>
                  <div className="flex items-center gap-2">
                    <div
                      className="w-2 h-2 rounded-full shrink-0 bg-stitch-muted"
                      style={statusTagColorSwatchStyle(statusMeta?.tag_color)}
                    />
                    <select
                      className={`flex-1 min-w-0 ${selectStitch} py-1.5 font-semibold text-sm border-0 bg-transparent pl-0 focus:ring-0`}
                      value={statusId}
                      onChange={(e) => setStatusId(Number(e.target.value))}
                    >
                      {statusOptions.map((s) => (
                        <option key={s.id} value={s.id}>
                          {s.title}
                        </option>
                      ))}
                      {!statusOptions.some((s) => s.id === statusId) && (
                        <option value={statusId}>Status #{statusId}</option>
                      )}
                    </select>
                  </div>
                </div>
                <div>
                  <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Version
                  </span>
                  <span className="text-sm font-mono font-medium text-stitch-fg">{latestVersionLabel}</span>
                </div>
                <div>
                  <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Author
                  </span>
                  <span className="text-sm font-semibold text-stitch-fg line-clamp-2">
                    {userLabel(authorId)}
                  </span>
                </div>
              </div>

              <div className="mt-6 grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div>
                  <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Category
                  </label>
                  <select className={selectStitch} value={categoryId} onChange={(e) => setCategoryId(Number(e.target.value))}>
                    {categories.map((c) => (
                      <option key={c.id} value={c.id}>
                        {c.title}
                      </option>
                    ))}
                    {!categories.some((c) => c.id === categoryId) && (
                      <option value={categoryId}>Category #{categoryId}</option>
                    )}
                  </select>
                </div>
                <div>
                  <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Applicability
                  </label>
                  <select
                    className={selectStitch}
                    value={applicabilityId}
                    onChange={(e) => setApplicabilityId(Number(e.target.value))}
                  >
                    {applicability.map((a) => (
                      <option key={a.id} value={a.id}>
                        {a.title}
                      </option>
                    ))}
                    {!applicability.some((a) => a.id === applicabilityId) && (
                      <option value={applicabilityId}>Applicability #{applicabilityId}</option>
                    )}
                  </select>
                </div>
                <div>
                  <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Author (account)
                  </label>
                  <select className={selectStitch} value={authorId} onChange={(e) => setAuthorId(Number(e.target.value))}>
                    {memberOptionIds.map((id) => (
                      <option key={id} value={id}>
                        {userLabel(id)}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Reviewer
                  </label>
                  <select className={selectStitch} value={reviewerId} onChange={(e) => setReviewerId(Number(e.target.value))}>
                    {memberOptionIds.map((id) => (
                      <option key={`r-${id}`} value={id}>
                        {userLabel(id)}
                      </option>
                    ))}
                  </select>
                </div>
              </div>
              <p className="mt-3 text-[10px] text-stitch-muted flex flex-wrap items-center gap-2">
                Displayed status:
                <StatusBadge title={statusTitle} tagColor={statusMeta?.tag_color} />
              </p>
            </section>

            <section className="bg-stitch-surface rounded-xl shadow-sm border border-stitch-border overflow-hidden">
              <div className="flex items-center justify-between px-6 py-3 border-b border-stitch-border bg-stitch-elevated">
                <h3 className="text-xs font-bold uppercase tracking-widest text-stitch-muted font-headline">
                  Requirement statement
                </h3>
                <div className="flex gap-1 opacity-40 pointer-events-none" aria-hidden>
                  <button type="button" className="p-1.5 hover:bg-stitch-higher rounded transition-colors">
                    <span className="material-symbols-outlined text-lg text-stitch-muted">format_bold</span>
                  </button>
                  <button type="button" className="p-1.5 hover:bg-stitch-higher rounded transition-colors">
                    <span className="material-symbols-outlined text-lg text-stitch-muted">format_italic</span>
                  </button>
                  <button type="button" className="p-1.5 hover:bg-stitch-higher rounded transition-colors">
                    <span className="material-symbols-outlined text-lg text-stitch-muted">format_list_bulleted</span>
                  </button>
                  <div className="w-px h-6 bg-stitch-border mx-1 self-center" />
                  <button type="button" className="p-1.5 hover:bg-stitch-higher rounded transition-colors">
                    <span className="material-symbols-outlined text-lg text-stitch-muted">link</span>
                  </button>
                </div>
              </div>
              <div className="p-6 md:p-8">
                <textarea
                  className="w-full min-h-[300px] text-sm leading-relaxed text-stitch-fg bg-transparent border-none focus:ring-0 rounded-md p-0 resize-y placeholder:text-stitch-muted"
                  placeholder="Describe the requirement…"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  required
                />
              </div>
            </section>
          </div>

          {/* Sidebar */}
          <aside className="col-span-12 lg:col-span-4 space-y-6">
            <div className="bg-stitch-surface rounded-xl shadow-sm border border-stitch-border p-6">
              <div className="flex items-center gap-2 mb-6">
                <span className="material-symbols-outlined text-stitch-accent text-xl">account_tree</span>
                <h3 className="text-sm font-bold font-headline text-stitch-accent">Traceability matrix</h3>
              </div>
              <div className="space-y-6">
                <div>
                  <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-3">
                    Upstream links (parent)
                  </p>
                  <div className="space-y-2">
                    {ts.parent_links.length === 0 ? (
                      <p className="text-xs text-stitch-muted py-1">None</p>
                    ) : (
                      ts.parent_links.map((l) => {
                        const parentReq = resolveParentReq(l);
                        return (
                          <div
                            key={l.id}
                            className="flex items-center justify-between gap-2 p-3 bg-stitch-elevated rounded-lg border border-stitch-border group"
                          >
                            <div className="min-w-0 flex-1">
                              {parentReq ? (
                                <Link
                                  to={`/p/${pid}/requirements/${parentReq.id}/edit`}
                                  className="block hover:opacity-90 transition-opacity"
                                  title={parentReq.title?.trim() || undefined}
                                >
                                  <span className="font-mono text-xs font-bold text-stitch-accent">
                                    {(parentReq.reference_code ?? '').trim() || `#${parentReq.id}`}
                                  </span>
                                  <span className="text-[10px] text-stitch-muted">{l.link_type}</span>
                                </Link>
                              ) : (
                                <div>
                                  <span className="font-mono text-[10px] font-bold text-stitch-accent">
                                    Version #{l.target_version_id}
                                  </span>
                                  <span className="block text-[10px] text-stitch-muted">{l.link_type}</span>
                                </div>
                              )}
                            </div>
                            <div className="flex items-center gap-1 shrink-0">
                              {parentReq ? (
                                <Link
                                  to={`/p/${pid}/requirements/${parentReq.id}/edit`}
                                  className="p-1 text-stitch-muted hover:text-stitch-accent transition-colors"
                                  title="Open parent"
                                >
                                  <span className="material-symbols-outlined text-lg">open_in_new</span>
                                </Link>
                              ) : null}
                              <button
                                type="button"
                                title="Remove link"
                                disabled={linkBusy || !(csrfToken ?? '').length}
                                onClick={() => void removeParentLink(l.id)}
                                className="p-1 text-stitch-muted hover:text-red-600 dark:text-red-400 transition-colors disabled:opacity-40"
                              >
                                <span className="material-symbols-outlined text-lg">link_off</span>
                              </button>
                            </div>
                          </div>
                        );
                      })
                    )}
                  </div>
                  {canEditParents ? (
                    <div className="mt-4 pt-4 border-t border-stitch-border space-y-2">
                      <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider">Add parent</p>
                      <select
                        className={selectStitch}
                        value={newParentId === '' ? '' : String(newParentId)}
                        onChange={(e) => setNewParentId(e.target.value === '' ? '' : Number(e.target.value))}
                      >
                        <option value="">Select requirement…</option>
                        {parentCandidates.map((r) => (
                          <option key={r.id} value={r.id}>
                            {r.reference_code || `#${r.id}`} — {r.title.slice(0, 48)}
                            {r.title.length > 48 ? '…' : ''}
                          </option>
                        ))}
                      </select>
                      <select
                        className={selectStitch}
                        value={newLinkType}
                        onChange={(e) => setNewLinkType(e.target.value)}
                      >
                        {linkTypes.map((t) => (
                          <option key={t} value={t}>
                            {t}
                          </option>
                        ))}
                      </select>
                      <button
                        type="button"
                        disabled={linkBusy || newParentId === '' || !(csrfToken ?? '').length}
                        onClick={() => void addParentLink()}
                        className="w-full text-xs font-bold uppercase tracking-wider bg-gradient-to-br from-[#000666] to-[#1a237e] text-white py-2 rounded-lg shadow-sm hover:opacity-95 disabled:opacity-40 transition-opacity"
                      >
                        {linkBusy ? 'Updating…' : 'Add upstream link'}
                      </button>
                    </div>
                  ) : (
                    <p className="mt-3 text-[10px] text-stitch-muted">
                      Parent links need a current requirement version.
                    </p>
                  )}
                </div>

                {ts.child_ids.length > 0 ? (
                  <div>
                    <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-3">
                      Child requirements
                    </p>
                    <div className="space-y-2">
                      {ts.child_ids.map((cid) => (
                        <Link
                          key={cid}
                          to={`/p/${pid}/requirements/${cid}/edit`}
                          className="flex items-center justify-between p-3 bg-stitch-elevated rounded-lg hover:bg-stitch-higher transition-colors"
                        >
                          <div className="min-w-0">
                            <span className="font-mono text-[10px] font-bold text-stitch-muted">#{cid}</span>
                            <span className="block text-xs font-medium text-stitch-fg truncate">
                              {reqTitleById.get(cid) ?? '—'}
                            </span>
                          </div>
                          <span className="material-symbols-outlined text-stitch-muted text-sm">chevron_right</span>
                        </Link>
                      ))}
                    </div>
                  </div>
                ) : null}

                <div>
                  <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-3">
                    Downstream links (verification)
                  </p>
                  <div className="space-y-2">
                    {ts.linked_test_ids.length === 0 ? (
                      <p className="text-xs text-stitch-muted py-1">None</p>
                    ) : (
                      ts.linked_test_ids.map((vid) => {
                        const v = verById.get(vid);
                        const vst = v ? verifStatusById.get(v.status_id) : undefined;
                        const borderColor = vst?.tag_color || undefined;
                        return (
                          <Link
                            key={vid}
                            to={`/p/${pid}/verifications/${vid}/edit`}
                            className="flex items-center justify-between p-3 bg-stitch-elevated rounded-lg border-l-2 border-stitch-muted hover:bg-stitch-higher transition-colors gap-2"
                            style={borderColor ? { borderLeftColor: borderColor } : undefined}
                          >
                            <div className="min-w-0 flex-1">
                              <span
                                className="font-mono text-[10px] font-bold text-stitch-accent-dim block"
                                style={borderColor ? { color: borderColor } : undefined}
                              >
                                {v?.reference_code ?? `TEST-${vid}`}
                              </span>
                              <span className="text-xs font-medium text-stitch-fg block truncate">
                                {v?.name ?? '—'}
                              </span>
                            </div>
                            <div className="shrink-0">
                              {vst?.title ? (
                                <StatusBadge title={vst.title} tagColor={vst.tag_color} />
                              ) : (
                                <span className="text-[10px] font-bold uppercase text-stitch-muted">—</span>
                              )}
                            </div>
                          </Link>
                        );
                      })
                    )}
                  </div>
                </div>
              </div>
            </div>

            <div className="bg-stitch-surface rounded-xl shadow-sm border border-stitch-border flex flex-col max-h-[400px]">
              <div className="px-6 py-4 border-b border-stitch-border flex items-center justify-between shrink-0">
                <h3 className="text-sm font-bold font-headline text-stitch-accent">Change log &amp; discussion</h3>
              </div>
              <div className="flex-1 overflow-y-auto p-6 space-y-6">
                {versions.length > 0 ? (
                  <div className="flex gap-3 opacity-90">
                    <span className="material-symbols-outlined text-stitch-muted text-lg shrink-0">history</span>
                    <div>
                      <div className="flex items-center gap-2 mb-1">
                        <span className="text-xs font-bold text-stitch-muted">Version history</span>
                        <span className="text-[10px] text-stitch-muted">{latestVersionLabel}</span>
                      </div>
                      <p className="text-xs text-stitch-muted leading-relaxed">
                        {versions.length} snapshot(s). Latest:{' '}
                        {latestVersionCreatedAt ? formatTs(latestVersionCreatedAt) : '—'}.
                      </p>
                      {projectSlug ? (
                        <a
                          href={`/p/${projectSlug}/requirements/show/${rid}`}
                          className="text-[10px] font-bold text-stitch-accent hover:underline mt-2 inline-block uppercase tracking-wide"
                        >
                          Full diffs in classic UI →
                        </a>
                      ) : null}
                    </div>
                  </div>
                ) : null}
                {comments.length === 0 && versions.length === 0 ? (
                  <p className="text-xs text-stitch-muted">No activity yet.</p>
                ) : null}
                {[...comments]
                  .sort(
                    (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
                  )
                  .map((c) => (
                    <div key={c.id} className="flex gap-3">
                      <div className="h-6 w-6 rounded-full bg-stitch-accent flex items-center justify-center text-[10px] text-stitch-on-accent font-bold shrink-0">
                        {c.author_name
                          ? c.author_name
                              .split(/\s+/)
                              .map((p) => p[0])
                              .join('')
                              .slice(0, 2)
                              .toUpperCase()
                          : '?'}
                      </div>
                      <div className="min-w-0">
                        <div className="flex items-center gap-2 mb-1 flex-wrap">
                          <span className="text-xs font-bold text-stitch-fg">{c.author_name}</span>
                          <span className="text-[10px] text-stitch-muted">{formatRelativeTime(c.created_at) || formatTs(c.created_at)}</span>
                        </div>
                        <p className="text-xs text-stitch-muted leading-relaxed whitespace-pre-wrap">{c.body}</p>
                      </div>
                    </div>
                  ))}
              </div>
              <div className="px-6 py-3 border-t border-stitch-border bg-stitch-elevated shrink-0">
                <textarea
                  className={`w-full min-h-[72px] text-sm resize-y ${selectStitch}`}
                  placeholder="Add a comment…"
                  value={commentBody}
                  onChange={(e) => setCommentBody(e.target.value)}
                />
                <button
                  type="button"
                  disabled={commentPosting || !commentBody.trim() || !(csrfToken ?? '').length}
                  onClick={() => void postComment()}
                  className="mt-2 text-stitch-accent text-[10px] font-bold uppercase tracking-wider hover:underline disabled:opacity-40"
                >
                  {commentPosting ? 'Posting…' : 'Add comment'}
                </button>
              </div>
            </div>

            <div className="bg-stitch-surface rounded-xl shadow-sm border border-stitch-border p-6">
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-sm font-bold font-headline text-stitch-accent">Attachments</h3>
                <span className="material-symbols-outlined text-stitch-muted" title="Manage in classic UI">
                  add_circle
                </span>
              </div>
              <p className="text-xs text-stitch-muted mb-2">
                Upload and manage files on the classic requirement page.
              </p>
              {projectSlug ? (
                <a
                  href={`/p/${projectSlug}/requirements/show/${rid}`}
                  className="text-xs font-bold text-stitch-accent hover:underline"
                >
                  Open classic requirement →
                </a>
              ) : (
                <p className="text-xs text-stitch-muted">Project slug unavailable.</p>
              )}
            </div>
          </aside>
        </div>

        {saveError && (
          <div className="fixed bottom-24 left-1/2 -translate-x-1/2 max-w-lg w-[calc(100%-2rem)] rounded-lg bg-red-500/15 border border-red-500/35 text-red-900 dark:text-red-100 text-sm px-4 py-2 shadow-lg z-[60]">
            {saveError}
          </div>
        )}

        <footer className="fixed bottom-0 left-0 right-0 z-40 bg-stitch-surface/95 backdrop-blur-md border-t border-stitch-border px-4 md:px-8 py-4 flex flex-wrap items-center justify-between gap-3 shadow-stitch">
          <div className="flex items-center gap-2">
            <button
              type="button"
              className="flex items-center gap-2 text-stitch-muted hover:text-red-600 dark:text-red-400 transition-colors text-xs font-bold uppercase tracking-wider px-3 py-2 rounded disabled:opacity-40"
              disabled={deleteBusy || !(csrfToken ?? '').length}
              onClick={() => void deleteRequirement()}
            >
              <span className="material-symbols-outlined text-lg">archive</span>
              Archive requirement
            </button>
            <button
              type="button"
              className="text-xs font-bold uppercase tracking-wider text-stitch-muted hover:text-stitch-fg px-3 py-2"
              onClick={() => navigate(`/p/${pid}/requirements`)}
            >
              Cancel
            </button>
          </div>
          <div className="flex items-center gap-3">
            <button
              type="button"
              disabled={!dirty}
              onClick={revert}
              className="px-5 py-2 text-xs font-bold uppercase tracking-widest text-stitch-muted hover:bg-stitch-elevated transition-colors rounded-lg disabled:opacity-40"
            >
              Revert changes
            </button>
            <button
              type="submit"
              disabled={saving || !dirty}
              className="bg-gradient-to-br from-[#000666] to-[#1a237e] text-white px-8 py-2.5 rounded-lg text-xs font-bold uppercase tracking-widest shadow-lg shadow-stitch active:scale-[0.98] transition-all disabled:opacity-50"
            >
              {saving ? 'Saving…' : 'Save requirement'}
            </button>
          </div>
        </footer>
      </form>
    </div>
  );
}

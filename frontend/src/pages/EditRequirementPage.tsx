import { FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import {
  createRequirementComment,
  deleteRequirementGlobally,
  getRequirementByProject,
  listApplicability,
  listCategories,
  listProjectMembers,
  listRequirementComments,
  listRequirementStatuses,
  listRequirementVersionsByProject,
  listRequirements,
  listUsersOptional,
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
  User,
  Verification,
} from '@/api/types';

function approvalLabel(state: string): string {
  return state.replace(/_/g, ' ').toUpperCase();
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
  const [categories, setCategories] = useState<Category[]>([]);
  const [applicability, setApplicability] = useState<Applicability[]>([]);
  const [members, setMembers] = useState<ProjectMember[]>([]);
  const [users, setUsers] = useState<User[] | null>(null);
  const [projectReqs, setProjectReqs] = useState<Requirement[]>([]);
  const [verifications, setVerifications] = useState<Verification[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [comments, setComments] = useState<RequirementCommentItem[]>([]);
  const [commentBody, setCommentBody] = useState('');
  const [commentPosting, setCommentPosting] = useState(false);
  const [deleteBusy, setDeleteBusy] = useState(false);

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
        cat,
        app,
        mem,
        reqs,
        ver,
        u,
        cmts,
      ] = await Promise.all([
        getRequirementByProject(pid, rid),
        listRequirementVersionsByProject(pid, rid),
        listRequirementStatuses(),
        listCategories(),
        listApplicability(),
        listProjectMembers(pid),
        listRequirements(pid),
        listVerifications(),
        listUsersOptional(),
        listRequirementComments(rid),
      ]);
      setDetail(d);
      setComments(cmts);
      setVersions(v);
      setStatuses(st);
      setCategories(cat.filter((c) => c.project_id === pid));
      setApplicability(app.filter((a) => a.project_id === pid));
      setMembers(mem);
      setProjectReqs(reqs);
      setVerifications(ver.filter((x) => x.project_id === pid));
      setUsers(u);

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

  const reqTitleById = useMemo(() => {
    const m = new Map<number, string>();
    for (const r of projectReqs) m.set(r.id, r.title);
    return m;
  }, [projectReqs]);

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
      await patchRequirementByProject(pid, rid, {
        title: title.trim(),
        description: description.trim(),
        status_id: statusId,
        category_id: categoryId,
        applicability_id: applicabilityId,
        author_id: authorId,
        reviewer_id: reviewerId,
      }, token);
      await refreshDashboard();
      await load();
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : 'Save failed');
    } finally {
      setSaving(false);
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

  if (loadError) {
    return (
      <div className="rounded-xl border border-red-500/30 bg-red-500/10 p-4 text-sm text-red-200">
        {loadError}
        <div className="mt-3">
          <Link
            to={`/p/${pid}/requirements`}
            className="font-semibold text-stitch-accent underline"
          >
            Back to requirements
          </Link>
        </div>
      </div>
    );
  }

  if (!detail) {
    return (
      <div className="text-stitch-muted text-sm py-12 text-center font-body">
        Loading requirement…
      </div>
    );
  }

  const { trace_summary: ts } = detail;
  const latestVersionLabel =
    versions.length > 0 ? `v${versions.length}` : '—';

  const selectClass =
    'w-full text-sm font-medium bg-stitch-elevated border border-stitch-border rounded-md px-2 py-2 text-white focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/40 outline-none transition-colors';

  return (
    <div className="pb-28 font-body text-white text-stitch">
      <nav className="flex items-center gap-2 text-[10px] font-semibold text-stitch-muted mb-6 uppercase tracking-widest">
        <Link to={`/p/${pid}/requirements`} className="hover:text-stitch-accent transition-colors">
          Requirements
        </Link>
        <span className="material-symbols-outlined text-sm text-stitch-muted">chevron_right</span>
        <span className="text-stitch-accent font-bold">
          {detail.reference_code || `REQ-${detail.id}`}
        </span>
      </nav>

      <form onSubmit={onSave} className="max-w-7xl mx-auto">
        <div className="grid grid-cols-12 gap-8">
          <div className="col-span-12 lg:col-span-8 space-y-8">
            <section className="bg-stitch-surface p-6 md:p-8 rounded-xl shadow-stitch border border-stitch-border">
              <div className="flex flex-col gap-2 mb-6">
                <div className="flex flex-wrap items-center gap-3">
                  <span className="font-mono text-xs font-bold text-stitch-accent bg-stitch-elevated px-2 py-1 rounded-md border border-stitch-border">
                    {detail.reference_code || `#${detail.id}`}
                  </span>
                  <div className="h-1 w-1 bg-stitch-muted/50 rounded-full" />
                  <span className="text-[10px] font-bold text-stitch-subtle bg-stitch-elevated px-2 py-1 rounded-md uppercase tracking-wide border border-stitch-border">
                    {approvalLabel(detail.approval_state)}
                  </span>
                </div>
                <input
                  className="text-2xl md:text-3xl font-bold font-headline bg-transparent border-none focus:ring-2 focus:ring-stitch-accent/30 rounded-md w-full p-0 text-white placeholder:text-stitch-muted"
                  placeholder="Requirement title"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  required
                />
              </div>

              <div className="grid grid-cols-2 md:grid-cols-4 gap-4 md:gap-6 p-4 bg-stitch-elevated rounded-lg border border-stitch-border">
                <div>
                  <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Priority
                  </span>
                  <div className="flex items-center gap-2 text-sm font-semibold text-white">
                    <span className="material-symbols-outlined text-stitch-accent text-base">
                      priority_high
                    </span>
                    {priorityLabel}
                  </div>
                </div>
                <div>
                  <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Status
                  </span>
                  <select
                    className={`mt-0.5 ${selectClass} font-semibold py-1.5`}
                    value={statusId}
                    onChange={(e) => setStatusId(Number(e.target.value))}
                  >
                    {statusOptions.map((s) => (
                      <option key={s.id} value={s.id} className="bg-stitch-surface text-white">
                        {s.title}
                      </option>
                    ))}
                    {!statusOptions.some((s) => s.id === statusId) && (
                      <option value={statusId} className="bg-stitch-surface text-white">
                        Status #{statusId}
                      </option>
                    )}
                  </select>
                </div>
                <div>
                  <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Versions
                  </span>
                  <span className="text-sm font-mono font-medium text-white">{latestVersionLabel}</span>
                </div>
                <div>
                  <span className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Display status
                  </span>
                  <span className="text-sm font-semibold text-white">{statusTitle}</span>
                </div>
              </div>

              <div className="mt-6 grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div>
                  <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Category
                  </label>
                  <select
                    className={selectClass}
                    value={categoryId}
                    onChange={(e) => setCategoryId(Number(e.target.value))}
                  >
                    {categories.map((c) => (
                      <option key={c.id} value={c.id} className="bg-stitch-surface text-white">
                        {c.title}
                      </option>
                    ))}
                    {!categories.some((c) => c.id === categoryId) && (
                      <option value={categoryId} className="bg-stitch-surface text-white">
                        Category #{categoryId}
                      </option>
                    )}
                  </select>
                </div>
                <div>
                  <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Applicability
                  </label>
                  <select
                    className={selectClass}
                    value={applicabilityId}
                    onChange={(e) => setApplicabilityId(Number(e.target.value))}
                  >
                    {applicability.map((a) => (
                      <option key={a.id} value={a.id} className="bg-stitch-surface text-white">
                        {a.title}
                      </option>
                    ))}
                    {!applicability.some((a) => a.id === applicabilityId) && (
                      <option value={applicabilityId} className="bg-stitch-surface text-white">
                        Applicability #{applicabilityId}
                      </option>
                    )}
                  </select>
                </div>
                <div>
                  <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Author
                  </label>
                  <select
                    className={selectClass}
                    value={authorId}
                    onChange={(e) => setAuthorId(Number(e.target.value))}
                  >
                    {memberOptionIds.map((id) => (
                      <option key={id} value={id} className="bg-stitch-surface text-white">
                        {userLabel(id)}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                    Reviewer
                  </label>
                  <select
                    className={selectClass}
                    value={reviewerId}
                    onChange={(e) => setReviewerId(Number(e.target.value))}
                  >
                    {memberOptionIds.map((id) => (
                      <option key={`r-${id}`} value={id} className="bg-stitch-surface text-white">
                        {userLabel(id)}
                      </option>
                    ))}
                  </select>
                </div>
              </div>
            </section>

            <section className="bg-stitch-surface rounded-xl shadow-stitch border border-stitch-border overflow-hidden">
              <div className="flex items-center justify-between px-6 py-3 border-b border-stitch-border bg-stitch-elevated">
                <h3 className="text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
                  Requirement statement
                </h3>
              </div>
              <div className="p-6 md:p-8">
                <textarea
                  className="w-full min-h-[280px] text-sm leading-relaxed text-white bg-transparent border-none focus:ring-2 focus:ring-stitch-accent/25 rounded-md p-0 resize-y placeholder:text-stitch-muted"
                  placeholder="Describe the requirement…"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  required
                />
              </div>
            </section>
          </div>

          <aside className="col-span-12 lg:col-span-4 space-y-6">
            <div className="bg-stitch-surface rounded-xl shadow-stitch border border-stitch-border p-6">
              <div className="flex items-center gap-2 mb-4">
                <span className="material-symbols-outlined text-stitch-accent">account_tree</span>
                <h3 className="text-sm font-bold font-headline text-white">Traceability</h3>
              </div>
              <div className="space-y-5">
                <div>
                  <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-2">
                    Parent links
                  </p>
                  {ts.parent_links.length === 0 ? (
                    <p className="text-xs text-stitch-muted">None</p>
                  ) : (
                    <ul className="space-y-2">
                      {ts.parent_links.map((l) => (
                        <li
                          key={l.id}
                          className="p-3 bg-stitch-elevated rounded-lg text-xs border-l-2 border-stitch-accent/60"
                        >
                          <span className="font-mono text-[10px] font-bold text-stitch-accent">
                            {l.link_type}
                          </span>
                          <span className="block text-stitch-muted mt-0.5">
                            Target version #{l.target_version_id}
                          </span>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
                <div>
                  <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-2">
                    Child requirements
                  </p>
                  {ts.child_ids.length === 0 ? (
                    <p className="text-xs text-stitch-muted">None</p>
                  ) : (
                    <ul className="space-y-2">
                      {ts.child_ids.map((cid) => (
                        <li key={cid} className="p-3 bg-stitch-elevated rounded-lg text-xs border border-stitch-border">
                          <Link
                            to={`/p/${pid}/requirements/${cid}/edit`}
                            className="block hover:opacity-90 transition-opacity"
                          >
                            <span className="font-mono text-[10px] font-bold text-stitch-accent">#{cid}</span>
                            <span className="block font-medium text-white">{reqTitleById.get(cid) ?? '—'}</span>
                          </Link>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
                <div>
                  <p className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-2">
                    Linked verifications
                  </p>
                  {ts.linked_test_ids.length === 0 ? (
                    <p className="text-xs text-stitch-muted">None</p>
                  ) : (
                    <ul className="space-y-2">
                      {ts.linked_test_ids.map((vid) => {
                        const v = verById.get(vid);
                        return (
                          <li
                            key={vid}
                            className="flex items-center justify-between p-3 bg-stitch-elevated rounded-lg border-l-2 border-emerald-400/70"
                          >
                            <Link
                              to={`/p/${pid}/verifications/${vid}/edit`}
                              className="min-w-0 flex-1 hover:opacity-90 transition-opacity"
                            >
                              <span className="font-mono text-[10px] font-bold text-emerald-300/90">
                                {v?.reference_code ?? `#${vid}`}
                              </span>
                              <span className="block text-xs font-medium text-white">{v?.name ?? '—'}</span>
                            </Link>
                          </li>
                        );
                      })}
                    </ul>
                  )}
                </div>
              </div>
            </div>

            <div className="bg-stitch-surface rounded-xl shadow-stitch border border-stitch-border p-6 space-y-4">
              <h3 className="text-sm font-bold font-headline text-white">Version history</h3>
              {versions.length === 0 ? (
                <p className="text-xs text-stitch-muted">No stored versions yet.</p>
              ) : (
                <ul className="space-y-2 max-h-52 overflow-y-auto">
                  {[...versions]
                    .sort(
                      (a, b) =>
                        new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
                    )
                    .map((ver, idx) => (
                      <li
                        key={ver.id}
                        className="text-xs p-3 rounded-lg bg-stitch-elevated border border-stitch-border"
                      >
                        <div className="flex justify-between gap-2 items-start">
                          <span className="font-mono font-bold text-stitch-accent shrink-0">
                            v{versions.length - idx}
                          </span>
                          <span className="text-stitch-muted text-[10px]">{formatTs(ver.created_at)}</span>
                        </div>
                        <p className="text-white/90 mt-1 line-clamp-2">{ver.title}</p>
                        <span className="text-[10px] uppercase text-stitch-subtle">
                          {approvalLabel(ver.approval_state)}
                        </span>
                      </li>
                    ))}
                </ul>
              )}
              {projectSlug ? (
                <a
                  href={`/p/${projectSlug}/requirements/show/${rid}`}
                  className="text-xs font-bold text-stitch-accent hover:underline inline-block"
                >
                  Full history &amp; diffs in classic UI →
                </a>
              ) : null}
            </div>

            <div className="bg-stitch-surface rounded-xl shadow-stitch border border-stitch-border p-6 space-y-3">
              <h3 className="text-sm font-bold font-headline text-white">Discussion</h3>
              <textarea
                className={`w-full min-h-[80px] text-sm resize-y ${selectClass}`}
                placeholder="Add a comment…"
                value={commentBody}
                onChange={(e) => setCommentBody(e.target.value)}
              />
              <button
                type="button"
                disabled={
                  commentPosting || !commentBody.trim() || !(csrfToken ?? '').length
                }
                onClick={() => void postComment()}
                className="text-xs font-bold uppercase tracking-wider bg-stitch-accent text-stitch-canvas px-4 py-2 rounded-md disabled:opacity-40"
              >
                {commentPosting ? 'Posting…' : 'Post comment'}
              </button>
              <ul className="space-y-3 max-h-64 overflow-y-auto pt-3 border-t border-stitch-border">
                {comments.length === 0 ? (
                  <li className="text-xs text-stitch-muted">No comments yet.</li>
                ) : (
                  [...comments]
                    .sort(
                      (a, b) =>
                        new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
                    )
                    .map((c) => (
                      <li key={c.id} className="text-xs border-l-2 border-stitch-accent/40 pl-3">
                        <div className="flex justify-between gap-2 text-stitch-muted">
                          <span className="font-semibold text-white">{c.author_name}</span>
                          <span className="shrink-0">{formatTs(c.created_at)}</span>
                        </div>
                        <p className="text-stitch-muted mt-1 whitespace-pre-wrap">{c.body}</p>
                      </li>
                    ))
                )}
              </ul>
            </div>

            <div className="bg-stitch-surface rounded-xl shadow-stitch border border-stitch-border p-6">
              <h3 className="text-sm font-bold font-headline text-white mb-2">Attachments</h3>
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
          <div className="fixed bottom-20 left-1/2 -translate-x-1/2 max-w-lg w-[calc(100%-2rem)] rounded-lg bg-red-500/20 border border-red-500/30 text-red-100 text-sm px-4 py-2 shadow-stitch z-50">
            {saveError}
          </div>
        )}

        <footer className="fixed bottom-0 left-0 right-0 z-40 bg-stitch-surface/85 backdrop-blur-md border-t border-stitch-border px-4 md:px-8 py-3 flex flex-wrap items-center justify-between gap-3">
          <div className="flex flex-wrap items-center gap-2">
            <button
              type="button"
              className="text-xs font-bold uppercase tracking-wider text-red-400/90 hover:text-red-300 transition-colors px-2 py-2 disabled:opacity-40"
              disabled={deleteBusy || !(csrfToken ?? '').length}
              onClick={() => void deleteRequirement()}
            >
              {deleteBusy ? 'Deleting…' : 'Delete requirement'}
            </button>
            <button
              type="button"
              className="text-xs font-bold uppercase tracking-wider text-stitch-muted hover:text-stitch-danger transition-colors px-2 py-2"
              onClick={() => navigate(`/p/${pid}/requirements`)}
            >
              Cancel
            </button>
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              disabled={!dirty}
              onClick={revert}
              className="px-4 py-2 text-xs font-bold uppercase tracking-widest text-stitch-muted hover:bg-white/[0.06] rounded-md transition-colors disabled:opacity-40"
            >
              Revert changes
            </button>
            <button
              type="submit"
              disabled={saving || !dirty}
              className="bg-stitch-accent text-stitch-canvas px-6 py-2.5 rounded-md text-xs font-bold uppercase tracking-widest shadow-stitch disabled:opacity-50 transition-transform active:scale-[0.98] hover:bg-stitch-accent-dim"
            >
              {saving ? 'Saving…' : 'Save requirement'}
            </button>
          </div>
        </footer>
      </form>
    </div>
  );
}

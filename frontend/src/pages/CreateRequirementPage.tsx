import { FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import {
  createRequirementByProject,
  listApplicability,
  listCategories,
  listProjectMembers,
  listRequirementStatuses,
  listVerificationMethodsByProject,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type {
  Applicability,
  Category,
  ProjectMember,
  RequirementStatus,
  User,
  VerificationMethod,
} from '@/api/types';
import { statusTagColorSwatchStyle } from '@/components/StatusBadge';

const selectClass =
  'w-full text-sm font-medium bg-stitch-elevated border border-stitch-border rounded-md px-2 py-2 text-stitch-fg focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/40 outline-none transition-colors';

function parseUser(u: unknown): User | null {
  if (u && typeof u === 'object' && 'username' in u) return u as User;
  return null;
}

export default function CreateRequirementPage() {
  const { projectId: projectIdParam } = useParams();
  const pid = Number(projectIdParam);
  const navigate = useNavigate();
  const { csrfToken, dashboard, refresh: refreshDashboard } = useDashboard();

  const me = useMemo(() => parseUser(dashboard?.user), [dashboard?.user]);

  const [statuses, setStatuses] = useState<RequirementStatus[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [applicability, setApplicability] = useState<Applicability[]>([]);
  const [members, setMembers] = useState<ProjectMember[]>([]);
  const [methods, setMethods] = useState<VerificationMethod[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [referenceCode, setReferenceCode] = useState('');
  const [statusId, setStatusId] = useState(0);
  const [categoryId, setCategoryId] = useState(0);
  const [applicabilityId, setApplicabilityId] = useState(0);
  const [authorId, setAuthorId] = useState(0);
  const [reviewerId, setReviewerId] = useState(0);
  const [justification, setJustification] = useState('');
  const [methodIds, setMethodIds] = useState<number[]>([]);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setLoadError(null);
    try {
      const [st, cat, app, mem, meth] = await Promise.all([
        listRequirementStatuses(),
        listCategories(),
        listApplicability(),
        listProjectMembers(pid),
        listVerificationMethodsByProject(pid),
      ]);
      setStatuses(st);
      setCategories(cat.filter((c) => c.project_id === pid));
      setApplicability(app.filter((a) => a.project_id === pid));
      setMembers(mem);
      setMethods(meth);
      const statusOpts = st.filter((s) => s.project_id === pid);
      const useStatuses = statusOpts.length > 0 ? statusOpts : st;
      if (useStatuses[0]) setStatusId((id) => (id === 0 ? useStatuses[0]!.id : id));
      const firstCat = cat.find((c) => c.project_id === pid);
      if (firstCat) setCategoryId((id) => (id === 0 ? firstCat.id : id));
      const firstApp = app.find((a) => a.project_id === pid);
      if (firstApp) setApplicabilityId((id) => (id === 0 ? firstApp.id : id));
      setMethodIds((prev) => (prev.length === 0 && meth[0] ? [meth[0]!.id] : prev));
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : 'Failed to load form data');
    }
  }, [pid]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (me?.id) {
      setAuthorId((a) => (a === 0 ? me.id : a));
      setReviewerId((r) => (r === 0 ? me.id : r));
    }
  }, [me?.id]);

  const userLabel = useCallback(
    (id: number) => {
      if (me && me.id === id) return `${me.name} (${me.username})`;
      return `User #${id}`;
    },
    [me],
  );

  const memberOptionIds = useMemo(() => {
    const ids = new Set(members.map((m) => m.user_id));
    if (me?.id) ids.add(me.id);
    ids.add(authorId);
    ids.add(reviewerId);
    return [...ids].sort((a, b) => a - b);
  }, [members, me?.id, authorId, reviewerId]);

  const statusOptions = useMemo(() => {
    const forProject = statuses.filter((s) => s.project_id === pid);
    return forProject.length > 0 ? forProject : statuses;
  }, [statuses, pid]);

  const statusMeta = useMemo(() => statuses.find((s) => s.id === statusId), [statuses, statusId]);

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    const token = csrfToken ?? '';
    if (!token) {
      setSaveError('Missing CSRF token; refresh the page.');
      return;
    }
    if (methodIds.length === 0) {
      setSaveError('Select at least one verification method (required to create a requirement).');
      return;
    }
    setSaveError(null);
    setSaving(true);
    try {
      const { id } = await createRequirementByProject(
        pid,
        {
          title: title.trim(),
          description: description.trim(),
          reference_code: referenceCode.trim(),
          status_id: statusId,
          category_id: categoryId,
          applicability_id: applicabilityId,
          author_id: authorId,
          reviewer_id: reviewerId,
          justification: justification.trim() || null,
          project_id: pid,
          verification_method_ids: methodIds,
          parent_links: [],
        },
        token,
      );
      await refreshDashboard();
      navigate(`/p/${pid}/requirements/${id}/edit`);
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : 'Create failed');
    } finally {
      setSaving(false);
    }
  }

  if (loadError) {
    return (
      <div className="rounded-xl border border-red-500/30 bg-red-500/10 p-4 text-sm text-red-200">
        {loadError}
        <div className="mt-3">
          <Link to={`/p/${pid}/requirements`} className="font-semibold text-stitch-accent underline">
            Back to requirements
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="pb-28 font-body text-stitch-fg text-stitch max-w-4xl">
      <nav className="flex items-center gap-2 text-[10px] font-semibold text-stitch-muted mb-6 uppercase tracking-widest">
        <Link to={`/p/${pid}/requirements`} className="hover:text-stitch-accent transition-colors">
          Requirements
        </Link>
        <span className="material-symbols-outlined text-sm text-stitch-muted">chevron_right</span>
        <span className="text-stitch-accent font-bold">New requirement</span>
      </nav>

      <div className="flex flex-col sm:flex-row sm:items-end sm:justify-between gap-4 mb-8">
        <div>
          <nav className="flex text-[10px] text-stitch-muted font-mono uppercase tracking-widest mb-1">
            <span>{projectName}</span>
            <span className="mx-2">/</span>
            <span className="text-stitch-accent font-bold">Create</span>
          </nav>
          <h1 className="text-2xl md:text-3xl font-extrabold text-stitch-fg tracking-tight font-headline">
            Create requirement
          </h1>
          <p className="text-stitch-muted text-sm mt-2">
            A verification method link is required by the API for each new requirement.
          </p>
        </div>
      </div>

      <form onSubmit={onSubmit} className="space-y-8">
        <section className="bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch p-6 md:p-8 space-y-6">
          <div>
            <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
              Reference code
            </label>
            <input
              required
              value={referenceCode}
              onChange={(e) => setReferenceCode(e.target.value)}
              className={selectClass}
              placeholder="REQ-0001"
            />
          </div>
          <div>
            <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
              Title
            </label>
            <input
              required
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              className={selectClass}
              placeholder="Short title"
            />
          </div>
          <div>
            <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
              Description
            </label>
            <textarea
              required
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={6}
              className={`${selectClass} min-h-[140px] resize-y`}
              placeholder="Requirement statement…"
            />
          </div>
          <div>
            <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-2">
              Verification methods <span className="text-stitch-danger">*</span>
            </label>
            {methods.length === 0 ? (
              <p className="text-sm text-amber-200/90 bg-amber-500/10 border border-amber-500/25 rounded-lg p-3">
                No verification methods in this project. Add methods in the legacy UI, then reload this page.
              </p>
            ) : (
              <select
                multiple
                required
                size={Math.min(8, methods.length)}
                value={methodIds.map(String)}
                onChange={(e) => {
                  const next = Array.from(e.target.selectedOptions, (o) => Number(o.value));
                  setMethodIds(next);
                }}
                className={`${selectClass} min-h-[120px]`}
              >
                {methods.map((m) => (
                  <option key={m.id} value={m.id} className="bg-stitch-surface text-stitch-fg">
                    {m.title} ({m.tag})
                  </option>
                ))}
              </select>
            )}
            <p className="text-[10px] text-stitch-muted mt-1">Hold Ctrl/Cmd to select multiple.</p>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                Status
              </label>
              <div className="flex items-center gap-2">
                <div
                  className="w-2 h-2 rounded-full shrink-0 bg-stitch-accent/40"
                  style={statusTagColorSwatchStyle(statusMeta?.tag_color)}
                  title={statusMeta?.tag_color ? 'Catalog color' : undefined}
                />
                <select
                  className={`${selectClass} flex-1 min-w-0`}
                  value={statusId}
                  onChange={(e) => setStatusId(Number(e.target.value))}
                >
                  {statusOptions.map((s) => (
                    <option key={s.id} value={s.id} className="bg-stitch-surface text-stitch-fg">
                      {s.title}
                    </option>
                  ))}
                </select>
              </div>
            </div>
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
                  <option key={c.id} value={c.id} className="bg-stitch-surface text-stitch-fg">
                    {c.title}
                  </option>
                ))}
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
                  <option key={a.id} value={a.id} className="bg-stitch-surface text-stitch-fg">
                    {a.title}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                Justification (optional)
              </label>
              <input
                value={justification}
                onChange={(e) => setJustification(e.target.value)}
                className={selectClass}
                placeholder="—"
              />
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
                  <option key={id} value={id} className="bg-stitch-surface text-stitch-fg">
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
                  <option key={`r-${id}`} value={id} className="bg-stitch-surface text-stitch-fg">
                    {userLabel(id)}
                  </option>
                ))}
              </select>
            </div>
          </div>
        </section>

        {saveError && (
          <div className="rounded-lg bg-red-500/15 border border-red-500/30 text-red-100 text-sm px-4 py-3">
            {saveError}
          </div>
        )}

        <footer className="fixed bottom-0 left-0 right-0 z-40 bg-stitch-surface/85 backdrop-blur-md border-t border-stitch-border px-4 md:px-8 py-3 flex flex-wrap items-center justify-between gap-3">
          <Link
            to={`/p/${pid}/requirements`}
            className="text-xs font-bold uppercase tracking-wider text-stitch-muted hover:text-stitch-danger transition-colors px-2 py-2"
          >
            Cancel
          </Link>
          <button
            type="submit"
            disabled={saving || methods.length === 0}
            className="bg-stitch-accent text-stitch-canvas px-6 py-2.5 rounded-md text-xs font-bold uppercase tracking-widest shadow-stitch disabled:opacity-50 hover:bg-stitch-accent-dim transition-colors"
          >
            {saving ? 'Creating…' : 'Create requirement'}
          </button>
        </footer>
      </form>
    </div>
  );
}

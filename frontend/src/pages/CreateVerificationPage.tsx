import { FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useNavigate, useOutletContext, useSearchParams } from 'react-router-dom';
import {
  createVerification,
  getMyPermissions,
  getProjectReviewers,
  getVerification,
  listProjectMembers,
  listUsersOptional,
  listRequirements,
  listVerificationMethodsByProject,
  listVerificationStatuses,
  listVerifications,
  putVerificationMatrix,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type {
  EffectivePermissions,
  ProjectMember,
  User,
  Requirement,
  Verification,
  VerificationMethod,
  VerificationStatus,
} from '@/api/types';
import { RequirementMatrixPicker } from '@/components/RequirementMatrixPicker';
import { statusTagColorSwatchStyle } from '@/components/StatusBadge';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import { initialVerificationStatusIdForAuthor } from '@/statusAuthorDefaults';
import { formatUserLabel } from '@/utils/userLabel';
import { nextDuplicateReference } from '@/utils/duplicateRequirement';
import { duplicateSourceQueryId, parsePositiveQueryId } from '@/utils/createQueryParams';

const selectClass =
  'w-full text-sm font-medium bg-stitch-elevated border border-stitch-border rounded-md px-2 py-2 text-stitch-fg focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/40 outline-none transition-colors';

export default function CreateVerificationPage() {
  const { basePath, projectId: pid } = useOutletContext<ProjectOutletContext>();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { csrfToken, dashboard, refresh: refreshDashboard } = useDashboard();
  const duplicateFrom = duplicateSourceQueryId(searchParams);
  const parentQueryId = parsePositiveQueryId(searchParams.get('parent'));

  const [statuses, setStatuses] = useState<VerificationStatus[]>([]);
  const [methods, setMethods] = useState<VerificationMethod[]>([]);
  const [existing, setExisting] = useState<Verification[]>([]);
  const [requirements, setRequirements] = useState<Requirement[]>([]);
  const [linkedReqIds, setLinkedReqIds] = useState<number[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [queryWarning, setQueryWarning] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const [name, setName] = useState('');
  const [referenceCode, setReferenceCode] = useState('');
  const [description, setDescription] = useState('');
  const [source, setSource] = useState('manual');
  const [statusId, setStatusId] = useState(0);
  const [parentId, setParentId] = useState<string>(''); // "" = none
  const [methodId, setMethodId] = useState<string>(''); // "" = none
  const [members, setMembers] = useState<ProjectMember[]>([]);
  const [projectReviewerIds, setProjectReviewerIds] = useState<number[]>([]);
  const [perms, setPerms] = useState<EffectivePermissions | null>(null);
  const [authorId, setAuthorId] = useState(0);
  const [reviewerId, setReviewerId] = useState(0);
  const [users, setUsers] = useState<User[] | null>(null);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setLoadError(null);
    setQueryWarning(null);
    try {
      const [st, meth, ver, reqs, mem, userList, revPool, p] = await Promise.all([
        listVerificationStatuses(),
        listVerificationMethodsByProject(pid),
        listVerifications(),
        listRequirements(pid),
        listProjectMembers(pid),
        listUsersOptional(),
        getProjectReviewers(pid).catch(() => ({ user_ids: [] as number[] })),
        getMyPermissions(pid).catch(() => null),
      ]);
      setPerms(p);
      setProjectReviewerIds(revPool.user_ids);
      setUsers(userList);
      setStatuses(st);
      setMethods(meth);
      const projectVers = ver.filter((v) => v.project_id === pid);
      setExisting(projectVers);
      setRequirements(reqs);
      setMembers(mem);
      const forProject = st.filter((s) => s.project_id === pid);
      const useSt = forProject.length > 0 ? forProject : st;
      if (useSt[0]) setStatusId((id) => (id === 0 ? useSt[0]!.id : id));
      const sessionUserId = (dashboard?.user as { id?: number } | undefined)?.id;
      const mids = mem.map((m) => m.user_id);
      const authorPick =
        sessionUserId != null && mids.includes(sessionUserId)
          ? sessionUserId
          : [...mids].sort((a, b) => a - b)[0] ?? 0;
      const revSorted = [...revPool.user_ids].sort((a, b) => a - b);
      const reviewerPick =
        sessionUserId != null && revPool.user_ids.includes(sessionUserId)
          ? sessionUserId
          : revSorted[0] ?? 0;
      setAuthorId((prev) => (prev === 0 ? authorPick : prev));
      setReviewerId((prev) => (prev === 0 ? reviewerPick : prev));

      const warnings: string[] = [];
      let nextParent = '';

      if (duplicateFrom != null) {
        const listed = projectVers.find((v) => v.id === duplicateFrom);
        if (!listed) {
          warnings.push(`Template verification ${duplicateFrom} is not in this project`);
        } else {
          try {
            const source = await getVerification(duplicateFrom);
            if (source.project_id !== pid) {
              warnings.push(`Template verification ${duplicateFrom} is not in this project`);
            } else {
              const trimmedName = source.name.trim();
              setName(`${trimmedName || 'Untitled verification'} (Copy)`);
              setDescription(source.description);
              setSource(source.source || 'manual');
              setReferenceCode(nextDuplicateReference(source.reference_code, projectVers));
              setStatusId(source.status_id);
              setMethodId(
                source.verification_method_id != null ? String(source.verification_method_id) : '',
              );
              setReviewerId(source.reviewer_id);
              if (
                source.parent_id != null &&
                projectVers.some((v) => v.id === source.parent_id)
              ) {
                nextParent = String(source.parent_id);
              }
            }
          } catch {
            warnings.push(`Template verification ${duplicateFrom} is not in this project`);
          }
        }
      }

      if (parentQueryId != null) {
        if (!projectVers.some((v) => v.id === parentQueryId)) {
          warnings.push(`Parent verification ${parentQueryId} is not in this project`);
        } else {
          nextParent = String(parentQueryId);
        }
      }

      setParentId(nextParent);
      setQueryWarning(warnings.length > 0 ? warnings.join(' ') : null);
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : 'Failed to load form data');
    }
  }, [dashboard?.user, duplicateFrom, parentQueryId, pid]);

  useEffect(() => {
    void load();
  }, [load]);

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  const statusOptions = useMemo(() => {
    const forProject = statuses.filter((s) => s.project_id === pid);
    return forProject.length > 0 ? forProject : statuses;
  }, [statuses, pid]);

  const defaultAuthorVerificationStatusId = useMemo(
    () => initialVerificationStatusIdForAuthor(statusOptions),
    [statusOptions],
  );

  const statusChoicesForForm = useMemo(() => {
    if (!perms || perms.is_project_reviewer) return statusOptions;
    if (defaultAuthorVerificationStatusId == null) return statusOptions;
    return statusOptions.filter((s) => s.id === defaultAuthorVerificationStatusId);
  }, [perms, statusOptions, defaultAuthorVerificationStatusId]);

  useEffect(() => {
    if (!perms || perms.is_project_reviewer) return;
    if (defaultAuthorVerificationStatusId != null && defaultAuthorVerificationStatusId > 0) {
      setStatusId(defaultAuthorVerificationStatusId);
    }
  }, [perms, perms?.is_project_reviewer, defaultAuthorVerificationStatusId]);

  const statusMeta = useMemo(() => statuses.find((s) => s.id === statusId), [statuses, statusId]);

  const userLabel = useCallback(
    (id: number) => formatUserLabel(id, { users, members }),
    [users, members],
  );

  const blocker =
    projectReviewerIds.length === 0
      ? {
          message: 'This project has no reviewers, so verifications cannot be created.',
          linkLabel: 'Add a reviewer in Project settings',
          href: `${basePath}/settings`,
        }
      : null;

  const selectedParent = useMemo(() => {
    if (parentId === '') return null;
    const id = Number(parentId);
    if (!Number.isFinite(id)) return null;
    return existing.find((x) => x.id === id) ?? null;
  }, [parentId, existing]);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    const token = csrfToken ?? '';
    if (!token) {
      setSaveError('Missing CSRF token; refresh the page.');
      return;
    }
    setSaveError(null);
    setSaving(true);
    try {
      if (authorId <= 0 || reviewerId <= 0) {
        setSaveError('Select author (project member) and reviewer (must be a project reviewer).');
        setSaving(false);
        return;
      }
      const resolvedStatusId =
        perms && !perms.is_project_reviewer && defaultAuthorVerificationStatusId != null
          ? defaultAuthorVerificationStatusId
          : statusId;
      const { id: newId } = await createVerification(
        {
          name: name.trim(),
          reference_code: referenceCode.trim(),
          description: description.trim(),
          source: source.trim() || 'manual',
          status_id: resolvedStatusId,
          parent_id: parentId === '' ? null : Number(parentId),
          project_id: pid,
          verification_method_id: methodId === '' ? null : Number(methodId),
          author_id: authorId,
          reviewer_id: reviewerId,
        },
        token,
      );
      if (linkedReqIds.length > 0) {
        try {
          await putVerificationMatrix(pid, newId, { requirement_ids: linkedReqIds }, token);
        } catch (linkErr) {
          await refreshDashboard();
          const msg =
            linkErr instanceof Error ? linkErr.message : 'Failed to save traceability links';
          setSaveError(
            `Verification was created (id ${newId}), but matrix links could not be saved: ${msg}. Open it from the list and edit links there.`,
          );
          return;
        }
      }
      await refreshDashboard();
      navigate(`${basePath}/verifications/${newId}`);
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
          <Link
            to={`${basePath}/verifications`}
            className="font-semibold text-stitch-accent underline"
          >
            Back to verifications
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="font-body text-stitch-fg text-stitch max-w-4xl">
      <nav className="flex items-center gap-2 text-[10px] font-semibold text-stitch-muted mb-6 uppercase tracking-widest">
        <Link to={`${basePath}/verifications`} className="hover:text-stitch-accent transition-colors">
          Verifications
        </Link>
        <span className="material-symbols-outlined text-sm text-stitch-muted">chevron_right</span>
        <span className="text-stitch-accent font-bold">New</span>
      </nav>

      <div className="mb-8">
        <nav className="flex text-[10px] text-stitch-muted font-mono uppercase tracking-widest mb-1">
          <span>{projectName}</span>
          <span className="mx-2">/</span>
          <span className="text-stitch-accent font-bold">Create verification</span>
        </nav>
        <h1 className="text-2xl md:text-3xl font-extrabold text-stitch-fg tracking-tight font-headline">
          Create verification
        </h1>
        <p className="text-stitch-muted text-sm mt-2">
          Add a test / verification record for this project. Optionally link requirements for traceability and set a
          parent for hierarchy.
        </p>
      </div>

      {queryWarning ? (
        <div
          role="status"
          className="mb-6 rounded-xl border border-amber-500/25 bg-amber-500/10 p-3 text-sm text-amber-100"
        >
          {queryWarning}
        </div>
      ) : null}

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
              placeholder="VER-0001"
            />
          </div>
          <div>
            <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
              Name
            </label>
            <input
              required
              value={name}
              onChange={(e) => setName(e.target.value)}
              className={selectClass}
              placeholder="Verification title"
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
              rows={5}
              className={`${selectClass} min-h-[120px] resize-y`}
              placeholder="What is being verified…"
            />
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                Source
              </label>
              <input
                value={source}
                onChange={(e) => setSource(e.target.value)}
                className={selectClass}
                placeholder="manual, test_rig, …"
              />
            </div>
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                Status
              </label>
              {perms && !perms.is_project_reviewer ? (
                <p className="text-[11px] text-stitch-muted mb-2">
                  As a non-reviewer you can only create verifications in the initial status (e.g. not run). Ask a
                  project reviewer to change it after creation.
                </p>
              ) : null}
              <div className="flex items-center gap-2">
                <div
                  className="w-2 h-2 rounded-full shrink-0 bg-stitch-accent/40"
                  style={statusTagColorSwatchStyle(statusMeta?.tag_color)}
                  title={statusMeta?.tag_color ? 'Catalog color' : undefined}
                />
                <select
                  className={`${selectClass} flex-1 min-w-0`}
                  value={statusId}
                  disabled={perms != null && !perms.is_project_reviewer}
                  onChange={(e) => setStatusId(Number(e.target.value))}
                >
                  {statusChoicesForForm.map((s) => (
                    <option key={s.id} value={s.id} className="bg-stitch-surface text-stitch-fg">
                      {s.title}
                    </option>
                  ))}
                </select>
              </div>
            </div>
            <div>
              <label
                htmlFor="verification-parent"
                className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1"
              >
                Parent verification (optional)
              </label>
              <select
                id="verification-parent"
                className={selectClass}
                value={parentId}
                onChange={(e) => setParentId(e.target.value)}
              >
                <option value="" className="bg-stitch-surface text-stitch-fg">
                  None
                </option>
                {existing.map((v) => (
                  <option key={v.id} value={v.id} className="bg-stitch-surface text-stitch-fg">
                    {(v.reference_code ?? '').trim() || `#${v.id}`} —{' '}
                    {v.name.length > 48 ? `${v.name.slice(0, 48)}…` : v.name}
                  </option>
                ))}
              </select>
              {parentId !== '' ? (
                <div className="mt-1.5">
                  <Link
                    to={`${basePath}/verifications/${parentId}`}
                    className="text-xs font-mono font-semibold text-stitch-accent hover:underline"
                    title={selectedParent?.name?.trim() || undefined}
                  >
                    {(selectedParent?.reference_code ?? '').trim() || `#${parentId}`}
                  </Link>
                </div>
              ) : null}
            </div>
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                Verification method (optional)
              </label>
              <select
                className={selectClass}
                value={methodId}
                onChange={(e) => setMethodId(e.target.value)}
              >
                <option value="" className="bg-stitch-surface text-stitch-fg">
                  None
                </option>
                {methods.map((m) => (
                  <option key={m.id} value={m.id} className="bg-stitch-surface text-stitch-fg">
                    {m.title}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                Author
              </label>
              <select
                className={selectClass}
                value={authorId || ''}
                onChange={(e) => setAuthorId(Number(e.target.value))}
                required
              >
                <option value="" disabled>
                  Select…
                </option>
                {members.map((m) => (
                  <option key={m.user_id} value={m.user_id} className="bg-stitch-surface text-stitch-fg">
                    {userLabel(m.user_id)}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                Reviewer
              </label>
              {projectReviewerIds.length === 0 ? (
                <p className="text-xs text-stitch-muted py-2">
                  No project reviewers configured. Add them in{' '}
                  <Link to={`${basePath}/settings`} className="text-stitch-accent underline font-semibold">
                    Project settings
                  </Link>{' '}
                  before assigning a reviewer.
                </p>
              ) : (
                <select
                  className={selectClass}
                  value={reviewerId || ''}
                  onChange={(e) => setReviewerId(Number(e.target.value))}
                  required
                >
                  <option value="" disabled>
                    Select…
                  </option>
                  {[...projectReviewerIds].sort((a, b) => a - b).map((uid) => (
                    <option key={`r-${uid}`} value={uid} className="bg-stitch-surface text-stitch-fg">
                      {userLabel(uid)}
                    </option>
                  ))}
                </select>
              )}
            </div>
          </div>
        </section>

        <section className="bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch p-6 md:p-8 space-y-4">
          <div>
            <h2 className="text-sm font-bold font-headline text-stitch-fg">Traceability (requirements)</h2>
            <p className="text-xs text-stitch-muted mt-1">
              Optional: link this verification to requirements in the matrix after it is created.
            </p>
          </div>
          <RequirementMatrixPicker
            projectId={pid}
            basePath={basePath}
            requirements={requirements}
            selectedIds={linkedReqIds}
            onChange={setLinkedReqIds}
            disabled={saving}
          />
        </section>

        {saveError && (
          <div className="rounded-lg bg-red-500/15 border border-red-500/30 text-red-100 text-sm px-4 py-3">
            {saveError}
          </div>
        )}

        <footer className="sticky bottom-0 z-30 bg-stitch-surface/85 backdrop-blur-md border-t border-stitch-border px-4 md:px-8 py-3 flex flex-wrap items-center justify-between gap-3">
          <Link
            to={`${basePath}/verifications`}
            className="text-xs font-bold uppercase tracking-wider text-stitch-muted hover:text-stitch-danger transition-colors px-2 py-2"
          >
            Cancel
          </Link>
          <div className="flex flex-wrap items-center justify-end gap-3">
            {blocker ? (
              <p className="text-xs text-amber-200/90 max-w-md">
                {blocker.message}{' '}
                <Link to={blocker.href} className="text-stitch-accent underline font-semibold">
                  {blocker.linkLabel}
                </Link>
                .
              </p>
            ) : null}
            <button
              type="submit"
              disabled={saving || blocker != null}
              className="bg-stitch-accent text-stitch-canvas px-6 py-2.5 rounded-md text-xs font-bold uppercase tracking-widest shadow-stitch disabled:opacity-50 hover:bg-stitch-accent-dim transition-colors"
            >
              {saving ? 'Creating…' : 'Create verification'}
            </button>
          </div>
        </footer>
      </form>
    </div>
  );
}

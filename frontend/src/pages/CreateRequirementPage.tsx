import { FormEvent, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link, useNavigate, useOutletContext, useSearchParams } from 'react-router-dom';
import {
  ApiError,
  createRequirementByProject,
  getMyPermissions,
  getProjectReviewers,
  getRequirementByProject,
  listApplicability,
  listCategories,
  listCustomFieldsByProject,
  listProjectMembers,
  listRequirementStatuses,
  listRequirements,
  listRequirementVersionLinkTypes,
  listUsersOptional,
  listVerificationMethodsByProject,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type {
  Applicability,
  Category,
  CustomFieldDefinition,
  EffectivePermissions,
  ProjectMember,
  Requirement,
  RequirementStatus,
  User,
  VerificationMethod,
} from '@/api/types';
import { statusTagColorSwatchStyle } from '@/components/StatusBadge';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import { authorDefaultRequirementStatusId } from '@/statusAuthorDefaults';
import { parseUser } from '@/utils/parseUser';
import { formatUserLabel } from '@/utils/userLabel';
import {
  duplicateRequirementTitle,
  nextDuplicateReference,
} from '@/utils/duplicateRequirement';
import { duplicateSourceQueryId, parsePositiveQueryId } from '@/utils/createQueryParams';
import StatementEditor from '@/components/StatementEditor';
import DraftStatus from '@/components/DraftStatus';
import { useDraftAutosave } from '@/hooks/useDraftAutosave';
import {
  clearDraft,
  formatDraftTime,
  readDraft,
  requirementDraftKey,
  type RequirementDraft,
} from '@/utils/requirementDraft';

type ParentLinkDraft = { target_version_id: number; link_type: string; rationale: string | null };

/** The form fields kept in a local draft of a new requirement (issue #255). */
type CreateDraftValues = {
  title: string;
  description: string;
  referenceCode: string;
  justification: string;
  categoryId: number;
  applicabilityId: number;
  reviewerId: number;
  methodIds: number[];
  customFieldValues: Record<number, string>;
  parentLinks: ParentLinkDraft[];
};

/** The typed text of the form, compared to decide whether there is anything to keep. */
function typedText(v: {
  title: string;
  description: string;
  referenceCode: string;
  justification: string;
  customFieldValues: Record<number, string>;
}): string {
  const custom = Object.entries(v.customFieldValues)
    .filter(([, value]) => value.trim() !== '')
    .sort(([a], [b]) => Number(a) - Number(b));
  return JSON.stringify([v.title, v.description, v.referenceCode, v.justification, custom]);
}

const selectClass =
  'w-full text-sm font-medium bg-stitch-elevated border border-stitch-border rounded-md px-2 py-2 text-stitch-fg focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/40 outline-hidden transition-colors';

function enumValues(value: unknown): string[] {
  return Array.isArray(value) ? value.map(String) : [];
}

export default function CreateRequirementPage() {
  const { basePath, projectId: pid } = useOutletContext<ProjectOutletContext>();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const { csrfToken, dashboard, refresh: refreshDashboard } = useDashboard();

  const me = useMemo(() => parseUser(dashboard?.user), [dashboard?.user]);
  const duplicateFrom = duplicateSourceQueryId(searchParams);
  const parentQueryId = parsePositiveQueryId(searchParams.get('parent'));
  const isDuplicate = duplicateFrom != null;

  const [statuses, setStatuses] = useState<RequirementStatus[]>([]);
  const [categories, setCategories] = useState<Category[]>([]);
  const [applicability, setApplicability] = useState<Applicability[]>([]);
  const [members, setMembers] = useState<ProjectMember[]>([]);
  const [projectReviewerIds, setProjectReviewerIds] = useState<number[]>([]);
  const [perms, setPerms] = useState<EffectivePermissions | null>(null);
  const [methods, setMethods] = useState<VerificationMethod[]>([]);
  const [requirements, setRequirements] = useState<Requirement[]>([]);
  const [customFields, setCustomFields] = useState<CustomFieldDefinition[]>([]);
  const [linkTypes, setLinkTypes] = useState<string[]>([]);
  const [users, setUsers] = useState<User[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [queryWarning, setQueryWarning] = useState<string | null>(null);
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
  const [customFieldValues, setCustomFieldValues] = useState<Record<number, string>>({});
  const [parentLinks, setParentLinks] = useState<
    Array<{ target_version_id: number; link_type: string; rationale: string | null }>
  >([]);
  const [newParentId, setNewParentId] = useState<number | ''>('');
  const [newLinkType, setNewLinkType] = useState('');
  /** The typed text right after loading (empty, or the copied template); `null` while loading. */
  const [pristineText, setPristineText] = useState<string | null>(null);

  const loadSeq = useRef(0);
  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    // Only the latest load may fill the form (StrictMode runs the effect twice).
    const seq = ++loadSeq.current;
    setLoadError(null);
    setQueryWarning(null);
    try {
      const [st, cat, app, mem, meth, revPool, p, u, reqs, fields, types] = await Promise.all([
        listRequirementStatuses(),
        listCategories(),
        listApplicability(),
        listProjectMembers(pid),
        listVerificationMethodsByProject(pid),
        getProjectReviewers(pid).catch(() => ({ user_ids: [] as number[] })),
        getMyPermissions(pid).catch(() => null),
        listUsersOptional(),
        listRequirements(pid),
        listCustomFieldsByProject(pid),
        listRequirementVersionLinkTypes(pid),
      ]);
      if (seq !== loadSeq.current) return;
      setPerms(p);
      setStatuses(st);
      setCategories(cat.filter((c) => c.project_id === pid));
      setApplicability(app.filter((a) => a.project_id === pid));
      setMembers(mem);
      setMethods(meth);
      setRequirements(reqs);
      setCustomFields(fields);
      setLinkTypes(types);
      setNewLinkType(types[0] ?? 'derives-from');
      setUsers(u);
      setProjectReviewerIds(revPool.user_ids);
      const statusOpts = st.filter((s) => s.project_id === pid);
      const useStatuses = statusOpts.length > 0 ? statusOpts : st;
      if (useStatuses[0]) setStatusId((id) => (id === 0 ? useStatuses[0]!.id : id));
      const firstCat = cat.find((c) => c.project_id === pid);
      if (firstCat) setCategoryId((id) => (id === 0 ? firstCat.id : id));
      const firstApp = app.find((a) => a.project_id === pid);
      if (firstApp) setApplicabilityId((id) => (id === 0 ? firstApp.id : id));
      setMethodIds((prev) => (prev.length === 0 && meth[0] ? [meth[0]!.id] : prev));

      const warnings: string[] = [];
      const defaultLinkType = types[0] ?? 'derives-from';
      let links: ParentLinkDraft[] = [];
      let loadedText = typedText({
        title: '',
        description: '',
        referenceCode: '',
        justification: '',
        customFieldValues: {},
      });

      if (duplicateFrom != null) {
        const listed = reqs.find((requirement) => requirement.id === duplicateFrom);
        if (!listed) {
          warnings.push(`Template requirement ${duplicateFrom} is not in this project`);
        } else {
          try {
            const source = await getRequirementByProject(pid, duplicateFrom);
            if (seq !== loadSeq.current) return;
            if (source.project_id !== pid) {
              warnings.push(`Template requirement ${duplicateFrom} is not in this project`);
            } else {
              setTitle(duplicateRequirementTitle(source.title));
              setDescription(source.description);
              setReferenceCode(nextDuplicateReference(source.reference_code, reqs));
              setStatusId(source.status_id);
              setCategoryId(source.category_id);
              setApplicabilityId(source.applicability_id);
              setReviewerId(source.reviewer_id);
              setJustification(source.justification ?? '');
              setMethodIds(source.verification_method_ids ?? []);
              const copiedFields: Record<number, string> = Object.fromEntries(
                (source.custom_fields ?? []).map((field) => [field.field_id, field.value ?? '']),
              );
              setCustomFieldValues(copiedFields);
              loadedText = typedText({
                title: duplicateRequirementTitle(source.title),
                description: source.description,
                referenceCode: nextDuplicateReference(source.reference_code, reqs),
                justification: source.justification ?? '',
                customFieldValues: copiedFields,
              });
              links = source.trace_summary.parent_links.map((link) => ({
                target_version_id: link.target_version_id,
                link_type: link.link_type,
                rationale: link.rationale,
              }));
            }
          } catch {
            warnings.push(`Template requirement ${duplicateFrom} is not in this project`);
          }
        }
      }

      if (parentQueryId != null) {
        const parent = reqs.find((requirement) => requirement.id === parentQueryId);
        if (!parent) {
          warnings.push(`Parent requirement ${parentQueryId} is not in this project`);
        } else if (parent.current_version_id == null) {
          warnings.push(`Parent requirement ${parentQueryId} has no current version`);
        } else if (!links.some((link) => link.target_version_id === parent.current_version_id)) {
          links = [
            ...links,
            {
              target_version_id: parent.current_version_id,
              link_type: defaultLinkType,
              rationale: null,
            },
          ];
        }
      }

      setParentLinks(links);
      setQueryWarning(warnings.length > 0 ? warnings.join(' ') : null);
      setPristineText(loadedText);
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : 'Failed to load form data');
    }
  }, [duplicateFrom, parentQueryId, pid]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (me?.id) {
      setAuthorId((a) => (a === 0 ? me.id : a));
    }
  }, [me?.id]);

  useEffect(() => {
    setReviewerId((r) => {
      if (projectReviewerIds.length === 0) return 0;
      if (r !== 0 && projectReviewerIds.includes(r)) return r;
      const uid = me?.id;
      if (uid != null && projectReviewerIds.includes(uid)) return uid;
      return [...projectReviewerIds].sort((a, b) => a - b)[0] ?? 0;
    });
  }, [me?.id, projectReviewerIds]);

  const userLabel = useCallback(
    (id: number) => formatUserLabel(id, { users, members, me }),
    [users, members, me],
  );

  const authorOptionIds = useMemo(() => {
    const ids = new Set(members.map((m) => m.user_id));
    if (me?.id) ids.add(me.id);
    if (authorId > 0) ids.add(authorId);
    return [...ids].sort((a, b) => a - b);
  }, [members, me?.id, authorId]);

  const requirementByVersionId = useMemo(
    () =>
      new Map(
        requirements
          .filter((requirement) => requirement.current_version_id != null)
          .map((requirement) => [requirement.current_version_id!, requirement]),
      ),
    [requirements],
  );

  const parentCandidates = useMemo(() => {
    const linkedVersionIds = new Set(parentLinks.map((link) => link.target_version_id));
    return requirements.filter(
      (requirement) =>
        requirement.current_version_id != null &&
        !linkedVersionIds.has(requirement.current_version_id),
    );
  }, [parentLinks, requirements]);

  const addParentLink = () => {
    if (newParentId === '') return;
    const parent = requirements.find((requirement) => requirement.id === newParentId);
    if (parent?.current_version_id == null) return;
    setParentLinks((links) => [
      ...links,
      {
        target_version_id: parent.current_version_id!,
        link_type: newLinkType || linkTypes[0] || 'derives-from',
        rationale: null,
      },
    ]);
    setNewParentId('');
  };

  const statusOptions = useMemo(() => {
    const forProject = statuses.filter((s) => s.project_id === pid);
    return forProject.length > 0 ? forProject : statuses;
  }, [statuses, pid]);

  const defaultAuthorRequirementStatusId = useMemo(
    () => authorDefaultRequirementStatusId(statusOptions),
    [statusOptions],
  );

  const statusChoicesForForm = useMemo(() => {
    if (!perms || perms.is_project_reviewer) return statusOptions;
    if (defaultAuthorRequirementStatusId == null) return statusOptions;
    return statusOptions.filter((s) => s.id === defaultAuthorRequirementStatusId);
  }, [perms, statusOptions, defaultAuthorRequirementStatusId]);

  useEffect(() => {
    if (!perms || perms.is_project_reviewer) return;
    if (defaultAuthorRequirementStatusId != null && defaultAuthorRequirementStatusId > 0) {
      setStatusId(defaultAuthorRequirementStatusId);
    }
  }, [perms, perms?.is_project_reviewer, defaultAuthorRequirementStatusId]);

  const statusMeta = useMemo(() => statuses.find((s) => s.id === statusId), [statuses, statusId]);

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  const blocker =
    methods.length === 0
      ? {
          message: 'This project has no verification methods, so requirements cannot be created.',
          linkLabel: 'Add a verification method',
          href: `${basePath}/settings/catalog/verification-methods`,
        }
      : projectReviewerIds.length === 0
        ? {
            message: 'This project has no reviewers, so requirements cannot be created.',
            linkLabel: 'Add a reviewer in Project settings',
            href: `${basePath}/settings/members`,
          }
        : null;

  // ---- Local draft (issue #255) -------------------------------------------
  const draftKey = me && Number.isFinite(pid) ? requirementDraftKey(me.id, pid, 'new') : null;
  const [draftChecked, setDraftChecked] = useState(false);
  const [draftOffer, setDraftOffer] = useState<RequirementDraft<CreateDraftValues> | null>(null);

  const draftValues = useMemo<CreateDraftValues>(
    () => ({
      title,
      description,
      referenceCode,
      justification,
      categoryId,
      applicabilityId,
      reviewerId,
      methodIds,
      customFieldValues,
      parentLinks,
    }),
    [
      title,
      description,
      referenceCode,
      justification,
      categoryId,
      applicabilityId,
      reviewerId,
      methodIds,
      customFieldValues,
      parentLinks,
    ],
  );

  const dirty =
    pristineText != null &&
    typedText({ title, description, referenceCode, justification, customFieldValues }) !== pristineText;

  const autosave = useDraftAutosave({
    key: draftChecked ? draftKey : null,
    dirty,
    values: draftValues,
    baseVersionId: null,
    // Never overwrite a stored draft while it is still on offer.
    paused: draftOffer != null,
  });

  // After loading, offer a draft left by an earlier visit (never applied by itself:
  // the defaults and any ?from= / ?parent= prefill are already in the form).
  useEffect(() => {
    if (draftChecked || pristineText == null || !draftKey) return;
    setDraftChecked(true);
    const stored = readDraft<CreateDraftValues>(draftKey);
    if (stored) setDraftOffer(stored);
  }, [draftChecked, pristineText, draftKey]);

  function restoreDraft() {
    if (!draftOffer) return;
    const v = draftOffer.values;
    setTitle(v.title);
    setDescription(v.description);
    setReferenceCode(v.referenceCode);
    setJustification(v.justification);
    if (categories.some((c) => c.id === v.categoryId)) setCategoryId(v.categoryId);
    if (applicability.some((a) => a.id === v.applicabilityId)) setApplicabilityId(v.applicabilityId);
    if (projectReviewerIds.includes(v.reviewerId)) setReviewerId(v.reviewerId);
    const knownMethods = v.methodIds.filter((id) => methods.some((m) => m.id === id));
    if (knownMethods.length > 0) setMethodIds(knownMethods);
    setCustomFieldValues(
      Object.fromEntries(
        Object.entries(v.customFieldValues).filter(([id]) =>
          customFields.some((field) => field.id === Number(id)),
        ),
      ),
    );
    setParentLinks(v.parentLinks.filter((link) => requirementByVersionId.has(link.target_version_id)));
    setDraftOffer(null);
  }

  function discardDraft() {
    if (draftKey) clearDraft(draftKey);
    setDraftOffer(null);
  }

  function cancelCreating(e: React.MouseEvent) {
    if (dirty && !window.confirm('Discard unsaved changes?')) {
      e.preventDefault();
      return;
    }
    autosave.discard();
  }

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
      const resolvedStatusId =
        perms && !perms.is_project_reviewer && defaultAuthorRequirementStatusId != null
          ? defaultAuthorRequirementStatusId
          : statusId;
      const { id } = await createRequirementByProject(
        pid,
        {
          title: title.trim(),
          description: description.trim(),
          reference_code: referenceCode.trim(),
          status_id: resolvedStatusId,
          category_id: categoryId,
          applicability_id: applicabilityId,
          author_id: authorId,
          reviewer_id: reviewerId,
          justification: justification.trim() || null,
          project_id: pid,
          verification_method_ids: methodIds,
          custom_fields: customFields.map((field) => ({
            field_id: field.id,
            value: customFieldValues[field.id]?.trim() || null,
          })),
          parent_links: parentLinks,
        },
        token,
      );
      // Created: the local draft has done its job.
      autosave.discard();
      await refreshDashboard();
      navigate(`${basePath}/requirements/${id}/edit`);
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) {
        setSaveError(
          'Your session has expired. Your draft is kept on this device: sign in again and open New requirement to restore it.',
        );
      } else {
        setSaveError(err instanceof Error ? err.message : 'Create failed');
      }
    } finally {
      setSaving(false);
    }
  }

  if (loadError) {
    return (
      <div className="rounded-xl border border-red-500/30 bg-red-500/10 p-4 text-sm text-red-200">
        {loadError}
        <div className="mt-3">
          <Link to={`${basePath}/requirements`} className="font-semibold text-stitch-accent underline">
            Back to requirements
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="font-body text-stitch-fg text-stitch max-w-4xl">
      <nav className="flex items-center gap-2 text-[10px] font-semibold text-stitch-muted mb-6 uppercase tracking-widest">
        <Link to={`${basePath}/requirements`} className="hover:text-stitch-accent transition-colors">
          Requirements
        </Link>
        <span className="material-symbols-outlined text-sm text-stitch-muted">chevron_right</span>
        <span className="text-stitch-accent font-bold">
          {isDuplicate ? 'Duplicate requirement' : 'New requirement'}
        </span>
      </nav>

      <div className="flex flex-col sm:flex-row sm:items-end sm:justify-between gap-4 mb-8">
        <div>
          <nav className="flex text-[10px] text-stitch-muted font-mono uppercase tracking-widest mb-1">
            <span>{projectName}</span>
            <span className="mx-2">/</span>
            <span className="text-stitch-accent font-bold">
              {isDuplicate ? 'Duplicate' : 'Create'}
            </span>
          </nav>
          <h1 className="text-2xl md:text-3xl font-extrabold text-stitch-fg tracking-tight font-headline">
            {isDuplicate ? 'Duplicate requirement' : 'Create requirement'}
          </h1>
          <p className="text-stitch-muted text-sm mt-2">
            {isDuplicate
              ? 'Review the copied fields and assign a unique reference before creating the independent requirement.'
              : 'A verification method link is required by the API for each new requirement.'}
          </p>
        </div>
      </div>

      {queryWarning ? (
        <div
          role="status"
          className="mb-6 rounded-xl border border-amber-500/25 bg-amber-500/10 p-3 text-sm text-amber-100"
        >
          {queryWarning}
        </div>
      ) : null}

      {draftOffer ? (
        <div
          role="region"
          aria-label="Unsaved draft"
          className="mb-6 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-amber-600/35 bg-amber-500/10 px-4 py-3 text-sm text-amber-900 dark:text-amber-100"
        >
          <p>
            You have an unsaved new requirement from {formatDraftTime(draftOffer.savedAt)}
            {draftOffer.values.title.trim() ? `: “${draftOffer.values.title.trim()}”` : ''}. Restore it?
          </p>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={restoreDraft}
              className="rounded-md border border-amber-700/40 px-3 py-1.5 text-xs font-bold uppercase tracking-wider hover:bg-amber-500/15"
            >
              Restore draft
            </button>
            <button
              type="button"
              onClick={discardDraft}
              className="rounded-md px-3 py-1.5 text-xs font-bold uppercase tracking-wider hover:bg-amber-500/15"
            >
              Discard draft
            </button>
          </div>
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
            <label
              htmlFor="requirement-statement"
              className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1"
            >
              Description
            </label>
            <StatementEditor
              id="requirement-statement"
              value={description}
              onChange={setDescription}
              required
              placeholder="Requirement statement…"
              minHeightClass="min-h-[140px]"
              textareaClassName={`${selectClass} resize-y`}
            />
          </div>
          <div>
            <label
              htmlFor="requirement-rationale"
              className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1"
            >
              Rationale (optional)
            </label>
            <textarea
              id="requirement-rationale"
              value={justification}
              onChange={(e) => setJustification(e.target.value)}
              rows={4}
              className={`${selectClass} min-h-[96px] resize-y`}
              placeholder="Why this requirement exists…"
            />
          </div>
          <div>
            <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-2">
              Verification methods <span className="text-stitch-danger">*</span>
            </label>
            {methods.length === 0 ? (
              <p className="text-sm text-amber-200/90 bg-amber-500/10 border border-amber-500/25 rounded-lg p-3">
                No verification methods in this project.{' '}
                <Link
                  to={`${basePath}/settings/catalog/verification-methods`}
                  className="text-stitch-accent underline font-semibold"
                >
                  Add one in Project settings › Catalog
                </Link>
                , then reload this page.
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
              {perms && !perms.is_project_reviewer ? (
                <p className="text-[11px] text-stitch-muted mb-2">
                  As a non-reviewer you can only create requirements in the default draft (initial) status. Ask a
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
                Author
              </label>
              <select
                className={selectClass}
                value={authorId}
                onChange={(e) => setAuthorId(Number(e.target.value))}
              >
                {authorOptionIds.map((id) => (
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
              {projectReviewerIds.length === 0 ? (
                <p className="text-xs text-stitch-muted py-2">
                  No project reviewers configured. Add them in{' '}
                  <Link to={`${basePath}/settings/members`} className="text-stitch-accent underline font-semibold">
                    Project settings
                  </Link>{' '}
                  before assigning a reviewer.
                </p>
              ) : (
                <select
                  className={selectClass}
                  value={reviewerId}
                  onChange={(e) => setReviewerId(Number(e.target.value))}
                >
                  {[...projectReviewerIds].sort((a, b) => a - b).map((id) => (
                    <option key={`r-${id}`} value={id} className="bg-stitch-surface text-stitch-fg">
                      {userLabel(id)}
                    </option>
                  ))}
                </select>
              )}
            </div>
          </div>

          {customFields.length > 0 ? (
            <div className="border-t border-stitch-border pt-6">
              <h2 className="mb-4 text-xs font-bold uppercase tracking-widest text-stitch-fg">
                Custom fields
              </h2>
              <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
                {[...customFields]
                  .sort((a, b) => a.sort_order - b.sort_order || a.label.localeCompare(b.label))
                  .map((field) => (
                    <label
                      key={field.id}
                      className="block text-[10px] font-bold uppercase tracking-wider text-stitch-muted"
                    >
                      {field.label}
                      {field.field_type === 'enum' ? (
                        <select
                          value={customFieldValues[field.id] ?? ''}
                          onChange={(event) =>
                            setCustomFieldValues((values) => ({
                              ...values,
                              [field.id]: event.target.value,
                            }))
                          }
                          className={`${selectClass} mt-1 font-normal normal-case tracking-normal`}
                        >
                          <option value="">—</option>
                          {enumValues(field.enum_values).map((value) => (
                            <option key={value} value={value}>
                              {value}
                            </option>
                          ))}
                        </select>
                      ) : field.field_type === 'boolean' ? (
                        <select
                          value={customFieldValues[field.id] ?? ''}
                          onChange={(event) =>
                            setCustomFieldValues((values) => ({
                              ...values,
                              [field.id]: event.target.value,
                            }))
                          }
                          className={`${selectClass} mt-1 font-normal normal-case tracking-normal`}
                        >
                          <option value="">—</option>
                          <option value="true">Yes</option>
                          <option value="false">No</option>
                        </select>
                      ) : (
                        <input
                          type={field.field_type === 'number' ? 'number' : 'text'}
                          value={customFieldValues[field.id] ?? ''}
                          onChange={(event) =>
                            setCustomFieldValues((values) => ({
                              ...values,
                              [field.id]: event.target.value,
                            }))
                          }
                          className={`${selectClass} mt-1 font-normal normal-case tracking-normal`}
                        />
                      )}
                    </label>
                  ))}
              </div>
            </div>
          ) : null}

          <div className="border-t border-stitch-border pt-6">
            <h2 className="text-xs font-bold uppercase tracking-widest text-stitch-fg">
              Parent requirements
            </h2>
            <p className="mb-4 mt-1 text-xs text-stitch-muted">
              Parent links are copied when duplicating and can be adjusted before creation.
            </p>
            <div className="space-y-2">
              {parentLinks.length === 0 ? (
                <p className="text-xs text-stitch-muted">No parent requirements selected.</p>
              ) : (
                parentLinks.map((link, index) => {
                  const parent = requirementByVersionId.get(link.target_version_id);
                  return (
                    <div
                      key={`${link.target_version_id}-${link.link_type}-${index}`}
                      className="flex items-center justify-between gap-3 rounded-lg border border-stitch-border bg-stitch-elevated px-3 py-2"
                    >
                      <span className="text-xs text-stitch-fg">
                        <span className="font-mono font-bold text-stitch-accent">
                          {parent?.reference_code || `Version #${link.target_version_id}`}
                        </span>{' '}
                        · {link.link_type}
                      </span>
                      <button
                        type="button"
                        aria-label="Remove parent requirement"
                        onClick={() =>
                          setParentLinks((links) => links.filter((_, itemIndex) => itemIndex !== index))
                        }
                        className="text-stitch-muted hover:text-stitch-danger"
                      >
                        <span className="material-symbols-outlined text-lg">link_off</span>
                      </button>
                    </div>
                  );
                })
              )}
            </div>
            <div className="mt-3 grid gap-2 sm:grid-cols-[1fr_12rem_auto]">
              <select
                value={newParentId}
                onChange={(event) =>
                  setNewParentId(event.target.value ? Number(event.target.value) : '')
                }
                className={selectClass}
              >
                <option value="">Select requirement…</option>
                {parentCandidates.map((requirement) => (
                  <option key={requirement.id} value={requirement.id}>
                    {requirement.reference_code || `#${requirement.id}`} — {requirement.title}
                  </option>
                ))}
              </select>
              <select
                value={newLinkType}
                onChange={(event) => setNewLinkType(event.target.value)}
                className={selectClass}
              >
                {linkTypes.map((linkType) => (
                  <option key={linkType} value={linkType}>
                    {linkType}
                  </option>
                ))}
              </select>
              <button
                type="button"
                disabled={newParentId === ''}
                onClick={addParentLink}
                className="rounded-md border border-stitch-border px-4 py-2 text-xs font-bold uppercase tracking-wider text-stitch-accent disabled:opacity-40"
              >
                Add parent
              </button>
            </div>
          </div>
        </section>

        {saveError && (
          <div className="rounded-lg bg-red-500/15 border border-red-500/30 text-red-100 text-sm px-4 py-3">
            {saveError}
          </div>
        )}

        <footer className="sticky bottom-0 z-30 bg-stitch-surface/85 backdrop-blur-md border-t border-stitch-border px-4 md:px-8 py-3 flex flex-wrap items-center justify-between gap-3">
          <Link
            to={`${basePath}/requirements`}
            onClick={cancelCreating}
            className="text-xs font-bold uppercase tracking-wider text-stitch-muted hover:text-stitch-danger transition-colors px-2 py-2"
          >
            Cancel
          </Link>
          <div className="flex flex-wrap items-center justify-end gap-3">
            <DraftStatus status={autosave.status} savedAt={autosave.savedAt} dirty={dirty} saving={saving} />
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
              {saving ? 'Creating…' : isDuplicate ? 'Create duplicate' : 'Create requirement'}
            </button>
          </div>
        </footer>
      </form>
    </div>
  );
}

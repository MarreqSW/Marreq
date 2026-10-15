import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Link, useNavigate, useOutletContext, useParams, useSearchParams } from 'react-router-dom';
import {
  type ApiError,
  applyUseCaseOperations,
  clearUseCaseLinkSuspect,
  createUseCaseLink,
  deleteUseCase,
  deleteUseCaseLink,
  discardUseCaseDraft,
  getMyPermissions,
  listProjectMembers,
  listUseCaseComments,
  getUseCase,
  getUseCaseDraft,
  listUseCaseLinks,
  listUseCaseVersions,
  publishUseCaseDraft,
  setUseCaseVersionApproval,
} from '@/api/client';
import type {
  EffectivePermissions,
  ProjectMember,
  UseCaseComment,
  UseCaseDetail,
  UseCaseDocument,
  UseCaseDraft,
  UseCaseDraftSummary,
  UseCaseEffect,
  UseCaseLabels,
  UseCaseLink,
  UseCaseOperation,
  UseCaseVersionSummary,
} from '@/api/types';
import Dialog from '@/components/Dialog';
import DeleteStepDialog, { type DependentBranch } from '@/components/useCases/DeleteStepDialog';
import LinkDialog, { type LinkableElement } from '@/components/useCases/LinkDialog';
import RelatedArtifactsPanel from '@/components/useCases/RelatedArtifactsPanel';
import RequirementPeek from '@/components/useCases/RequirementPeek';
import UseCaseDiscussion from '@/components/useCases/UseCaseDiscussion';
import UseCaseDocumentView from '@/components/useCases/UseCaseDocumentView';
import UseCaseReviewView from '@/components/useCases/UseCaseReviewView';
import { useDashboard } from '@/context/DashboardContext';
import { useProjectArchived } from '@/hooks/useProjectArchived';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import {
  approvalChip,
  formatRelative,
  inverseOperations,
  locate,
} from '@/utils/useCaseDocument';

type Editor = {
  doc: UseCaseDocument;
  labels: UseCaseLabels;
  /** `null` until the first write opens a draft from the current version. */
  revision: number | null;
  baseVersionId: number | null;
};

type Entry = { forward: UseCaseOperation[]; inverse: UseCaseOperation[] };

type Toast = { message: string; undo: boolean };

type DependentsPrompt = {
  op: Extract<UseCaseOperation, { op: 'remove_element' }>;
  stepLabel: string;
  stepText: string;
  branches: DependentBranch[];
};

/** The JSON body of an API error (see {@link ApiError}), for its `code` and details. */
function errorBody(e: unknown): Record<string, unknown> | null {
  const body = e && typeof e === 'object' ? (e as Partial<ApiError>).body : undefined;
  return body && typeof body === 'object' ? (body as Record<string, unknown>) : null;
}

function describeEffects(effects: UseCaseEffect[], doc: UseCaseDocument): string | null {
  const parts: string[] = [];
  for (const e of effects) {
    const title = doc.branches.find((b) => b.id === e.id)?.title;
    if (e.type === 'relabelled') {
      parts.push(`“${title ?? e.id}” followed its step: now ${e.to}.`);
    } else if (e.type === 'removed') {
      parts.push(`A branch was deleted with its step.`);
    } else if (e.type === 'rebranched') {
      parts.push(`“${title ?? e.id}” now branches from another step.`);
    }
  }
  return parts.length ? parts.join(' ') : null;
}

/** A use case: read the current or an older version, edit the shared draft, publish and approve (issue #399). */
export default function ViewUseCasePage() {
  const { projectId: pid, basePath } = useOutletContext<ProjectOutletContext>();
  const { useCaseId } = useParams();
  const id = Number(useCaseId);
  const [params, setParams] = useSearchParams();
  const versionParam = params.get('version');
  const requestedVersion = versionParam ? Number(versionParam) : undefined;
  const navigate = useNavigate();
  const { csrfToken } = useDashboard();
  const archived = useProjectArchived(pid);

  const [detail, setDetail] = useState<UseCaseDetail | null>(null);
  const [versions, setVersions] = useState<UseCaseVersionSummary[]>([]);
  const [perms, setPerms] = useState<EffectivePermissions | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [editor, setEditor] = useState<Editor | null>(null);
  const [selectedBranch, setSelectedBranch] = useState<string | null>(null);
  const [undo, setUndo] = useState<Entry[]>([]);
  const [redo, setRedo] = useState<Entry[]>([]);
  const [saving, setSaving] = useState(false);
  const [editError, setEditError] = useState<string | null>(null);
  const [conflict, setConflict] = useState<UseCaseDraftSummary | null | undefined>(undefined);
  const [toast, setToast] = useState<Toast | null>(null);
  const [dependents, setDependents] = useState<DependentsPrompt | null>(null);
  const [publishOpen, setPublishOpen] = useState(false);
  const [changeNote, setChangeNote] = useState('');
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [links, setLinks] = useState<UseCaseLink[]>([]);
  const [linkDialog, setLinkDialog] = useState<{ elementId: string | null } | null>(null);
  const [linkError, setLinkError] = useState<string | null>(null);
  const [peek, setPeek] = useState<UseCaseLink | null>(null);
  const [comments, setComments] = useState<UseCaseComment[]>([]);
  const [members, setMembers] = useState<ProjectMember[]>([]);
  const [commentFocus, setCommentFocus] = useState<string | null>(null);
  const reviewing = params.get('view') === 'review';

  const editorRef = useRef<Editor | null>(null);
  editorRef.current = editor;
  const queue = useRef<Promise<unknown>>(Promise.resolve());

  const editing = editor != null;
  const canEdit = Boolean(perms?.edit_requirements) && !archived && requestedVersion == null;
  const canApprove = Boolean(perms?.is_project_reviewer) && !archived;

  const load = useCallback(async () => {
    setLoadError(null);
    try {
      const [d, v, p, l, c, m] = await Promise.all([
        getUseCase(pid, id, requestedVersion),
        listUseCaseVersions(pid, id),
        getMyPermissions(pid).catch(() => null),
        listUseCaseLinks(pid, id).catch(() => [] as UseCaseLink[]),
        listUseCaseComments(pid, id).catch(() => [] as UseCaseComment[]),
        listProjectMembers(pid).catch(() => [] as ProjectMember[]),
      ]);
      setDetail(d);
      setVersions(v);
      setPerms(p);
      setLinks(l);
      setComments(c);
      setMembers(m);
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : 'Failed to load the use case');
    }
  }, [pid, id, requestedVersion]);

  useEffect(() => {
    void load();
  }, [load]);

  const openEditor = useCallback(async () => {
    setEditError(null);
    setConflict(undefined);
    setUndo([]);
    setRedo([]);
    try {
      const fresh = await getUseCase(pid, id);
      const draft: UseCaseDraft | null = fresh.draft ? await getUseCaseDraft(pid, id) : null;
      if (draft) {
        setEditor({
          doc: draft.document,
          labels: draft.labels,
          revision: draft.revision,
          baseVersionId: draft.base_version_id,
        });
      } else {
        if (!fresh.current_version) throw new Error('This use case has nothing to edit.');
        setEditor({
          doc: fresh.current_version.document,
          labels: fresh.current_version.labels,
          revision: null,
          baseVersionId: fresh.current_version.id,
        });
      }
    } catch (e) {
      setActionError(e instanceof Error ? e.message : 'Failed to open the draft');
    }
  }, [pid, id]);

  // `?edit=1` opens the editor (used right after creating a use case).
  useEffect(() => {
    if (params.get('edit') === '1' && detail && perms && canEdit && !editing) {
      void openEditor();
      const next = new URLSearchParams(params);
      next.delete('edit');
      setParams(next, { replace: true });
    }
  }, [params, detail, perms, canEdit, editing, openEditor, setParams]);

  useEffect(() => {
    if (!toast) return;
    const t = window.setTimeout(() => setToast(null), 7000);
    return () => window.clearTimeout(t);
  }, [toast]);

  /** Send operations; `record` adds them to the undo history. */
  const send = useCallback(
    (ops: UseCaseOperation[], record: 'new' | 'undo' | 'redo' = 'new'): Promise<void> => {
      const run = async () => {
        const current = editorRef.current;
        if (!current || !csrfToken) return;
        const inverse = inverseOperations(current.doc, ops);
        setSaving(true);
        setEditError(null);
        try {
          const r = await applyUseCaseOperations(pid, id, current.revision, ops, csrfToken);
          const next: Editor = {
            doc: r.document,
            labels: r.labels,
            revision: r.revision,
            baseVersionId: r.base_version_id,
          };
          editorRef.current = next;
          setEditor(next);
          const entry = inverse ? { forward: ops, inverse } : null;
          if (record === 'new') {
            setUndo((u) => (entry ? [...u, entry] : []));
            setRedo([]);
          } else if (record === 'undo') {
            setRedo((r2) => (entry ? [...r2, { forward: entry.inverse, inverse: entry.forward }] : []));
          } else {
            setUndo((u) => (entry ? [...u, { forward: entry.inverse, inverse: entry.forward }] : []));
          }
          const message = describeEffects(r.effects, r.document);
          if (message) setToast({ message, undo: record === 'new' && entry != null });
        } catch (e) {
          const body = errorBody(e);
          if (body?.code === 'stale_revision') {
            setConflict((body.current as UseCaseDraftSummary | null) ?? null);
          } else if (body?.code === 'element_has_dependents' && ops.length === 1 && ops[0]!.op === 'remove_element') {
            const op = ops[0] as DependentsPrompt['op'];
            setDependents({
              op,
              stepLabel: String(body.element_label ?? ''),
              stepText: locate(current.doc, op.id)?.element.text ?? '',
              branches: (body.branches as DependentBranch[]) ?? [],
            });
          } else {
            setEditError(e instanceof Error ? e.message : 'The change was not saved');
          }
        } finally {
          setSaving(false);
        }
      };
      const next = queue.current.then(run, run);
      queue.current = next;
      return next;
    },
    [pid, id, csrfToken],
  );

  // Undo / redo with the keyboard when the focus is not in a text field.
  useEffect(() => {
    if (!editing) return;
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName)) return;
      if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== 'z') return;
      e.preventDefault();
      if (e.shiftKey) {
        const last = redo.at(-1);
        if (last) {
          setRedo(redo.slice(0, -1));
          void send(last.forward, 'redo');
        }
      } else {
        const last = undo.at(-1);
        if (last) {
          setUndo(undo.slice(0, -1));
          void send(last.inverse, 'undo');
        }
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [editing, undo, redo, send]);

  function doUndo() {
    const last = undo.at(-1);
    if (!last) return;
    setUndo(undo.slice(0, -1));
    setToast(null);
    void send(last.inverse, 'undo');
  }

  function doRedo() {
    const last = redo.at(-1);
    if (!last) return;
    setRedo(redo.slice(0, -1));
    void send(last.forward, 'redo');
  }

  async function closeEditor() {
    await queue.current;
    setToast(null);
    setEditor(null);
    setSelectedBranch(null);
    await load();
  }

  async function onPublish() {
    const current = editorRef.current;
    if (!csrfToken || !current || current.revision == null) return;
    setBusy(true);
    setActionError(null);
    try {
      await queue.current;
      const published = await publishUseCaseDraft(
        pid,
        id,
        editorRef.current!.revision!,
        changeNote.trim() || null,
        csrfToken,
      );
      const removed = published.removed_links ?? [];
      if (removed.length > 0) {
        setToast({
          message: `Removed ${removed.length} link${removed.length === 1 ? '' : 's'} to deleted steps: ${removed
            .map((l) => l.target_reference_code)
            .join(', ')}.`,
          undo: false,
        });
      }
      setPublishOpen(false);
      setChangeNote('');
      setEditor(null);
      await load();
    } catch (e) {
      const body = errorBody(e);
      setActionError(
        body?.code === 'invalid_document' && Array.isArray(body.errors)
          ? `Cannot publish: ${(body.errors as { message: string }[]).map((x) => x.message).join('; ')}`
          : e instanceof Error
            ? e.message
            : 'Publish failed',
      );
    } finally {
      setBusy(false);
    }
  }

  async function onDiscard() {
    if (!csrfToken) return;
    if (!window.confirm('Discard the draft? Everyone’s unpublished changes to this use case are lost.')) return;
    setBusy(true);
    try {
      await queue.current;
      await discardUseCaseDraft(pid, id, csrfToken);
      setEditor(null);
      await load();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : 'Discard failed');
    } finally {
      setBusy(false);
    }
  }

  async function onDelete() {
    if (!csrfToken || !detail) return;
    if (!window.confirm(`Delete ${detail.reference_code} with all its versions? This cannot be undone.`)) return;
    try {
      await deleteUseCase(pid, id, csrfToken);
      navigate(`${basePath}/use-cases`);
    } catch (e) {
      setActionError(e instanceof Error ? e.message : 'Delete failed');
    }
  }

  async function onApproval(state: 'reviewed' | 'approved') {
    const version = detail?.current_version;
    if (!csrfToken || !version) return;
    setBusy(true);
    setActionError(null);
    try {
      await setUseCaseVersionApproval(pid, id, version.id, state, csrfToken);
      await load();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : 'Approval failed');
    } finally {
      setBusy(false);
    }
  }

  async function reloadLinks() {
    setLinks(await listUseCaseLinks(pid, id).catch(() => links));
  }

  async function onCreateLink(body: Parameters<typeof createUseCaseLink>[2]) {
    if (!csrfToken) return;
    setBusy(true);
    setLinkError(null);
    try {
      await createUseCaseLink(pid, id, body, csrfToken);
      setLinkDialog(null);
      await reloadLinks();
    } catch (e) {
      setLinkError(e instanceof Error ? e.message : 'Link failed');
    } finally {
      setBusy(false);
    }
  }

  async function onRemoveLink(link: UseCaseLink) {
    if (!csrfToken) return;
    if (!window.confirm(`Remove the link to ${link.target_reference_code}?`)) return;
    try {
      await deleteUseCaseLink(pid, id, link.id, csrfToken);
      await reloadLinks();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : 'Remove failed');
    }
  }

  async function onClearLink(link: UseCaseLink) {
    if (!csrfToken) return;
    try {
      await clearUseCaseLinkSuspect(pid, id, link.id, csrfToken);
      await reloadLinks();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : 'Clear failed');
    }
  }

  function openLink(link: UseCaseLink) {
    if (link.requirement_id != null) setPeek(link);
    else if (link.verification_id != null) navigate(`${basePath}/verifications/${link.verification_id}`);
  }

  const shown = useMemo(() => {
    if (editor) return { doc: editor.doc, labels: editor.labels };
    if (detail?.current_version) {
      return { doc: detail.current_version.document, labels: detail.current_version.labels };
    }
    return null;
  }, [editor, detail]);

  const linksByElement = useMemo(() => {
    const m = new Map<string, UseCaseLink[]>();
    for (const l of links) {
      if (!l.element_id) continue;
      m.set(l.element_id, [...(m.get(l.element_id) ?? []), l]);
    }
    return m;
  }, [links]);

  const linkable: LinkableElement[] = useMemo(() => {
    if (!shown) return [];
    const { doc: d, labels: lb } = shown;
    const els = [
      ...d.preconditions,
      ...d.main,
      ...d.branches.flatMap((b) => b.steps),
      ...d.postconditions,
    ];
    return els.map((e) => ({
      id: e.id,
      label: /^\d/.test(lb[e.id] ?? '') ? `Step ${lb[e.id]}` : (lb[e.id] ?? e.id),
      text: e.text,
    }));
  }, [shown]);

  const canLink = Boolean(perms?.edit_requirements) && !archived;

  const commentCounts = useMemo(() => {
    const m = new Map<string, number>();
    for (const c of comments) if (c.element_id) m.set(c.element_id, (m.get(c.element_id) ?? 0) + 1);
    return m;
  }, [comments]);

  function setReviewing(on: boolean) {
    const next = new URLSearchParams(params);
    if (on) next.set('view', 'review');
    else next.delete('view');
    setParams(next, { replace: true });
  }

  if (loadError) {
    return (
      <div role="alert" className="rounded-xl border border-red-500/40 bg-red-500/10 p-4 text-sm text-red-800 dark:text-red-100">
        {loadError}
        <div className="mt-3">
          <Link to={`${basePath}/use-cases`} className="font-semibold text-stitch-accent underline">
            Back to use cases
          </Link>
        </div>
      </div>
    );
  }
  if (!detail) {
    return <div className="p-8 text-center text-sm text-stitch-muted">Loading use case…</div>;
  }

  const version = detail.current_version;
  const latest = versions[0];
  const isHistorical = requestedVersion != null && latest != null && version?.id !== latest.id;
  const chip = approvalChip(version?.approval_state);
  const doc = shown?.doc;
  const draft = detail.draft;

  return (
    <div>
      <nav className="mb-2 flex items-center gap-1 text-[11px] font-bold uppercase tracking-[0.14em] text-stitch-muted">
        <Link to={`${basePath}/use-cases`} className="hover:text-stitch-accent">
          Use cases
        </Link>
        <span className="material-symbols-outlined text-sm">chevron_right</span>
        <span className="text-stitch-accent">{detail.reference_code}</span>
      </nav>

      <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="mb-1 flex flex-wrap items-center gap-2">
            <span className="font-mono text-xs font-semibold text-stitch-accent">{detail.reference_code}</span>
            <span className={`rounded-sm px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wider ${chip.className}`}>
              {chip.label}
            </span>
            {version ? (
              <span className="rounded-sm bg-stitch-elevated px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wider text-stitch-fg">
                v{version.version_number}
              </span>
            ) : null}
            {doc?.primary_actor ? (
              <span className="text-xs text-stitch-muted">
                Primary actor <strong className="text-stitch-fg">{doc.primary_actor}</strong>
              </span>
            ) : null}
          </div>
          <h1 className="font-headline text-2xl font-bold text-stitch-fg">{doc?.title ?? detail.title}</h1>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {!editing && canApprove && version && !isHistorical && version.approval_state === 'draft' ? (
            <button type="button" disabled={busy} onClick={() => void onApproval('reviewed')} className="rounded-md border border-stitch-border bg-stitch-surface px-3 py-1.5 text-xs font-semibold text-stitch-fg disabled:opacity-50">
              Mark as reviewed
            </button>
          ) : null}
          {!editing && canApprove && version && !isHistorical && version.approval_state === 'reviewed' ? (
            <button type="button" disabled={busy} onClick={() => void onApproval('approved')} className="rounded-md border border-stitch-border bg-stitch-surface px-3 py-1.5 text-xs font-semibold text-stitch-fg disabled:opacity-50">
              Approve
            </button>
          ) : null}
          {!editing && canEdit ? (
            <>
              <button type="button" onClick={() => void onDelete()} className="rounded-md border border-stitch-border bg-stitch-surface px-3 py-1.5 text-xs font-semibold text-stitch-danger">
                Delete
              </button>
              <button
                type="button"
                onClick={() => void openEditor()}
                className="inline-flex items-center gap-1.5 rounded-md bg-linear-to-br from-primary to-primary-container px-3 py-1.5 text-xs font-semibold text-white shadow-md"
              >
                <span className="material-symbols-outlined text-sm">edit</span>
                {draft ? 'Continue editing' : 'Edit'}
              </button>
            </>
          ) : null}
          {editing ? (
            <button type="button" onClick={() => void closeEditor()} className="rounded-md border border-stitch-border bg-stitch-surface px-3 py-1.5 text-xs font-semibold text-stitch-fg">
              Done
            </button>
          ) : null}
        </div>
      </div>

      {isHistorical ? (
        <div role="status" className="mb-4 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-amber-500/40 bg-amber-500/10 px-4 py-2 text-sm text-stitch-fg">
          <span>
            Viewing version {version?.version_number}, not the current version.
          </span>
          <Link to={`${basePath}/use-cases/${id}`} className="text-xs font-bold uppercase tracking-wider text-stitch-accent hover:underline">
            View current
          </Link>
        </div>
      ) : null}

      {!editing && draft && !isHistorical ? (
        <div role="status" className="mb-4 flex flex-wrap items-center gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 px-4 py-2 text-sm text-stitch-fg">
          <span className="material-symbols-outlined text-lg text-amber-700">edit_note</span>
          {version ? 'A draft of the next version is in progress' : 'Not published yet: this use case only has a draft'}
          {draft.updated_by_name ? ` · last change by ${draft.updated_by_name}` : ''} · {formatRelative(draft.updated_at)}
        </div>
      ) : null}

      {editing ? (
        <div className="sticky top-0 z-20 mb-3 flex flex-wrap items-center gap-3 rounded-lg border border-amber-500/40 bg-amber-50 px-4 py-2 dark:bg-amber-950/60">
          <span className="material-symbols-outlined text-lg text-amber-700">edit_note</span>
          <span className="text-sm text-stitch-fg">
            <strong>Draft</strong>
            {editor.baseVersionId != null && version ? ` · based on v${version.version_number}` : ''}
            {' · '}
            {saving ? 'Saving…' : editor.revision == null ? 'no changes yet' : 'all changes saved'}
          </span>
          <span className="ml-auto flex flex-wrap items-center gap-2">
            <button type="button" title="Undo (Ctrl+Z)" aria-label="Undo" disabled={undo.length === 0 || saving} onClick={doUndo} className="rounded-md border border-stitch-border bg-stitch-surface px-2 py-1 text-stitch-fg disabled:opacity-40">
              <span className="material-symbols-outlined text-base">undo</span>
            </button>
            <button type="button" title="Redo (Ctrl+Shift+Z)" aria-label="Redo" disabled={redo.length === 0 || saving} onClick={doRedo} className="rounded-md border border-stitch-border bg-stitch-surface px-2 py-1 text-stitch-fg disabled:opacity-40">
              <span className="material-symbols-outlined text-base">redo</span>
            </button>
            {version && editor.revision != null ? (
              <button type="button" disabled={busy} onClick={() => void onDiscard()} className="rounded-md border border-stitch-border bg-stitch-surface px-3 py-1.5 text-xs font-semibold text-stitch-fg disabled:opacity-50">
                Discard draft
              </button>
            ) : null}
            <button
              type="button"
              disabled={busy || saving || editor.revision == null}
              onClick={() => {
                setActionError(null);
                setPublishOpen(true);
              }}
              className="inline-flex items-center gap-1.5 rounded-md bg-linear-to-br from-primary to-primary-container px-3 py-1.5 text-xs font-semibold text-white shadow-md disabled:opacity-50"
            >
              <span className="material-symbols-outlined text-sm">publish</span>
              Publish version
            </button>
          </span>
        </div>
      ) : null}

      {conflict !== undefined ? (
        <div role="alert" className="mb-3 flex flex-wrap items-center gap-3 rounded-lg border border-red-500/40 bg-red-500/10 px-4 py-2 text-sm text-stitch-fg">
          <span>
            Someone else changed this draft
            {conflict?.updated_by_name ? ` (${conflict.updated_by_name}, ${formatRelative(conflict.updated_at)})` : ''}
            . Your last change was not saved.
          </span>
          <button type="button" onClick={() => void openEditor()} className="ml-auto text-xs font-bold uppercase tracking-wider text-stitch-accent hover:underline">
            Reload the draft
          </button>
        </div>
      ) : null}
      {editError ? (
        <div role="alert" className="mb-3 rounded-lg border border-red-500/40 bg-red-500/10 px-4 py-2 text-sm text-red-800 dark:text-red-200">
          {editError}
        </div>
      ) : null}
      {actionError ? (
        <div role="alert" className="mb-3 rounded-lg border border-red-500/40 bg-red-500/10 px-4 py-2 text-sm text-red-800 dark:text-red-200">
          {actionError}
        </div>
      ) : null}

      <div className="flex flex-col gap-5 lg:flex-row">
        <div className="min-w-0 flex-1">
        {!editing && (versions.length > 1 || (draft && versions.length > 0)) ? (
          <div role="tablist" aria-label="View" className="mb-3 flex gap-1">
            {(
              [
                [false, 'article', 'Document'],
                [true, 'compare', 'Review changes'],
              ] as const
            ).map(([on, icon, text]) => (
              <button
                key={text}
                type="button"
                role="tab"
                aria-selected={reviewing === on}
                onClick={() => setReviewing(on)}
                className={`inline-flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-xs font-semibold ${
                  reviewing === on
                    ? 'border-stitch-accent bg-stitch-accent/10 text-stitch-accent'
                    : 'border-stitch-border bg-stitch-surface text-stitch-fg'
                }`}
              >
                <span className="material-symbols-outlined text-sm">{icon}</span>
                {text}
              </button>
            ))}
          </div>
        ) : null}
        {reviewing && !editing ? (
          <UseCaseReviewView
            projectId={pid}
            useCaseId={id}
            versions={versions}
            hasDraft={draft != null}
            comments={comments}
            onCommentElement={setCommentFocus}
          />
        ) : doc && shown ? (
          <UseCaseDocumentView
            doc={doc}
            labels={shown.labels}
            editing={editing}
            apply={(ops) => send(ops)}
            selectedBranch={selectedBranch}
            onSelectBranch={setSelectedBranch}
            linksByElement={linksByElement}
            onOpenLink={openLink}
            onLinkElement={
              canLink
                ? (elementId) => {
                    setLinkError(null);
                    setLinkDialog({ elementId });
                  }
                : undefined
            }
            commentCounts={commentCounts}
            onCommentElement={setCommentFocus}
          />
        ) : (
          <div className="flex-1 rounded-lg border border-dashed border-stitch-border p-8 text-center text-sm text-stitch-muted">
            Nothing published yet.
            {canEdit ? ' Open the editor to write the scenario.' : ''}
          </div>
        )}
        <UseCaseDiscussion
          projectId={pid}
          useCaseId={id}
          comments={comments}
          members={members}
          elements={linkable}
          versionId={version?.id ?? null}
          canComment={canLink}
          locked={false}
          csrfToken={csrfToken}
          focusElement={commentFocus}
          onFocusElement={setCommentFocus}
          onPosted={(c) => setComments((all) => [...all, c])}
        />
        </div>

        <aside className="w-full shrink-0 space-y-3 lg:w-72">
          {selectedBranch && doc ? (
            <div className="space-y-2 rounded-lg border border-stitch-border bg-stitch-surface px-4 py-3 text-xs shadow-sm">
              {(() => {
                const b = doc.branches.find((x) => x.id === selectedBranch);
                if (!b) return null;
                const labels = shown!.labels;
                return (
                  <>
                    <p className="text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted">
                      Selected: {labels[b.id]}
                    </p>
                    <p className="text-stitch-fg">
                      Branches from <strong>step {labels[b.from_step]}</strong>, highlighted in the main scenario.
                    </p>
                    <button
                      type="button"
                      onClick={() =>
                        document
                          .querySelector(`[data-element-id="${b.from_step}"]`)
                          ?.scrollIntoView({ behavior: 'smooth', block: 'center' })
                      }
                      className="flex items-center gap-1.5 font-semibold text-stitch-accent"
                    >
                      <span className="material-symbols-outlined text-sm">north_west</span>
                      Jump to origin
                    </button>
                    <p className="border-t border-stitch-border pt-2 text-stitch-muted">
                      {b.end.type === 'rejoin'
                        ? `Rejoins at step ${labels[b.end.step]}.`
                        : `Ends the use case${b.end.outcome === 'failure' ? ' without success' : ''}.`}
                    </p>
                  </>
                );
              })()}
            </div>
          ) : null}

          <RelatedArtifactsPanel
            links={links}
            labels={shown?.labels ?? {}}
            canEdit={canLink}
            onAdd={() => {
              setLinkError(null);
              setLinkDialog({ elementId: null });
            }}
            onOpen={openLink}
            onRemove={(l) => void onRemoveLink(l)}
            onClear={(l) => void onClearLink(l)}
          />

          <div className="rounded-lg border border-stitch-border bg-stitch-surface px-4 py-3 text-xs shadow-sm">
            <p className="mb-2 text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted">Details</p>
            <dl className="space-y-1.5">
              <div className="flex justify-between gap-2">
                <dt className="text-stitch-muted">Primary actor</dt>
                <dd className="text-right text-stitch-fg">{doc?.primary_actor ?? '—'}</dd>
              </div>
              <div className="flex justify-between gap-2">
                <dt className="text-stitch-muted">Secondary actors</dt>
                <dd className="text-right text-stitch-fg">{doc?.secondary_actors.join(', ') || '—'}</dd>
              </div>
              {version ? (
                <div className="flex justify-between gap-2">
                  <dt className="text-stitch-muted">Published</dt>
                  <dd className="text-right text-stitch-fg">
                    {version.created_by_name ?? '—'} · {formatRelative(version.created_at)}
                  </dd>
                </div>
              ) : null}
              {version?.change_note ? (
                <div>
                  <dt className="text-stitch-muted">Change note</dt>
                  <dd className="text-stitch-fg">{version.change_note}</dd>
                </div>
              ) : null}
            </dl>
          </div>

          {versions.length > 0 ? (
            <div className="rounded-lg border border-stitch-border bg-stitch-surface px-4 py-3 text-xs shadow-sm">
              <p className="mb-2 text-[10px] font-bold uppercase tracking-[0.12em] text-stitch-muted">Versions</p>
              <ul className="space-y-1">
                {versions.map((v) => {
                  const c = approvalChip(v.approval_state);
                  const active = version?.id === v.id;
                  return (
                    <li key={v.id} className="flex items-center gap-2">
                      <Link
                        to={v.id === latest?.id ? `${basePath}/use-cases/${id}` : `${basePath}/use-cases/${id}?version=${v.id}`}
                        className={`font-mono font-semibold ${active ? 'text-stitch-fg' : 'text-stitch-accent hover:underline'}`}
                      >
                        v{v.version_number}
                      </Link>
                      <span className={`rounded-sm px-1 py-px text-[9px] font-bold uppercase tracking-wider ${c.className}`}>{c.label}</span>
                      <span className="ml-auto text-stitch-muted">{formatRelative(v.created_at)}</span>
                    </li>
                  );
                })}
              </ul>
            </div>
          ) : null}

          {editing ? (
            <div className="space-y-1.5 rounded-lg border border-stitch-border bg-stitch-surface px-4 py-3 text-xs text-stitch-muted shadow-sm">
              <p className="text-[10px] font-bold uppercase tracking-[0.12em]">Keyboard</p>
              <p>
                <span className="font-mono text-stitch-fg">Enter</span> new step below ·{' '}
                <span className="font-mono text-stitch-fg">Shift+Enter</span> new line
              </p>
              <p>
                <span className="font-mono text-stitch-fg">Alt ↑ / ↓</span> move the step ·{' '}
                <span className="font-mono text-stitch-fg">Esc</span> cancel the edit
              </p>
              <p>
                <span className="font-mono text-stitch-fg">Ctrl+Z</span> undo outside a text field
              </p>
              <p className="border-t border-stitch-border pt-1.5">
                Every change is saved to the shared draft at once. Publish to create a new version for review.
              </p>
            </div>
          ) : null}
        </aside>
      </div>

      {toast ? (
        <div role="status" className="fixed bottom-6 right-6 z-70 flex max-w-md items-center gap-3 rounded-lg bg-slate-900 px-4 py-2.5 text-sm text-white shadow-stitch ring-1 ring-white/10">
          <span className="material-symbols-outlined text-lg text-emerald-300">check_circle</span>
          <span>{toast.message}</span>
          {toast.undo ? (
            <button type="button" onClick={doUndo} className="ml-auto font-semibold text-sky-300">
              Undo
            </button>
          ) : null}
        </div>
      ) : null}

      <LinkDialog
        open={linkDialog != null}
        projectId={pid}
        elements={linkable}
        elementId={linkDialog?.elementId ?? null}
        busy={busy}
        error={linkError}
        onClose={() => setLinkDialog(null)}
        onCreate={(body) => void onCreateLink(body)}
      />

      <RequirementPeek projectId={pid} basePath={basePath} link={peek} onClose={() => setPeek(null)} />

      <DeleteStepDialog
        open={dependents != null}
        stepLabel={dependents?.stepLabel ?? ''}
        stepText={dependents?.stepText ?? ''}
        branches={dependents?.branches ?? []}
        targets={
          editor && dependents
            ? editor.doc.main
                .filter((s) => s.id !== dependents.op.id)
                .map((s) => ({ id: s.id, label: editor.labels[s.id] ?? '', text: s.text }))
            : []
        }
        busy={saving}
        onCancel={() => setDependents(null)}
        onConfirm={(choice) => {
          const op = dependents!.op;
          setDependents(null);
          void send([{ ...op, dependents: choice }]);
        }}
      />

      <Dialog
        open={publishOpen}
        onClose={() => setPublishOpen(false)}
        title={`Publish version ${(latest?.version_number ?? 0) + 1}`}
        subtitle="The draft becomes an immutable version in the Draft approval state, ready for review."
      >
        <label className="block">
          <span className="mb-1 block text-[10px] font-bold uppercase text-stitch-muted">Change note (optional)</span>
          <textarea
            value={changeNote}
            rows={3}
            onChange={(e) => setChangeNote(e.target.value)}
            placeholder="What changed, for reviewers"
            className="w-full rounded-md border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg"
          />
        </label>
        {actionError ? (
          <p role="alert" className="mt-3 text-sm text-red-700 dark:text-red-300">
            {actionError}
          </p>
        ) : null}
        <div className="mt-4 flex justify-end gap-2">
          <button type="button" onClick={() => setPublishOpen(false)} className="rounded-md border border-stitch-border px-3 py-1.5 text-xs font-semibold text-stitch-fg">
            Cancel
          </button>
          <button type="button" disabled={busy} onClick={() => void onPublish()} className="rounded-md bg-stitch-accent px-3 py-1.5 text-xs font-bold text-stitch-on-accent disabled:opacity-50">
            {busy ? 'Publishing…' : 'Publish'}
          </button>
        </div>
      </Dialog>
    </div>
  );
}

import { type FormEvent, useEffect, useMemo, useState } from 'react';
import { getGroup, listCreatableGroups, listProjectsOptional, updateProject } from '@/api/client';
import type { GroupResponse, Project, ProjectMember, ProjectUpdateBody } from '@/api/types';

type Status = NonNullable<ProjectUpdateBody['status']>;

const STATUS_OPTIONS: { value: Status; label: string }[] = [
  { value: 'Active', label: 'Active' },
  { value: 'OnHold', label: 'On hold' },
  { value: 'Completed', label: 'Completed' },
  { value: 'Cancelled', label: 'Cancelled' },
];

/** Same limits as the backend validation (`validate_project`). */
const NAME_MIN = 2;
const NAME_MAX = 100;
const DESCRIPTION_MAX = 1000;

type Draft = {
  name: string;
  description: string;
  status: Status;
  ownerId: number | null;
  groupId: number | null;
};

function toDraft(p: Project): Draft {
  return {
    name: p.name,
    description: p.description ?? '',
    status: (p.status as Status) ?? 'Active',
    ownerId: p.owner_id,
    groupId: p.group_id,
  };
}

/** Only the fields that changed, in the PATCH body format. */
export function changedFields(original: Draft, draft: Draft): ProjectUpdateBody {
  const body: ProjectUpdateBody = {};
  if (draft.name.trim() !== original.name) body.name = draft.name.trim();
  if (draft.description.trim() !== original.description.trim()) {
    body.description = draft.description.trim() === '' ? null : draft.description.trim();
  }
  if (draft.status !== original.status) body.status = draft.status;
  if (draft.ownerId !== original.ownerId && draft.ownerId != null) body.owner_id = draft.ownerId;
  if (draft.groupId !== original.groupId) body.group_id = draft.groupId;
  return body;
}

type Props = {
  projectId: number;
  members: ProjectMember[];
  userLabel: (userId: number) => string;
  /** `manage_project_configuration` (project Admin or instance admin). */
  canEdit: boolean;
  csrfToken: string;
  /** Called after a successful save, e.g. to refresh the dashboard (header, sidebar, project list). */
  onSaved: () => Promise<void> | void;
};

const labelCls = 'block text-[10px] uppercase tracking-widest text-stitch-muted font-bold mb-1';
const inputCls =
  'w-full rounded-md border border-stitch-border bg-stitch-surface px-3 py-2 text-sm text-stitch-fg disabled:opacity-70 disabled:bg-stitch-elevated/50';

export default function ProjectGeneralSettings({
  projectId,
  members,
  userLabel,
  canEdit,
  csrfToken,
  onSaved,
}: Props) {
  const [project, setProject] = useState<Project | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [groups, setGroups] = useState<GroupResponse[]>([]);
  const [loadErr, setLoadErr] = useState<string | null>(null);
  const [saveErr, setSaveErr] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const list = await listProjectsOptional();
      const p = list?.find((x) => x.id === projectId) ?? null;
      if (cancelled) return;
      if (!p) {
        setLoadErr('Could not load the project properties.');
        return;
      }
      setProject(p);
      setDraft(toDraft(p));
      // Groups the user may move the project into, plus its current group.
      const creatable = await listCreatableGroups().catch(() => [] as GroupResponse[]);
      let all = creatable;
      if (p.group_id != null && !creatable.some((g) => g.id === p.group_id)) {
        const current = await getGroup(p.group_id).catch(() => null);
        if (current) all = [current, ...creatable];
      }
      if (!cancelled) setGroups(all);
    })();
    return () => {
      cancelled = true;
    };
  }, [projectId]);

  const original = useMemo(() => (project ? toDraft(project) : null), [project]);
  const changes = useMemo(
    () => (original && draft ? changedFields(original, draft) : {}),
    [original, draft],
  );
  const dirty = Object.keys(changes).length > 0;

  const nameLength = draft?.name.trim().length ?? 0;
  const validation =
    nameLength < NAME_MIN
      ? `Name must be at least ${NAME_MIN} characters.`
      : nameLength > NAME_MAX
        ? `Name must be at most ${NAME_MAX} characters.`
        : (draft?.description.trim().length ?? 0) > DESCRIPTION_MAX
          ? `Description must be at most ${DESCRIPTION_MAX} characters.`
          : null;

  const set = (patch: Partial<Draft>) => {
    setSaved(false);
    setSaveErr(null);
    setDraft((d) => (d ? { ...d, ...patch } : d));
  };

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    if (!dirty || validation || !canEdit) return;
    setBusy(true);
    setSaveErr(null);
    try {
      const updated = await updateProject(projectId, changes, csrfToken);
      setProject(updated);
      setDraft(toDraft(updated));
      setSaved(true);
      await onSaved();
    } catch (err) {
      setSaveErr(err instanceof Error ? err.message : 'Could not save the project');
    } finally {
      setBusy(false);
    }
  }

  const groupLabel = (id: number) => {
    const g = groups.find((x) => x.id === id);
    return g ? `${g.name} (${g.slug})` : `Group #${id}`;
  };

  return (
    <section id="project-general" className="mb-10 scroll-mt-8">
      <h3 className="text-sm font-bold text-stitch-fg uppercase tracking-widest mb-4">General</h3>
      {loadErr ? <p className="text-sm text-red-300">{loadErr}</p> : null}
      {!draft || !project ? (
        loadErr ? null : <p className="text-sm text-stitch-muted">Loading…</p>
      ) : (
        <form
          onSubmit={(e) => void onSubmit(e)}
          aria-label="Project properties"
          className="rounded-xl border border-stitch-border bg-stitch-surface p-4 grid gap-4 md:grid-cols-2 max-w-4xl"
        >
          {!canEdit ? (
            <p className="md:col-span-2 text-xs text-stitch-muted">
              Only project Admins and instance administrators can change these properties.
            </p>
          ) : null}
          <label className="md:col-span-2">
            <span className={labelCls}>Name</span>
            <input
              className={inputCls}
              value={draft.name}
              maxLength={NAME_MAX}
              disabled={!canEdit}
              onChange={(e) => set({ name: e.target.value })}
            />
          </label>
          <label className="md:col-span-2">
            <span className={labelCls}>Description</span>
            <textarea
              className={`${inputCls} min-h-24`}
              value={draft.description}
              maxLength={DESCRIPTION_MAX}
              disabled={!canEdit}
              onChange={(e) => set({ description: e.target.value })}
            />
          </label>
          <label>
            <span className={labelCls}>Status</span>
            <select
              className={inputCls}
              value={draft.status}
              disabled={!canEdit}
              onChange={(e) => set({ status: e.target.value as Status })}
            >
              {STATUS_OPTIONS.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span className={labelCls}>Owner</span>
            <select
              className={inputCls}
              value={draft.ownerId ?? ''}
              disabled={!canEdit}
              onChange={(e) => set({ ownerId: e.target.value ? Number(e.target.value) : null })}
            >
              {draft.ownerId == null ? <option value="">—</option> : null}
              {draft.ownerId != null && !members.some((m) => m.user_id === draft.ownerId) ? (
                <option value={draft.ownerId}>{userLabel(draft.ownerId)} (not a member)</option>
              ) : null}
              {members.map((m) => (
                <option key={m.user_id} value={m.user_id}>
                  {userLabel(m.user_id)}
                </option>
              ))}
            </select>
          </label>
          <label>
            <span className={labelCls}>Group</span>
            <select
              className={inputCls}
              value={draft.groupId ?? ''}
              disabled={!canEdit}
              onChange={(e) => set({ groupId: e.target.value ? Number(e.target.value) : null })}
            >
              <option value="">Personal (no group)</option>
              {project.group_id != null && !groups.some((g) => g.id === project.group_id) ? (
                <option value={project.group_id}>{groupLabel(project.group_id)}</option>
              ) : null}
              {groups.map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name} ({g.slug})
                </option>
              ))}
            </select>
          </label>
          <div>
            <span className={labelCls}>URL</span>
            <p className="rounded-md border border-stitch-border bg-stitch-elevated/50 px-3 py-2 font-mono text-sm text-stitch-fg">
              /{project.slug}
            </p>
            <p className="mt-1 text-[11px] text-stitch-muted">The URL stays the same when the name changes.</p>
          </div>
          {canEdit ? (
            <div className="md:col-span-2 flex flex-wrap items-center gap-3">
              <button
                type="submit"
                disabled={!dirty || Boolean(validation) || busy}
                className="rounded-md bg-stitch-accent px-4 py-2 text-xs font-bold uppercase tracking-wider text-stitch-on-accent disabled:opacity-50"
              >
                {busy ? 'Saving…' : 'Save changes'}
              </button>
              {dirty ? (
                <button
                  type="button"
                  onClick={() => setDraft(toDraft(project))}
                  className="text-xs font-bold uppercase tracking-wider text-stitch-muted hover:text-stitch-accent"
                >
                  Discard
                </button>
              ) : null}
              {validation && dirty ? <span className="text-xs text-red-400">{validation}</span> : null}
              {saveErr ? (
                <span role="alert" className="text-xs text-red-400">
                  {saveErr}
                </span>
              ) : null}
              {saved ? (
                <span role="status" className="text-xs text-emerald-600 dark:text-emerald-400">
                  Saved.
                </span>
              ) : null}
            </div>
          ) : null}
        </form>
      )}
    </section>
  );
}

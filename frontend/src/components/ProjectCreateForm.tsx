import { useState, type FormEvent } from 'react';
import { createProject } from '@/api/client';
import type { GroupResponse } from '@/api/types';

type ProjectCreateFormProps = {
  csrfToken: string;
  personalNamespace: string;
  groups?: GroupResponse[];
  fixedGroupId?: number;
  onCreated: (project: Awaited<ReturnType<typeof createProject>>) => void | Promise<void>;
  onCancel?: () => void;
  compact?: boolean;
};

export default function ProjectCreateForm({
  csrfToken,
  personalNamespace,
  groups = [],
  fixedGroupId,
  onCreated,
  onCancel,
  compact = false,
}: ProjectCreateFormProps) {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [namespace, setNamespace] = useState(
    fixedGroupId == null ? 'personal' : `group:${fixedGroupId}`,
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const inputClass =
    'w-full text-sm font-medium bg-stitch-elevated border border-stitch-border rounded-md px-3 py-2 text-stitch-fg focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/40 outline-none transition-colors';

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!csrfToken || !name.trim()) return;
    const groupId = fixedGroupId ?? (
      namespace === 'personal' ? null : Number(namespace.slice('group:'.length))
    );
    setBusy(true);
    setError(null);
    try {
      const project = await createProject(
        {
          name: name.trim(),
          description: description.trim() || null,
          group_id: groupId,
        },
        csrfToken,
      );
      await onCreated(project);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to create project');
    } finally {
      setBusy(false);
    }
  }

  return (
    <form onSubmit={(event) => void handleSubmit(event)} className={compact ? 'space-y-3' : 'space-y-5'}>
      {fixedGroupId == null ? (
        <div>
          <label htmlFor="project-namespace" className="block text-xs font-bold text-stitch-muted uppercase tracking-wider mb-1.5">
            Namespace
          </label>
          <select
            id="project-namespace"
            value={namespace}
            onChange={(event) => setNamespace(event.target.value)}
            className={inputClass}
          >
            <option value="personal">{personalNamespace} — Personal</option>
            {groups.map((group) => (
              <option key={group.id} value={`group:${group.id}`}>
                {group.slug} — Group
              </option>
            ))}
          </select>
        </div>
      ) : null}

      <div>
        <label htmlFor="project-name" className="block text-xs font-bold text-stitch-muted uppercase tracking-wider mb-1.5">
          Project name
        </label>
        <input
          id="project-name"
          value={name}
          onChange={(event) => setName(event.target.value)}
          placeholder="e.g. Flight Control System"
          className={inputClass}
          autoFocus
        />
      </div>
      <div>
        <label htmlFor="project-description" className="block text-xs font-bold text-stitch-muted uppercase tracking-wider mb-1.5">
          Description (optional)
        </label>
        <input
          id="project-description"
          value={description}
          onChange={(event) => setDescription(event.target.value)}
          placeholder="Brief description"
          className={inputClass}
        />
      </div>

      {error ? <div role="alert" className="text-sm text-red-400">{error}</div> : null}

      <div className="flex items-center gap-2">
        <button
          type="submit"
          disabled={busy || !name.trim()}
          className="bg-gradient-to-br from-[#000666] to-[#1a237e] text-white px-4 py-2 rounded-md text-xs font-bold uppercase tracking-widest shadow-lg disabled:opacity-50 hover:opacity-95 transition-opacity"
        >
          {busy ? 'Creating…' : 'Create project'}
        </button>
        {onCancel ? (
          <button
            type="button"
            onClick={onCancel}
            className="text-xs font-bold uppercase tracking-wider text-stitch-muted hover:text-stitch-fg transition-colors px-2 py-2"
          >
            Cancel
          </button>
        ) : null}
      </div>
    </form>
  );
}

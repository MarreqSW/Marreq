import { type FormEvent, useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { deleteProject, listProjectsOptional } from '@/api/client';
import type { Project } from '@/api/types';
import Dialog from '@/components/Dialog';
import { useDashboard } from '@/context/DashboardContext';
import { parseUser } from '@/utils/parseUser';

type Props = {
  projectId: number;
  /** Project root path, for the link to Reports & exports. */
  basePath: string;
  userLabel: (userId: number) => string;
};

/**
 * Project settings › General: the "Danger zone" that permanently deletes the
 * project (issue #349). Only the project owner or an instance administrator
 * may delete; they must type the project slug to confirm.
 */
export default function DeleteProjectSection({ projectId, basePath, userLabel }: Props) {
  const { dashboard, csrfToken, refresh } = useDashboard();
  const navigate = useNavigate();
  const me = useMemo(() => parseUser(dashboard?.user), [dashboard?.user]);
  const [project, setProject] = useState<Project | null>(null);
  const [open, setOpen] = useState(false);
  const [typed, setTyped] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const list = await listProjectsOptional();
      if (!cancelled) setProject(list?.find((p) => p.id === projectId) ?? null);
    })();
    return () => {
      cancelled = true;
    };
  }, [projectId]);

  if (!project || !me) return null;

  const canDelete = me.is_admin || (project.owner_id != null && project.owner_id === me.id);
  const confirmed = typed === project.slug;

  const close = () => {
    if (busy) return;
    setOpen(false);
    setTyped('');
    setError(null);
  };

  async function onDelete(e: FormEvent) {
    e.preventDefault();
    if (!project || !confirmed || busy) return;
    setBusy(true);
    setError(null);
    try {
      await deleteProject(project.id, typed, csrfToken ?? '');
      await refresh();
      navigate('/', { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not delete the project.');
      setBusy(false);
    }
  }

  if (!canDelete) {
    const owner = project.owner_id != null ? userLabel(project.owner_id) : null;
    return (
      <p className="mb-10 text-xs text-stitch-muted">
        Only the project owner{owner ? ` (${owner})` : ''} or an instance administrator can delete
        this project.
      </p>
    );
  }

  return (
    <section className="mb-10 rounded-xl border border-red-600/40 p-5" aria-labelledby="danger-zone">
      <h3
        id="danger-zone"
        className="text-sm font-bold text-red-700 dark:text-red-300 uppercase tracking-widest mb-2"
      >
        Danger zone
      </h3>
      <div className="flex flex-wrap items-center justify-between gap-4">
        <p className="text-sm text-stitch-muted max-w-xl">
          Permanently delete <strong className="text-stitch-fg">{project.name}</strong> and all of
          its data. This cannot be undone.
        </p>
        <button
          type="button"
          onClick={() => setOpen(true)}
          className="rounded-md border border-red-600/60 px-4 py-2 text-sm font-semibold text-red-700 dark:text-red-300 hover:bg-red-600 hover:text-white transition-colors"
        >
          Delete project…
        </button>
      </div>

      <Dialog open={open} onClose={close} title="Delete project" subtitle={project.name}>
        <form onSubmit={onDelete} className="space-y-4 text-sm text-stitch-fg">
          <p>This permanently removes the project and everything in it:</p>
          <ul className="list-disc pl-5 text-stitch-muted space-y-0.5">
            <li>requirements, their versions, links and comments</li>
            <li>verifications and the traceability matrix</li>
            <li>baselines and saved views</li>
            <li>attachments and their files</li>
            <li>members, reviewers, catalog and custom fields</li>
          </ul>
          <p>
            <strong>This cannot be undone.</strong> To keep a copy, export a bundle or ReqIFZ from{' '}
            <Link to={`${basePath}/reports`} className="text-stitch-accent underline">
              Reports &amp; exports
            </Link>{' '}
            first. The audit log keeps a record of the deletion.
          </p>
          <label className="block">
            <span className="block mb-1">
              Type <code className="font-mono font-bold">{project.slug}</code> to confirm
            </span>
            <input
              value={typed}
              onChange={(e) => setTyped(e.target.value)}
              autoComplete="off"
              spellCheck={false}
              aria-label="Project slug"
              className="w-full rounded-md border border-stitch-border bg-stitch-surface px-3 py-2 font-mono"
            />
          </label>
          {error ? (
            <p role="alert" className="text-red-700 dark:text-red-300">
              {error}
            </p>
          ) : null}
          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={close}
              disabled={busy}
              className="rounded-md border border-stitch-border px-4 py-2"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!confirmed || busy}
              className="rounded-md bg-red-600 px-4 py-2 font-semibold text-white disabled:opacity-50"
            >
              {busy ? 'Deleting…' : 'Delete project'}
            </button>
          </div>
        </form>
      </Dialog>
    </section>
  );
}

import { FormEvent, useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { importProjectBundle, listCreatableGroups } from '@/api/client';
import type { GroupResponse } from '@/api/types';
import { useDashboard } from '@/context/DashboardContext';
import { parseUser } from '@/utils/parseUser';

export default function ProjectBundleImportPage() {
  const navigate = useNavigate();
  const { csrfToken, dashboard, refresh } = useDashboard();
  const user = useMemo(() => parseUser(dashboard?.user), [dashboard?.user]);
  const [groups, setGroups] = useState<GroupResponse[]>([]);
  const [file, setFile] = useState<File | null>(null);
  const [namespace, setNamespace] = useState('personal');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [warnings, setWarnings] = useState<string[]>([]);
  /** Set after an import with warnings: the page stays so they can be read. */
  const [importedPath, setImportedPath] = useState<string | null>(null);

  useEffect(() => {
    listCreatableGroups()
      .then(setGroups)
      .catch(() => setGroups([]));
  }, []);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    if (!file || !csrfToken) return;
    setBusy(true);
    setError(null);
    setWarnings([]);
    setImportedPath(null);
    try {
      const groupId =
        namespace.startsWith('group:') ? Number(namespace.slice('group:'.length)) : null;
      const result = await importProjectBundle(file, csrfToken, groupId);
      await refresh();
      const notes = [...(result.errors ?? []), ...(result.warnings ?? [])];
      if (notes.length === 0) {
        navigate(`${result.project_base_path}/dashboard`, { replace: true });
        return;
      }
      // Stay on the page so skipped files and other warnings can be read (issue #341).
      setWarnings(notes);
      setImportedPath(result.project_base_path);
    } catch (ex) {
      setError(ex instanceof Error ? ex.message : 'Import failed');
    } finally {
      setBusy(false);
    }
  }

  const inputClass =
    'w-full text-sm font-medium bg-stitch-elevated border border-stitch-border rounded-md px-3 py-2 text-stitch-fg focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/40 outline-hidden transition-colors';

  return (
    <div className="min-h-screen bg-stitch-canvas text-stitch-fg">
      <div className="max-w-2xl mx-auto px-6 py-10">
        <nav className="flex items-center gap-2 text-xs text-stitch-muted mb-6">
          <Link to="/" className="hover:text-stitch-accent">
            Dashboard
          </Link>
          <span aria-hidden="true">/</span>
          <Link to="/projects/new" className="hover:text-stitch-accent">
            New project
          </Link>
          <span aria-hidden="true">/</span>
          <span>Import bundle</span>
        </nav>
        <div className="rounded-xl border border-stitch-border bg-stitch-surface p-6 shadow-stitch">
          <h1 className="text-2xl font-bold font-headline tracking-tight">Import project bundle</h1>
          <p className="text-sm text-stitch-muted mt-1 mb-6">
            Creates a <strong>new</strong> project from a bundle exported from Reports: the{' '}
            <code>.json</code> bundle, or the <code>.zip</code> bundle with attachment files. It
            does not merge into an existing project.
          </p>
          <p className="text-sm text-stitch-muted -mt-4 mb-6">
            Files are stored within the new project&apos;s storage quota. A file that is too large,
            of a type that is not allowed, or over the quota is skipped and listed in the warnings.
          </p>
          <form onSubmit={onSubmit} className="space-y-4">
            {groups.length > 0 ? (
              <div>
                <label htmlFor="bundle-namespace" className="block text-xs font-bold text-stitch-muted uppercase tracking-wider mb-1.5">
                  Namespace
                </label>
                <select
                  id="bundle-namespace"
                  value={namespace}
                  onChange={(event) => setNamespace(event.target.value)}
                  className={inputClass}
                  disabled={busy}
                >
                  <option value="personal">{user?.username ?? 'Personal'} — Personal</option>
                  {groups.map((group) => (
                    <option key={group.id} value={`group:${group.id}`}>
                      {group.slug} — Group
                    </option>
                  ))}
                </select>
              </div>
            ) : null}
            <div>
              <label htmlFor="bundle-file" className="block text-xs font-bold text-stitch-muted uppercase tracking-wider mb-1.5">
                Bundle file
              </label>
              <input
                id="bundle-file"
                type="file"
                accept=".json,.zip,application/json,application/zip"
                onChange={(event) => {
                  setFile(event.target.files?.[0] ?? null);
                  setImportedPath(null);
                  setWarnings([]);
                }}
                disabled={busy}
                className={inputClass}
              />
            </div>
            {error ? (
              <div role="alert" className="text-sm text-red-400">
                {error}
              </div>
            ) : null}
            {importedPath ? (
              <div
                role="status"
                className="rounded-md border border-amber-600/35 bg-amber-500/10 px-3 py-2 text-sm text-amber-900 dark:text-amber-100 space-y-2"
              >
                <p>
                  The project was imported with {warnings.length}{' '}
                  {warnings.length === 1 ? 'warning' : 'warnings'}:
                </p>
                <ul className="text-xs list-disc pl-5 space-y-1">
                  {warnings.map((w, i) => (
                    <li key={`${i}-${w}`}>{w}</li>
                  ))}
                </ul>
                <button
                  type="button"
                  onClick={() => navigate(`${importedPath}/dashboard`, { replace: true })}
                  className="bg-linear-to-br from-primary to-primary-container text-white px-4 py-2 rounded-md text-xs font-bold uppercase tracking-widest shadow-lg"
                >
                  Open project
                </button>
              </div>
            ) : null}
            <div className="flex items-center gap-2">
              <button
                type="submit"
                disabled={busy || !file || !csrfToken || importedPath != null}
                className="bg-linear-to-br from-primary to-primary-container text-white px-4 py-2 rounded-md text-xs font-bold uppercase tracking-widest shadow-lg disabled:opacity-50 hover:opacity-95 transition-opacity"
              >
                {busy ? 'Importing…' : 'Import bundle'}
              </button>
              <button
                type="button"
                onClick={() => navigate('/projects/new')}
                className="text-xs font-bold uppercase tracking-wider text-stitch-muted hover:text-stitch-fg transition-colors px-2 py-2"
              >
                Cancel
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}

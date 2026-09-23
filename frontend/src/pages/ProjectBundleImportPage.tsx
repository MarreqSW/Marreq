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
    try {
      const groupId =
        namespace.startsWith('group:') ? Number(namespace.slice('group:'.length)) : null;
      const result = await importProjectBundle(file, csrfToken, groupId);
      setWarnings(result.warnings ?? []);
      await refresh();
      navigate(`${result.project_base_path}/dashboard`, { replace: true });
    } catch (ex) {
      setError(ex instanceof Error ? ex.message : 'Import failed');
    } finally {
      setBusy(false);
    }
  }

  const inputClass =
    'w-full text-sm font-medium bg-stitch-elevated border border-stitch-border rounded-md px-3 py-2 text-stitch-fg focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/40 outline-none transition-colors';

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
            Creates a <strong>new</strong> project from a JSON snapshot exported from Reports. It
            does not merge into an existing project.
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
                accept=".json,application/json"
                onChange={(event) => setFile(event.target.files?.[0] ?? null)}
                disabled={busy}
                className={inputClass}
              />
            </div>
            {error ? (
              <div role="alert" className="text-sm text-red-400">
                {error}
              </div>
            ) : null}
            {warnings.length ? (
              <ul className="text-xs text-amber-300 list-disc pl-5 space-y-1">
                {warnings.map((w) => (
                  <li key={w}>{w}</li>
                ))}
              </ul>
            ) : null}
            <div className="flex items-center gap-2">
              <button
                type="submit"
                disabled={busy || !file || !csrfToken}
                className="bg-gradient-to-br from-[#000666] to-[#1a237e] text-white px-4 py-2 rounded-md text-xs font-bold uppercase tracking-widest shadow-lg disabled:opacity-50 hover:opacity-95 transition-opacity"
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

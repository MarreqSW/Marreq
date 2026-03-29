import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { listGroups } from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type { GroupResponse } from '@/api/types';

export default function GroupsListPage() {
  const { dashboard } = useDashboard();
  const [groups, setGroups] = useState<GroupResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState('');

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setGroups(await listGroups());
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load groups');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const filtered = search.trim()
    ? groups.filter(
        (g) =>
          g.name.toLowerCase().includes(search.toLowerCase()) ||
          g.slug.toLowerCase().includes(search.toLowerCase()),
      )
    : groups;

  return (
    <div className="min-h-screen bg-stitch-canvas text-stitch-fg">
      <div className="max-w-5xl mx-auto px-6 py-10">
        <div className="flex flex-wrap items-center justify-between gap-4 mb-8">
          <div>
            <h1 className="text-2xl font-bold font-headline tracking-tight">Groups</h1>
            <p className="text-sm text-stitch-muted mt-1">
              Organize projects and team members into groups.
            </p>
          </div>
          <Link
            to="/groups/new"
            className="inline-flex items-center gap-2 px-4 py-2 rounded-md bg-gradient-to-br from-[#000666] to-[#1a237e] text-white text-sm font-semibold shadow-lg hover:opacity-95 transition-opacity"
          >
            <span className="material-symbols-outlined text-sm">add</span>
            New group
          </Link>
        </div>

        <div className="relative mb-6 max-w-sm">
          <span className="absolute inset-y-0 left-3 flex items-center text-stitch-muted pointer-events-none">
            <span className="material-symbols-outlined text-sm">search</span>
          </span>
          <input
            type="search"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Filter groups…"
            className="w-full pl-10 pr-4 py-2 bg-stitch-elevated border border-stitch-border rounded-md text-sm text-stitch-fg placeholder:text-stitch-muted focus:ring-1 focus:ring-stitch-accent focus:border-stitch-accent outline-none"
          />
        </div>

        {loading ? (
          <div className="text-stitch-muted text-sm py-12 text-center">Loading groups…</div>
        ) : error ? (
          <div className="rounded-xl border border-red-500/40 bg-red-500/10 p-4 text-sm text-red-800 dark:text-red-100">
            {error}
          </div>
        ) : filtered.length === 0 ? (
          <div className="text-center py-16 text-stitch-muted">
            <span className="material-symbols-outlined text-4xl mb-3 block">workspaces</span>
            <p className="text-sm">
              {search.trim() ? 'No groups match your search.' : 'No groups yet.'}
            </p>
            {!search.trim() && (
              <Link
                to="/groups/new"
                className="inline-block mt-4 text-sm font-semibold text-stitch-accent hover:underline"
              >
                Create your first group
              </Link>
            )}
          </div>
        ) : (
          <div className="bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-stitch-border bg-stitch-elevated text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
                  <th className="text-left px-4 py-3">Name</th>
                  <th className="text-left px-4 py-3">Slug</th>
                  <th className="text-left px-4 py-3 hidden sm:table-cell">Description</th>
                  <th className="text-left px-4 py-3 hidden md:table-cell">Created</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-stitch-border">
                {filtered.map((g) => (
                  <tr key={g.id} className="hover:bg-stitch-elevated/60 transition-colors">
                    <td className="px-4 py-3">
                      <Link
                        to={`/groups/${g.id}`}
                        className="font-semibold text-stitch-accent hover:underline"
                      >
                        {g.name}
                      </Link>
                    </td>
                    <td className="px-4 py-3 font-mono text-xs text-stitch-muted">{g.slug}</td>
                    <td className="px-4 py-3 text-stitch-muted hidden sm:table-cell truncate max-w-[200px]">
                      {g.description || '—'}
                    </td>
                    <td className="px-4 py-3 text-stitch-muted text-xs font-mono hidden md:table-cell">
                      {g.created_at.slice(0, 10)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}

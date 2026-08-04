import { FormEvent, useCallback, useEffect, useState } from 'react';
import { Link, useOutletContext } from 'react-router-dom';
import { createBaseline, listBaselines, listSavedViews } from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';
import type { Baseline, SavedView } from '@/api/types';
import type { ProjectOutletContext } from '@/types/projectOutlet';

export default function BaselinesPage() {
  const { projectId, basePath } = useOutletContext<ProjectOutletContext>();
  const pid = projectId;
  const { csrfToken, dashboard } = useDashboard();

  const [rows, setRows] = useState<Baseline[]>([]);
  const [views, setViews] = useState<SavedView[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [savedViewId, setSavedViewId] = useState<number | ''>('');
  const [creating, setCreating] = useState(false);
  const [createErr, setCreateErr] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setLoading(true);
    setErr(null);
    try {
      const [list, sv] = await Promise.all([listBaselines(pid), listSavedViews(pid)]);
      setRows(list);
      setViews(sv.filter((v) => !v.locked));
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load baselines');
    } finally {
      setLoading(false);
    }
  }, [pid]);

  useEffect(() => {
    void load();
  }, [load]);

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  async function onCreate(e: FormEvent) {
    e.preventDefault();
    const token = csrfToken ?? '';
    if (!token) {
      setCreateErr('Missing CSRF token.');
      return;
    }
    if (!name.trim()) return;
    setCreateErr(null);
    setCreating(true);
    try {
      await createBaseline(
        pid,
        name.trim(),
        description.trim() || null,
        token,
        savedViewId === '' ? null : savedViewId,
      );
      setName('');
      setDescription('');
      setSavedViewId('');
      await load();
    } catch (e) {
      setCreateErr(e instanceof Error ? e.message : 'Create failed');
    } finally {
      setCreating(false);
    }
  }

  if (loading) {
    return (
      <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
        Loading baselines…
      </div>
    );
  }

  if (err) {
    return (
      <div className="p-4 rounded-xl bg-red-500/10 border border-red-500/25 text-red-200 text-sm">
        {err}
      </div>
    );
  }

  return (
    <div>
      <StitchPageHeader
        projectName={projectName}
        section="Baselines"
        title="Baselines"
        subtitle="Immutable snapshots of requirements and traceability. Same data as the classic baseline pages."
      >
        <a
          href={`${basePath}/baselines`}
          className="text-xs font-bold uppercase tracking-wider text-stitch-accent border border-stitch-border rounded-md px-3 py-2 hover:bg-stitch-higher"
        >
          Classic baselines
        </a>
      </StitchPageHeader>

      <form
        onSubmit={onCreate}
        className="mb-8 rounded-xl border border-stitch-border bg-stitch-surface p-5 shadow-stitch space-y-4 max-w-xl"
      >
        <h3 className="text-sm font-bold text-stitch-fg uppercase tracking-wide">Create baseline</h3>
        <div>
          <label className="block text-[10px] font-bold text-stitch-muted uppercase mb-1">
            Name
          </label>
          <input
            required
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="w-full text-sm bg-stitch-elevated border border-stitch-border rounded-md px-3 py-2 text-stitch-fg"
            placeholder="e.g. PDR freeze"
          />
        </div>
        <div>
          <label className="block text-[10px] font-bold text-stitch-muted uppercase mb-1">
            Description (optional)
          </label>
          <input
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            className="w-full text-sm bg-stitch-elevated border border-stitch-border rounded-md px-3 py-2 text-stitch-fg"
          />
        </div>
        <div>
          <label className="block text-[10px] font-bold text-stitch-muted uppercase mb-1">
            From saved view (optional)
          </label>
          <select
            value={savedViewId === '' ? '' : String(savedViewId)}
            onChange={(e) =>
              setSavedViewId(e.target.value ? Number(e.target.value) : '')
            }
            className="w-full text-sm bg-stitch-elevated border border-stitch-border rounded-md px-3 py-2 text-stitch-fg"
          >
            <option value="">Entire project</option>
            {views.map((v) => (
              <option key={v.id} value={v.id}>
                {v.name} ({v.visibility})
              </option>
            ))}
          </select>
          <p className="mt-1 text-[10px] text-stitch-muted">
            Using a saved view snapshots matching requirements and locks that view.
          </p>
        </div>
        {createErr && <p className="text-sm text-red-300">{createErr}</p>}
        <button
          type="submit"
          disabled={creating}
          className="bg-stitch-accent text-stitch-canvas px-4 py-2 rounded-md text-xs font-bold uppercase tracking-widest disabled:opacity-50"
        >
          {creating ? 'Creating…' : 'Create'}
        </button>
      </form>

      <div className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
        <table className="w-full text-left text-sm">
          <thead>
            <tr className="border-b border-stitch-border bg-stitch-elevated text-[10px] text-stitch-muted uppercase tracking-widest">
              <th className="px-4 py-3">Name</th>
              <th className="px-4 py-3">Source view</th>
              <th className="px-4 py-3">Created</th>
              <th className="px-4 py-3 text-right">Open</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-stitch-border">
            {rows.length === 0 ? (
              <tr>
                <td colSpan={4} className="px-4 py-8 text-center text-stitch-muted">
                  No baselines yet.
                </td>
              </tr>
            ) : (
              rows.map((b) => (
                <tr key={b.id} className="hover:bg-white/[0.03]">
                  <td className="px-4 py-3 text-stitch-fg font-medium">{b.name}</td>
                  <td className="px-4 py-3 text-stitch-muted text-xs">
                    {b.source_saved_view_id != null ? `#${b.source_saved_view_id}` : '—'}
                  </td>
                  <td className="px-4 py-3 text-stitch-muted text-xs font-mono">
                    {b.created_at?.replace('T', ' ').slice(0, 16) ?? '—'}
                  </td>
                  <td className="px-4 py-3 text-right">
                    <Link
                      to={`${basePath}/baselines/${b.id}`}
                      className="text-xs font-bold text-stitch-accent hover:underline"
                    >
                      Details
                    </Link>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

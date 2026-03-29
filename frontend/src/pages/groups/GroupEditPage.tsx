import { FormEvent, useCallback, useEffect, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { getGroup, updateGroup } from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type { GroupResponse } from '@/api/types';

export default function GroupEditPage() {
  const { groupId } = useParams();
  const gid = Number(groupId);
  const navigate = useNavigate();
  const { csrfToken } = useDashboard();

  const [group, setGroup] = useState<GroupResponse | null>(null);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!Number.isFinite(gid)) return;
    setLoading(true);
    try {
      const g = await getGroup(gid);
      setGroup(g);
      setName(g.name);
      setDescription(g.description ?? '');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load group');
    } finally {
      setLoading(false);
    }
  }, [gid]);

  useEffect(() => {
    void load();
  }, [load]);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    const token = csrfToken ?? '';
    if (!token) {
      setError('Missing CSRF token; refresh the page.');
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await updateGroup(gid, { name: name.trim(), description: description.trim() || null }, token);
      navigate(`/groups/${gid}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update group');
    } finally {
      setSaving(false);
    }
  }

  const inputClass =
    'w-full text-sm font-medium bg-stitch-elevated border border-stitch-border rounded-md px-3 py-2 text-stitch-fg focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/40 outline-none transition-colors';

  if (loading) {
    return (
      <div className="min-h-screen bg-stitch-canvas flex items-center justify-center text-stitch-muted text-sm">
        Loading…
      </div>
    );
  }

  if (!group) {
    return (
      <div className="min-h-screen bg-stitch-canvas text-stitch-fg">
        <div className="max-w-xl mx-auto px-6 py-10">
          <div className="rounded-xl border border-red-500/40 bg-red-500/10 p-4 text-sm text-red-800 dark:text-red-100">
            {error ?? 'Group not found'}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-stitch-canvas text-stitch-fg">
      <div className="max-w-xl mx-auto px-6 py-10">
        <nav className="flex items-center gap-2 text-[10px] font-semibold text-stitch-muted mb-6 uppercase tracking-widest">
          <Link to="/groups" className="hover:text-stitch-accent transition-colors">
            Groups
          </Link>
          <span className="material-symbols-outlined text-sm">chevron_right</span>
          <Link to={`/groups/${gid}`} className="hover:text-stitch-accent transition-colors">
            {group.name}
          </Link>
          <span className="material-symbols-outlined text-sm">chevron_right</span>
          <span className="text-stitch-accent font-bold">Edit</span>
        </nav>

        <h1 className="text-2xl font-bold font-headline tracking-tight mb-2">Edit group</h1>
        <p className="text-sm text-stitch-muted mb-8">
          Update the group name or description. The slug is derived from the name.
        </p>

        <form onSubmit={onSubmit} className="space-y-6">
          <section className="bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch p-6 space-y-5">
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                Group name
              </label>
              <input
                required
                value={name}
                onChange={(e) => setName(e.target.value)}
                className={inputClass}
              />
            </div>
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                Description (optional)
              </label>
              <textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                rows={3}
                className={`${inputClass} resize-y min-h-[80px]`}
              />
            </div>
            <div>
              <span className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider">
                Current slug
              </span>
              <p className="font-mono text-xs text-stitch-fg mt-1">/{group.slug}</p>
            </div>
          </section>

          {error && (
            <div className="rounded-lg bg-red-500/15 border border-red-500/30 text-red-800 dark:text-red-100 text-sm px-4 py-3">
              {error}
            </div>
          )}

          <div className="flex items-center justify-between gap-3">
            <Link
              to={`/groups/${gid}`}
              className="text-xs font-bold uppercase tracking-wider text-stitch-muted hover:text-stitch-fg transition-colors px-2 py-2"
            >
              Cancel
            </Link>
            <button
              type="submit"
              disabled={saving || !name.trim()}
              className="bg-gradient-to-br from-[#000666] to-[#1a237e] text-white px-6 py-2.5 rounded-md text-xs font-bold uppercase tracking-widest shadow-lg disabled:opacity-50 hover:opacity-95 transition-opacity"
            >
              {saving ? 'Saving…' : 'Save changes'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

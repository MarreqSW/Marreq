import { useCallback, useEffect, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import {
  deleteGroup,
  getGroup,
  listGroupMembers,
  listGroupProjects,
  listUsersOptional,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type { GroupMemberResponse, GroupResponse, Project, User } from '@/api/types';

const ROLE_LABELS: Record<number, string> = {
  1: 'Owner',
  2: 'Maintainer',
  3: 'Contributor',
  4: 'Viewer',
};

export default function GroupViewPage() {
  const { groupId } = useParams();
  const gid = Number(groupId);
  const navigate = useNavigate();
  const { csrfToken, dashboard } = useDashboard();
  const currentUserId = (dashboard?.user as { id?: number } | undefined)?.id;

  const [group, setGroup] = useState<GroupResponse | null>(null);
  const [members, setMembers] = useState<GroupMemberResponse[]>([]);
  const [projects, setProjects] = useState<Project[]>([]);
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [deleteBusy, setDeleteBusy] = useState(false);

  const load = useCallback(async () => {
    if (!Number.isFinite(gid)) return;
    setLoading(true);
    setError(null);
    try {
      const [g, m, p, u] = await Promise.all([
        getGroup(gid),
        listGroupMembers(gid),
        listGroupProjects(gid),
        listUsersOptional(),
      ]);
      setGroup(g);
      setMembers(m);
      setProjects(p);
      setUsers(u ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load group');
    } finally {
      setLoading(false);
    }
  }, [gid]);

  useEffect(() => {
    void load();
  }, [load]);

  const userMap = new Map(users.map((u) => [u.id, u]));
  const isOwner = members.some((m) => m.user_id === currentUserId && m.role === 1);
  const isAdmin = (dashboard?.user as { is_admin?: boolean } | undefined)?.is_admin ?? false;
  const canManage = isOwner || isAdmin;

  async function handleDelete() {
    if (!window.confirm('Delete this group? Projects in this group will need to be reassigned.'))
      return;
    const token = csrfToken ?? '';
    if (!token) return;
    setDeleteBusy(true);
    try {
      await deleteGroup(gid, token);
      navigate('/groups');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Delete failed');
    } finally {
      setDeleteBusy(false);
    }
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-stitch-canvas flex items-center justify-center text-stitch-muted text-sm">
        Loading group…
      </div>
    );
  }

  if (error || !group) {
    return (
      <div className="min-h-screen bg-stitch-canvas text-stitch-fg">
        <div className="max-w-3xl mx-auto px-6 py-10">
          <div className="rounded-xl border border-red-500/40 bg-red-500/10 p-4 text-sm text-red-800 dark:text-red-100">
            {error ?? 'Group not found'}
            <div className="mt-3">
              <Link to="/groups" className="font-semibold text-stitch-accent underline">
                Back to groups
              </Link>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-stitch-canvas text-stitch-fg">
      <div className="max-w-4xl mx-auto px-6 py-10">
        <nav className="flex items-center gap-2 text-[10px] font-semibold text-stitch-muted mb-6 uppercase tracking-widest">
          <Link to="/groups" className="hover:text-stitch-accent transition-colors">
            Groups
          </Link>
          <span className="material-symbols-outlined text-sm">chevron_right</span>
          <span className="text-stitch-accent font-bold">{group.name}</span>
        </nav>

        <div className="flex flex-wrap items-start justify-between gap-4 mb-8">
          <div>
            <h1 className="text-2xl font-bold font-headline tracking-tight">{group.name}</h1>
            <p className="text-sm text-stitch-muted mt-1 font-mono">/{group.slug}</p>
            {group.description && (
              <p className="text-sm text-stitch-muted mt-2">{group.description}</p>
            )}
          </div>
          {canManage && (
            <div className="flex items-center gap-2">
              <Link
                to={`/groups/${gid}/edit`}
                className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-stitch-border text-stitch-muted hover:text-stitch-accent hover:border-stitch-accent/40 text-[10px] font-bold uppercase tracking-wider transition-colors"
              >
                <span className="material-symbols-outlined text-sm">edit</span>
                Edit
              </Link>
              <Link
                to={`/groups/${gid}/members`}
                className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md border border-stitch-border text-stitch-muted hover:text-stitch-accent hover:border-stitch-accent/40 text-[10px] font-bold uppercase tracking-wider transition-colors"
              >
                <span className="material-symbols-outlined text-sm">group</span>
                Members
              </Link>
            </div>
          )}
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Projects */}
          <div className="lg:col-span-2">
            <div className="bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch overflow-hidden">
              <div className="px-5 py-3 border-b border-stitch-border bg-stitch-elevated flex items-center justify-between">
                <h2 className="text-xs font-bold uppercase tracking-widest text-stitch-muted">
                  Projects ({projects.length})
                </h2>
              </div>
              {projects.length === 0 ? (
                <div className="p-6 text-center text-sm text-stitch-muted">
                  No projects in this group yet.
                </div>
              ) : (
                <ul className="divide-y divide-stitch-border">
                  {projects.map((p) => {
                    const dashProject = dashboard?.projects?.find((dp) => dp.id === p.id);
                    const href = dashProject
                      ? `${dashProject.project_base_path}/dashboard`
                      : `/${group.slug}/${p.slug}/dashboard`;
                    return (
                      <li key={p.id}>
                        <Link
                          to={href}
                          className="flex items-center justify-between px-5 py-3 hover:bg-stitch-elevated/60 transition-colors"
                        >
                          <div className="min-w-0">
                            <span className="font-semibold text-stitch-fg text-sm">{p.name}</span>
                            <span className="block text-xs text-stitch-muted font-mono">
                              /{group.slug}/{p.slug}
                            </span>
                          </div>
                          <span className="material-symbols-outlined text-stitch-muted text-sm">
                            chevron_right
                          </span>
                        </Link>
                      </li>
                    );
                  })}
                </ul>
              )}
            </div>
          </div>

          {/* Members sidebar */}
          <div>
            <div className="bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch overflow-hidden">
              <div className="px-5 py-3 border-b border-stitch-border bg-stitch-elevated flex items-center justify-between">
                <h2 className="text-xs font-bold uppercase tracking-widest text-stitch-muted">
                  Members ({members.length})
                </h2>
                {canManage && (
                  <Link
                    to={`/groups/${gid}/members`}
                    className="text-[10px] font-bold uppercase tracking-wider text-stitch-accent hover:underline"
                  >
                    Manage
                  </Link>
                )}
              </div>
              {members.length === 0 ? (
                <div className="p-5 text-center text-sm text-stitch-muted">No members.</div>
              ) : (
                <ul className="divide-y divide-stitch-border">
                  {members.map((m) => {
                    const u = userMap.get(m.user_id);
                    return (
                      <li key={m.user_id} className="flex items-center justify-between px-5 py-3">
                        <div className="min-w-0">
                          <span className="text-sm font-semibold text-stitch-fg block truncate">
                            {u?.name ?? `User #${m.user_id}`}
                          </span>
                          {u && (
                            <span className="text-xs text-stitch-muted">{u.username}</span>
                          )}
                        </div>
                        <span className="text-[10px] font-bold uppercase tracking-wider text-stitch-muted bg-stitch-elevated px-2 py-0.5 rounded border border-stitch-border">
                          {ROLE_LABELS[m.role] ?? m.role_label}
                        </span>
                      </li>
                    );
                  })}
                </ul>
              )}
            </div>

            {/* Info card */}
            <div className="mt-4 bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch p-5 space-y-3 text-sm">
              <dl className="space-y-2">
                <div>
                  <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider">
                    Created
                  </dt>
                  <dd className="font-mono text-xs text-stitch-fg">{group.created_at.slice(0, 10)}</dd>
                </div>
                <div>
                  <dt className="text-[10px] font-bold text-stitch-muted uppercase tracking-wider">
                    Updated
                  </dt>
                  <dd className="font-mono text-xs text-stitch-fg">{group.updated_at.slice(0, 10)}</dd>
                </div>
              </dl>
              {canManage && (
                <button
                  type="button"
                  disabled={deleteBusy}
                  onClick={() => void handleDelete()}
                  className="text-xs font-bold uppercase tracking-wider text-red-400/90 hover:text-red-300 transition-colors disabled:opacity-40"
                >
                  {deleteBusy ? 'Deleting…' : 'Delete group'}
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

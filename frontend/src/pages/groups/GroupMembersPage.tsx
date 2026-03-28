import { useCallback, useEffect, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import {
  getGroup,
  listGroupMembers,
  listUsersOptional,
  removeGroupMember,
  setGroupMemberRole,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import type { GroupMemberResponse, GroupResponse, User } from '@/api/types';

const ROLES = [
  { value: 1, label: 'Owner' },
  { value: 2, label: 'Maintainer' },
  { value: 3, label: 'Contributor' },
  { value: 4, label: 'Viewer' },
];

export default function GroupMembersPage() {
  const { groupId } = useParams();
  const gid = Number(groupId);
  const { csrfToken } = useDashboard();

  const [group, setGroup] = useState<GroupResponse | null>(null);
  const [members, setMembers] = useState<GroupMemberResponse[]>([]);
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [addUserId, setAddUserId] = useState('');
  const [addRole, setAddRole] = useState(4);
  const [addBusy, setAddBusy] = useState(false);

  const load = useCallback(async () => {
    if (!Number.isFinite(gid)) return;
    setLoading(true);
    setError(null);
    try {
      const [g, m, u] = await Promise.all([
        getGroup(gid),
        listGroupMembers(gid),
        listUsersOptional(),
      ]);
      setGroup(g);
      setMembers(m);
      setUsers(u ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load');
    } finally {
      setLoading(false);
    }
  }, [gid]);

  useEffect(() => {
    void load();
  }, [load]);

  const userMap = new Map(users.map((u) => [u.id, u]));
  const memberIds = new Set(members.map((m) => m.user_id));
  const availableUsers = users.filter((u) => !memberIds.has(u.id));

  async function handleAddMember() {
    const uid = Number(addUserId);
    if (!Number.isFinite(uid)) return;
    const token = csrfToken ?? '';
    if (!token) return;
    setAddBusy(true);
    setActionError(null);
    try {
      await setGroupMemberRole(gid, uid, addRole, token);
      setAddUserId('');
      await load();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : 'Failed to add member');
    } finally {
      setAddBusy(false);
    }
  }

  async function handleChangeRole(userId: number, role: number) {
    const token = csrfToken ?? '';
    if (!token) return;
    setActionError(null);
    try {
      await setGroupMemberRole(gid, userId, role, token);
      await load();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : 'Failed to update role');
    }
  }

  async function handleRemove(userId: number) {
    const u = userMap.get(userId);
    if (!window.confirm(`Remove ${u?.name ?? `User #${userId}`} from this group?`)) return;
    const token = csrfToken ?? '';
    if (!token) return;
    setActionError(null);
    try {
      await removeGroupMember(gid, userId, token);
      await load();
    } catch (e) {
      setActionError(e instanceof Error ? e.message : 'Failed to remove member');
    }
  }

  const selectClass =
    'text-sm font-medium bg-stitch-elevated border border-stitch-border rounded-md px-2 py-1.5 text-stitch-fg focus:border-stitch-accent focus:ring-1 focus:ring-stitch-accent/40 outline-none transition-colors';

  if (loading) {
    return (
      <div className="min-h-screen bg-stitch-canvas flex items-center justify-center text-stitch-muted text-sm">
        Loading…
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
      <div className="max-w-3xl mx-auto px-6 py-10">
        <nav className="flex items-center gap-2 text-[10px] font-semibold text-stitch-muted mb-6 uppercase tracking-widest">
          <Link to="/groups" className="hover:text-stitch-accent transition-colors">
            Groups
          </Link>
          <span className="material-symbols-outlined text-sm">chevron_right</span>
          <Link to={`/groups/${gid}`} className="hover:text-stitch-accent transition-colors">
            {group.name}
          </Link>
          <span className="material-symbols-outlined text-sm">chevron_right</span>
          <span className="text-stitch-accent font-bold">Members</span>
        </nav>

        <h1 className="text-2xl font-bold font-headline tracking-tight mb-2">
          Members of {group.name}
        </h1>
        <p className="text-sm text-stitch-muted mb-8">
          Manage who has access to this group and their roles.
        </p>

        {actionError && (
          <div className="rounded-lg bg-red-500/15 border border-red-500/30 text-red-800 dark:text-red-100 text-sm px-4 py-3 mb-6">
            {actionError}
          </div>
        )}

        {/* Add member */}
        {availableUsers.length > 0 && (
          <div className="bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch p-5 mb-6">
            <h2 className="text-xs font-bold uppercase tracking-widest text-stitch-muted mb-3">
              Add member
            </h2>
            <div className="flex flex-wrap items-end gap-3">
              <div className="min-w-[180px] flex-1">
                <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                  User
                </label>
                <select
                  className={`${selectClass} w-full`}
                  value={addUserId}
                  onChange={(e) => setAddUserId(e.target.value)}
                >
                  <option value="">Select user…</option>
                  {availableUsers.map((u) => (
                    <option key={u.id} value={u.id}>
                      {u.name} ({u.username})
                    </option>
                  ))}
                </select>
              </div>
              <div className="min-w-[140px]">
                <label className="block text-[10px] font-bold text-stitch-muted uppercase tracking-wider mb-1">
                  Role
                </label>
                <select
                  className={`${selectClass} w-full`}
                  value={addRole}
                  onChange={(e) => setAddRole(Number(e.target.value))}
                >
                  {ROLES.map((r) => (
                    <option key={r.value} value={r.value}>
                      {r.label}
                    </option>
                  ))}
                </select>
              </div>
              <button
                type="button"
                disabled={addBusy || !addUserId}
                onClick={() => void handleAddMember()}
                className="bg-gradient-to-br from-[#000666] to-[#1a237e] text-white px-4 py-2 rounded-md text-xs font-bold uppercase tracking-widest shadow-lg disabled:opacity-50 hover:opacity-95 transition-opacity"
              >
                {addBusy ? 'Adding…' : 'Add'}
              </button>
            </div>
          </div>
        )}

        {/* Members table */}
        <div className="bg-stitch-surface rounded-xl border border-stitch-border shadow-stitch overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-stitch-border bg-stitch-elevated text-[10px] font-bold uppercase tracking-widest text-stitch-muted">
                <th className="text-left px-4 py-3">User</th>
                <th className="text-left px-4 py-3">Role</th>
                <th className="text-right px-4 py-3">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-stitch-border">
              {members.map((m) => {
                const u = userMap.get(m.user_id);
                return (
                  <tr key={m.user_id} className="hover:bg-stitch-elevated/60 transition-colors">
                    <td className="px-4 py-3">
                      <span className="font-semibold text-stitch-fg">
                        {u?.name ?? `User #${m.user_id}`}
                      </span>
                      {u && (
                        <span className="text-xs text-stitch-muted ml-2">{u.username}</span>
                      )}
                    </td>
                    <td className="px-4 py-3">
                      <select
                        className={selectClass}
                        value={m.role}
                        onChange={(e) => void handleChangeRole(m.user_id, Number(e.target.value))}
                      >
                        {ROLES.map((r) => (
                          <option key={r.value} value={r.value}>
                            {r.label}
                          </option>
                        ))}
                      </select>
                    </td>
                    <td className="px-4 py-3 text-right">
                      <button
                        type="button"
                        onClick={() => void handleRemove(m.user_id)}
                        className="text-xs font-bold text-red-400/90 hover:text-red-300 transition-colors"
                      >
                        Remove
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}

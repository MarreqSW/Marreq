import { type FormEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { useParams } from 'react-router-dom';
import {
  getMyPermissions,
  listCustomFieldsByProject,
  listProjectMembers,
  listUsersOptional,
  removeProjectMember,
  setProjectMemberRole,
} from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';
import type { CustomFieldDefinition, EffectivePermissions, User } from '@/api/types';

function PermPill({ label, on }: { label: string; on: boolean }) {
  return (
    <span
      className={`inline-flex items-center px-2 py-1 rounded-md text-[10px] font-bold uppercase tracking-wide border ${
        on
          ? 'bg-emerald-500/15 text-emerald-200 border-emerald-500/30'
          : 'bg-white/[0.05] text-stitch-muted border-stitch-border'
      }`}
    >
      {label}
    </span>
  );
}

const ROLES = [
  { id: 1, label: 'Admin' },
  { id: 2, label: 'Reviewer' },
  { id: 3, label: 'Author' },
  { id: 4, label: 'Viewer' },
];

export default function ProjectSettingsPage() {
  const { projectId: projectIdParam } = useParams();
  const pid = Number(projectIdParam);
  const { dashboard, csrfToken } = useDashboard();

  const [perms, setPerms] = useState<EffectivePermissions | null>(null);
  const [members, setMembers] = useState<Awaited<ReturnType<typeof listProjectMembers>>>([]);
  const [fields, setFields] = useState<CustomFieldDefinition[]>([]);
  const [users, setUsers] = useState<User[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [memberErr, setMemberErr] = useState<string | null>(null);
  const [addUserId, setAddUserId] = useState('');
  const [addRole, setAddRole] = useState(4);
  const [memberBusy, setMemberBusy] = useState<number | 'add' | null>(null);

  const load = useCallback(async () => {
    if (!Number.isFinite(pid)) return;
    setLoading(true);
    setErr(null);
    try {
      const [p, m, f, u] = await Promise.all([
        getMyPermissions(pid),
        listProjectMembers(pid),
        listCustomFieldsByProject(pid),
        listUsersOptional(),
      ]);
      setPerms(p);
      setMembers(m);
      setFields(f);
      setUsers(u);
    } catch (e) {
      setErr(e instanceof Error ? e.message : 'Failed to load settings');
    } finally {
      setLoading(false);
    }
  }, [pid]);

  useEffect(() => {
    void load();
  }, [load]);

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  const userLabel = (uid: number) => {
    const u = users?.find((x) => x.id === uid);
    if (u) return `${u.name} (${u.username})`;
    return `User #${uid}`;
  };

  const memberIds = useMemo(() => new Set(members.map((m) => m.user_id)), [members]);

  const usersNotInProject = useMemo(() => {
    if (!users?.length) return [];
    return users.filter((u) => !memberIds.has(u.id));
  }, [users, memberIds]);

  const canManage = perms?.manage_project_members && (csrfToken ?? '').length > 0;

  async function updateRole(userId: number, role: number) {
    const token = csrfToken ?? '';
    if (!token) return;
    setMemberErr(null);
    setMemberBusy(userId);
    try {
      await setProjectMemberRole(pid, userId, role, token);
      await load();
    } catch (e) {
      setMemberErr(e instanceof Error ? e.message : 'Update failed');
    } finally {
      setMemberBusy(null);
    }
  }

  async function removeMember(userId: number) {
    const token = csrfToken ?? '';
    if (!token) return;
    if (!window.confirm(`Remove ${userLabel(userId)} from this project?`)) return;
    setMemberErr(null);
    setMemberBusy(userId);
    try {
      await removeProjectMember(pid, userId, token);
      await load();
    } catch (e) {
      setMemberErr(e instanceof Error ? e.message : 'Remove failed');
    } finally {
      setMemberBusy(null);
    }
  }

  async function addMember(e: FormEvent) {
    e.preventDefault();
    const token = csrfToken ?? '';
    const uid = Number(addUserId);
    if (!token || !Number.isFinite(uid)) return;
    setMemberErr(null);
    setMemberBusy('add');
    try {
      await setProjectMemberRole(pid, uid, addRole, token);
      setAddUserId('');
      await load();
    } catch (e) {
      setMemberErr(e instanceof Error ? e.message : 'Add failed');
    } finally {
      setMemberBusy(null);
    }
  }

  if (loading) {
    return (
      <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
        Loading settings…
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
        section="Settings"
        title="Project settings"
        subtitle="Permissions, members (with API parity when allowed), and custom field definitions. Editing field schemas remains in the classic UI."
      />

      <section className="mb-10">
        <h3 className="text-sm font-bold text-white uppercase tracking-widest mb-4">
          Your permissions
        </h3>
        {perms ? (
          <div className="flex flex-wrap gap-2">
            <PermPill label="View requirements" on={perms.view_requirements} />
            <PermPill label="Edit requirements" on={perms.edit_requirements} />
            <PermPill label="Approve versions" on={perms.approve_versions} />
            <PermPill label="Manage custom fields" on={perms.manage_custom_fields} />
            <PermPill label="Manage members" on={perms.manage_project_members} />
          </div>
        ) : null}
      </section>

      <section className="mb-10">
        <h3 className="text-sm font-bold text-white uppercase tracking-widest mb-4">
          Project members
        </h3>
        {memberErr && (
          <p className="text-sm text-red-300 mb-3">{memberErr}</p>
        )}
        {canManage && usersNotInProject.length > 0 && (
          <form
            onSubmit={addMember}
            className="mb-4 flex flex-wrap items-end gap-3 rounded-xl border border-stitch-border bg-stitch-elevated p-4"
          >
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase mb-1">
                Add user
              </label>
              <select
                value={addUserId}
                onChange={(e) => setAddUserId(e.target.value)}
                className="text-sm bg-stitch-surface border border-stitch-border rounded-md px-2 py-2 text-white min-w-[200px]"
                required
              >
                <option value="">Select account…</option>
                {usersNotInProject.map((u) => (
                  <option key={u.id} value={u.id} className="bg-stitch-surface">
                    {u.name} ({u.username})
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className="block text-[10px] font-bold text-stitch-muted uppercase mb-1">
                Role
              </label>
              <select
                value={addRole}
                onChange={(e) => setAddRole(Number(e.target.value))}
                className="text-sm bg-stitch-surface border border-stitch-border rounded-md px-2 py-2 text-white"
              >
                {ROLES.map((r) => (
                  <option key={r.id} value={r.id} className="bg-stitch-surface">
                    {r.label}
                  </option>
                ))}
              </select>
            </div>
            <button
              type="submit"
              disabled={memberBusy === 'add'}
              className="bg-stitch-accent text-stitch-canvas text-xs font-bold uppercase px-4 py-2 rounded-md disabled:opacity-50"
            >
              {memberBusy === 'add' ? '…' : 'Add'}
            </button>
          </form>
        )}
        {canManage && users === null && (
          <p className="text-xs text-amber-200/90 mb-4">
            User directory is admin-only.{' '}
            <a href={`/p/${pid}/members`} className="text-stitch-accent underline font-semibold">
              Open classic members page
            </a>{' '}
            to add people by account.
          </p>
        )}
        <div className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-stitch-border bg-stitch-elevated text-[10px] text-stitch-muted uppercase tracking-widest">
                <th className="px-4 py-3">User</th>
                <th className="px-4 py-3">Role</th>
                {canManage ? <th className="px-4 py-3 text-right">Actions</th> : null}
              </tr>
            </thead>
            <tbody className="divide-y divide-stitch-border">
              {members.map((m) => (
                <tr key={m.user_id} className="hover:bg-white/[0.03]">
                  <td className="px-4 py-3 text-white">{userLabel(m.user_id)}</td>
                  <td className="px-4 py-3">
                    {canManage ? (
                      <select
                        value={m.role}
                        disabled={memberBusy === m.user_id}
                        onChange={(e) => void updateRole(m.user_id, Number(e.target.value))}
                        className="text-xs bg-stitch-elevated border border-stitch-border rounded-md px-2 py-1.5 text-white"
                      >
                        {ROLES.map((r) => (
                          <option key={r.id} value={r.id} className="bg-stitch-surface">
                            {r.label}
                          </option>
                        ))}
                      </select>
                    ) : (
                      <>
                        <span className="text-xs font-semibold text-stitch-accent">{m.role_label}</span>
                        <span className="text-[10px] text-stitch-muted ml-2">({m.role})</span>
                      </>
                    )}
                  </td>
                  {canManage ? (
                    <td className="px-4 py-3 text-right">
                      <button
                        type="button"
                        disabled={memberBusy === m.user_id}
                        onClick={() => void removeMember(m.user_id)}
                        className="text-xs font-bold text-red-300 hover:underline disabled:opacity-40"
                      >
                        Remove
                      </button>
                    </td>
                  ) : null}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {!canManage && (
          <p className="text-xs text-stitch-muted mt-3">
            You need “Manage members” to change roles here.{' '}
            <a href={`/p/${pid}/members`} className="text-stitch-accent underline">
              Classic members UI
            </a>
          </p>
        )}
      </section>

      <section>
        <h3 className="text-sm font-bold text-white uppercase tracking-widest mb-4">
          Custom fields
        </h3>
        <div className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
          <table className="w-full text-left text-sm">
            <thead>
              <tr className="border-b border-stitch-border bg-stitch-elevated text-[10px] text-stitch-muted uppercase tracking-widest">
                <th className="px-4 py-3">Label</th>
                <th className="px-4 py-3">Type</th>
                <th className="px-4 py-3">Order</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-stitch-border">
              {fields.length === 0 ? (
                <tr>
                  <td colSpan={3} className="px-4 py-8 text-center text-stitch-muted">
                    No custom fields defined for this project.
                  </td>
                </tr>
              ) : (
                fields.map((f) => (
                  <tr key={f.id} className="hover:bg-white/[0.03]">
                    <td className="px-4 py-3 text-white font-medium">{f.label}</td>
                    <td className="px-4 py-3 text-stitch-muted font-mono text-xs">{f.field_type}</td>
                    <td className="px-4 py-3 text-stitch-muted tabular-nums">{f.sort_order}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}

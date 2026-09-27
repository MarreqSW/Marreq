import { useCallback, useEffect, useState } from 'react';
import { Link, useOutletContext } from 'react-router-dom';
import { deleteUser, getCsrfToken, getDeploymentInfo, listUsersOptional } from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';
import type { DeploymentInfo, User } from '@/api/types';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import { parseUser } from '@/utils/parseUser';
import { btnDanger } from '@/pages/catalog/catalogUi';
import SetPasswordDialog from '@/pages/admin/SetPasswordDialog';
import UserFormDialog from '@/pages/admin/UserFormDialog';

const headerBtn =
  'text-xs font-bold uppercase tracking-wider text-stitch-accent border border-stitch-border rounded-md px-3 py-2 hover:bg-stitch-higher';
const rowBtn =
  'text-xs font-bold uppercase text-stitch-accent hover:text-stitch-fg disabled:opacity-40';

export default function AdminPage() {
  const { projectId: pid, basePath } = useOutletContext<ProjectOutletContext>();
  const { dashboard, csrfToken } = useDashboard();

  const me = parseUser(dashboard?.user);
  const [users, setUsers] = useState<User[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [deployment, setDeployment] = useState<DeploymentInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);
  // `undefined` = closed, `null` = creating, a user = editing.
  const [editing, setEditing] = useState<User | null | undefined>(undefined);
  const [passwordFor, setPasswordFor] = useState<User | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await listUsersOptional();
      setUsers(list);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    let alive = true;
    getDeploymentInfo()
      .then((info) => {
        if (alive) setDeployment(info);
      })
      .catch(() => {
        if (alive) setDeployment(null);
      });
    return () => {
      alive = false;
    };
  }, []);

  const token = useCallback(async () => csrfToken ?? (await getCsrfToken()), [csrfToken]);

  const canCreate = deployment?.allows_self_administered_user_creation === true;
  const allowsAdminPromotion = deployment?.allows_admin_promotion === true;

  async function removeUser(u: User) {
    if (!window.confirm(`Delete user "${u.username}"? This cannot be undone.`)) return;
    setBusyId(u.id);
    setError(null);
    setNotice(null);
    try {
      await deleteUser(u.id, await token());
      setNotice(`Deleted ${u.username}.`);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Delete failed');
    } finally {
      setBusyId(null);
    }
  }

  async function afterSave(message: string) {
    setEditing(undefined);
    setPasswordFor(null);
    setError(null);
    setNotice(message);
    await load();
  }

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  if (loading) {
    return (
      <div className="p-8 text-center text-stitch-muted text-sm border border-stitch-border rounded-xl bg-stitch-surface">
        Loading…
      </div>
    );
  }

  if (users === null) {
    return (
      <div>
        <StitchPageHeader
          projectName={projectName}
          section="Admin"
          title="Administration"
          subtitle="Restricted area."
        />
        <div className="rounded-xl border border-stitch-border bg-stitch-surface p-8 text-center">
          <span className="material-symbols-outlined text-4xl text-stitch-muted mb-3 block">lock</span>
          <p className="text-stitch-fg font-semibold">Access denied</p>
          <p className="text-sm text-stitch-muted mt-2 max-w-md mx-auto">
            Listing users requires a global administrator account. You are signed in as{' '}
            <span className="text-stitch-accent">{me?.username ?? '?'}</span>.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div>
      <StitchPageHeader
        projectName={projectName}
        section="Admin"
        title="User directory"
        subtitle={
          canCreate || deployment === null
            ? 'Create, edit, and remove accounts, set passwords, and grant administrator rights.'
            : 'Users self-register in this deployment. Edit, reset passwords, or remove accounts here.'
        }
      >
        {canCreate ? (
          <button
            type="button"
            onClick={() => setEditing(null)}
            className="bg-stitch-accent text-stitch-canvas px-3 py-2 rounded-md text-xs font-bold uppercase tracking-wider"
          >
            New user
          </button>
        ) : null}
        <Link to={`${basePath}/admin/logs`} className={headerBtn}>
          System logs
        </Link>
        {deployment?.allows_database_backup !== false ? (
          <Link to={`${basePath}/admin/backup`} className={headerBtn}>
            Backup
          </Link>
        ) : null}
        <button type="button" onClick={() => void load()} className={headerBtn}>
          Refresh
        </button>
      </StitchPageHeader>

      {error ? (
        <div
          role="alert"
          className="mb-4 rounded-lg border border-red-500/30 bg-red-500/10 text-red-800 dark:text-red-100 text-sm px-4 py-2"
        >
          {error}
        </div>
      ) : null}
      {notice ? (
        <div
          role="status"
          className="mb-4 rounded-lg border border-emerald-500/30 bg-emerald-500/10 text-emerald-900 dark:text-emerald-100 text-sm px-4 py-2"
        >
          {notice}
        </div>
      ) : null}

      <div className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
        <table className="w-full text-left text-sm">
          <thead>
            <tr className="border-b border-stitch-border bg-stitch-elevated text-[10px] text-stitch-muted uppercase tracking-widest">
              <th className="px-4 py-3">Username</th>
              <th className="px-4 py-3">Name</th>
              <th className="px-4 py-3">Email</th>
              <th className="px-4 py-3 text-center">Admin</th>
              <th className="px-4 py-3 text-right">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-stitch-border">
            {users.map((u) => (
              <tr key={u.id} className="hover:bg-white/3">
                <td className="px-4 py-3 font-mono text-stitch-accent">{u.username}</td>
                <td className="px-4 py-3 text-stitch-fg">{u.name}</td>
                <td className="px-4 py-3 text-stitch-muted text-xs">{u.email}</td>
                <td className="px-4 py-3 text-center">
                  {u.is_admin ? (
                    <span className="text-[10px] font-bold uppercase text-emerald-300">Yes</span>
                  ) : (
                    <span className="text-stitch-muted">—</span>
                  )}
                </td>
                <td className="px-4 py-3">
                  <div className="flex justify-end gap-3">
                    <button type="button" className={rowBtn} onClick={() => setEditing(u)}>
                      Edit
                    </button>
                    <button type="button" className={rowBtn} onClick={() => setPasswordFor(u)}>
                      Set password
                    </button>
                    <button
                      type="button"
                      className={btnDanger}
                      onClick={() => void removeUser(u)}
                      disabled={u.id === me?.id || busyId === u.id}
                      title={u.id === me?.id ? 'You cannot delete your own account' : undefined}
                    >
                      Delete
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <UserFormDialog
        open={editing !== undefined}
        user={editing ?? null}
        allowsAdminPromotion={allowsAdminPromotion}
        isSelf={editing != null && editing.id === me?.id}
        getCsrfToken={token}
        onClose={() => setEditing(undefined)}
        onSaved={() => void afterSave(editing ? `Saved ${editing.username}.` : 'User created.')}
      />
      <SetPasswordDialog
        user={passwordFor}
        getCsrfToken={token}
        onClose={() => setPasswordFor(null)}
        onSaved={(u) => void afterSave(`Password set for ${u.username}.`)}
      />
    </div>
  );
}

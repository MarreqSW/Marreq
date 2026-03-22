import { useCallback, useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { listUsersOptional } from '@/api/client';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';
import type { User } from '@/api/types';

function parseUser(u: unknown): User | null {
  if (u && typeof u === 'object' && 'username' in u) return u as User;
  return null;
}

export default function AdminPage() {
  const { projectId: projectIdParam } = useParams();
  const pid = Number(projectIdParam);
  const { dashboard } = useDashboard();

  const me = parseUser(dashboard?.user);
  const [users, setUsers] = useState<User[] | null>(null);
  const [loading, setLoading] = useState(true);

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
          <p className="text-white font-semibold">Access denied</p>
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
        subtitle="All accounts in the system (admin API). User management actions remain in the legacy admin UI for now."
      >
        <button
          type="button"
          onClick={() => void load()}
          className="text-xs font-bold uppercase tracking-wider text-stitch-accent border border-stitch-border rounded-md px-3 py-2 hover:bg-white/[0.06]"
        >
          Refresh
        </button>
      </StitchPageHeader>

      <div className="bg-stitch-surface rounded-xl border border-stitch-border overflow-hidden shadow-stitch">
        <table className="w-full text-left text-sm">
          <thead>
            <tr className="border-b border-stitch-border bg-stitch-elevated text-[10px] text-stitch-muted uppercase tracking-widest">
              <th className="px-4 py-3">Username</th>
              <th className="px-4 py-3">Name</th>
              <th className="px-4 py-3">Email</th>
              <th className="px-4 py-3 text-center">Admin</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-stitch-border">
            {users.map((u) => (
              <tr key={u.id} className="hover:bg-white/[0.03]">
                <td className="px-4 py-3 font-mono text-stitch-accent">{u.username}</td>
                <td className="px-4 py-3 text-white">{u.name}</td>
                <td className="px-4 py-3 text-stitch-muted text-xs">{u.email}</td>
                <td className="px-4 py-3 text-center">
                  {u.is_admin ? (
                    <span className="text-[10px] font-bold uppercase text-emerald-300">Yes</span>
                  ) : (
                    <span className="text-stitch-muted">—</span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

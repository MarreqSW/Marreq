import { useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { listCreatableGroups } from '@/api/client';
import type { GroupResponse } from '@/api/types';
import ProjectCreateForm from '@/components/ProjectCreateForm';
import { useDashboard } from '@/context/DashboardContext';
import { parseUser } from '@/utils/parseUser';

export default function ProjectCreatePage() {
  const navigate = useNavigate();
  const { csrfToken, dashboard, refresh } = useDashboard();
  const user = useMemo(() => parseUser(dashboard?.user), [dashboard?.user]);
  const [groups, setGroups] = useState<GroupResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listCreatableGroups()
      .then(setGroups)
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : 'Failed to load namespaces');
      })
      .finally(() => setLoading(false));
  }, []);

  const personalNamespace = user?.username ?? 'Personal';

  return (
    <div className="min-h-screen bg-stitch-canvas text-stitch-fg">
      <div className="max-w-2xl mx-auto px-6 py-10">
        <nav className="flex items-center gap-2 text-xs text-stitch-muted mb-6">
          <Link to="/" className="hover:text-stitch-accent">Dashboard</Link>
          <span aria-hidden="true">/</span>
          <span>New project</span>
        </nav>
        <div className="rounded-xl border border-stitch-border bg-stitch-surface p-6 shadow-stitch">
          <h1 className="text-2xl font-bold font-headline tracking-tight">New project</h1>
          <p className="text-sm text-stitch-muted mt-1 mb-6">
            Create a personal project or choose a group namespace you manage.{' '}
            <Link to="/projects/import-bundle" className="text-stitch-accent hover:underline">
              Import from JSON bundle
            </Link>
          </p>

          {loading ? (
            <div className="text-sm text-stitch-muted">Loading namespaces…</div>
          ) : error ? (
            <div role="alert" className="text-sm text-red-400">{error}</div>
          ) : (
            <ProjectCreateForm
              csrfToken={csrfToken ?? ''}
              personalNamespace={personalNamespace}
              groups={groups}
              onCreated={async (project) => {
                await refresh();
                navigate(`${project.project_base_path}/dashboard`, { replace: true });
              }}
              onCancel={() => navigate(-1)}
            />
          )}
        </div>
      </div>
    </div>
  );
}

import { useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useDashboard } from '@/context/DashboardContext';
import type { User } from '@/api/types';
import { parseUser } from '@/utils/parseUser';
import NoProjectsHome from '@/pages/NoProjectsHome';

export default function HomeRedirect() {
  const navigate = useNavigate();
  const { dashboard, loading } = useDashboard();

  const user = useMemo(() => parseUser(dashboard?.user), [dashboard?.user]);
  const projects = dashboard?.projects;
  const hasProjects = (projects?.length ?? 0) > 0;

  useEffect(() => {
    const pl = dashboard?.projects;
    if (!pl?.length) return;
    const id = dashboard?.selected_project_id ?? pl[0]!.id;
    const project = pl.find((p) => p.id === id) ?? pl[0]!;
    navigate(`${project.project_base_path}/dashboard`, { replace: true });
  }, [dashboard, navigate]);

  if (!dashboard) {
    if (loading) {
      return (
        <div className="min-h-screen flex items-center justify-center bg-stitch-canvas text-stitch-muted text-sm">
          Loading…
        </div>
      );
    }
    return null;
  }

  if (!hasProjects) {
    const isAdmin = user?.is_admin ?? false;
    const displayName = (user?.name?.trim() || user?.username || '').trim();
    return <NoProjectsHome isAdmin={isAdmin} displayName={displayName} />;
  }

  return (
    <div className="min-h-screen flex items-center justify-center text-stitch-muted text-sm bg-stitch-canvas">
      Opening project…
    </div>
  );
}

import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useDashboard } from '@/context/DashboardContext';

export default function HomeRedirect() {
  const navigate = useNavigate();
  const { dashboard } = useDashboard();

  useEffect(() => {
    if (!dashboard?.projects?.length) return;
    const id =
      dashboard.selected_project_id ?? dashboard.projects[0]?.id ?? null;
    if (id != null) {
      navigate(`/p/${id}/dashboard`, { replace: true });
    }
  }, [dashboard, navigate]);

  return (
    <div className="min-h-screen flex items-center justify-center text-slate-500 text-sm">
      Opening project…
    </div>
  );
}

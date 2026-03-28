import { useMemo } from 'react';
import { useOutletContext, useParams, useSearchParams } from 'react-router-dom';
import RequirementsTable from '@/components/RequirementsTable';
import RequirementsViewSwitcher from '@/components/RequirementsViewSwitcher';
import { useDashboard } from '@/context/DashboardContext';
import type { ProjectOutletContext } from '@/types/projectOutlet';

export default function RequirementsPage() {
  const { projectId, globalSearch, basePath } = useOutletContext<ProjectOutletContext>();
  const [searchParams] = useSearchParams();
  const { projectId: projectIdParam } = useParams();
  const { dashboard } = useDashboard();
  const pid = Number(projectIdParam);

  const projectName = useMemo(() => {
    const p = dashboard?.projects?.find((x) => x.id === pid);
    return p?.name ?? 'Project';
  }, [dashboard?.projects, pid]);

  const viewMode = searchParams.get('view') === 'list' ? 'list' : 'table';

  return (
    <div>
      <div className="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between mb-8">
        <div>
          <nav className="flex text-[10px] text-stitch-muted font-mono uppercase tracking-widest mb-1">
            <span>{projectName}</span>
            <span className="mx-2">/</span>
            <span className="text-stitch-accent font-bold">Requirements</span>
          </nav>
          <h2 className="text-2xl md:text-3xl font-extrabold text-stitch-fg tracking-tight font-headline">
            System specifications
          </h2>
        </div>
        <RequirementsViewSwitcher />
      </div>

      <RequirementsTable projectId={projectId} basePath={basePath} globalSearch={globalSearch} viewMode={viewMode} />
    </div>
  );
}

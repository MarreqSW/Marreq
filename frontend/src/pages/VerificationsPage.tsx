import { useMemo } from 'react';
import { useOutletContext, useSearchParams } from 'react-router-dom';
import VerificationsTable from '@/components/VerificationsTable';
import VerificationsViewSwitcher from '@/components/VerificationsViewSwitcher';
import { useDashboard } from '@/context/DashboardContext';
import type { ProjectOutletContext } from '@/types/projectOutlet';

export default function VerificationsPage() {
  const { projectId, globalSearch, basePath } = useOutletContext<ProjectOutletContext>();
  const [searchParams] = useSearchParams();
  const { dashboard } = useDashboard();

  const projectName = useMemo(() => {
    const p = dashboard?.projects?.find((x) => x.id === projectId);
    return p?.name ?? 'Project';
  }, [dashboard?.projects, projectId]);

  const viewMode = searchParams.get('view') === 'list' ? 'list' : 'table';

  return (
    <div>
      <div className="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between mb-8">
        <div>
          <nav className="flex text-[10px] text-stitch-muted font-mono uppercase tracking-widest mb-1">
            <span>{projectName}</span>
            <span className="mx-2">/</span>
            <span className="text-stitch-accent font-bold">Verifications</span>
          </nav>
          <h2 className="text-2xl md:text-3xl font-extrabold text-stitch-fg tracking-tight font-headline">
            Verification registry
          </h2>
        </div>
        <VerificationsViewSwitcher />
      </div>

      <VerificationsTable projectId={projectId} basePath={basePath} globalSearch={globalSearch} viewMode={viewMode} />
    </div>
  );
}

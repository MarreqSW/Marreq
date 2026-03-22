import { useMemo } from 'react';
import { useOutletContext, useParams } from 'react-router-dom';
import TraceabilityGraph from '@/components/TraceabilityGraph';
import RequirementsViewSwitcher from '@/components/RequirementsViewSwitcher';
import { useDashboard } from '@/context/DashboardContext';
import type { ProjectOutletContext } from '@/types/projectOutlet';

type Ctx = ProjectOutletContext;

export default function TraceabilityPage() {
  const { projectId } = useOutletContext<Ctx>();
  const { projectId: projectIdParam } = useParams();
  const { dashboard } = useDashboard();
  const pid = Number(projectIdParam);

  const projectName = useMemo(() => {
    const p = dashboard?.projects?.find((x) => x.id === pid);
    return p?.name ?? 'Project';
  }, [dashboard?.projects, pid]);

  return (
    <div>
      <div className="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between mb-8">
        <div>
          <nav className="flex text-[10px] text-stitch-muted font-mono uppercase tracking-widest mb-1">
            <span>{projectName}</span>
            <span className="mx-2">/</span>
            <span className="text-stitch-accent font-bold">Traceability</span>
          </nav>
          <h2 className="text-2xl md:text-3xl font-extrabold text-stitch-fg tracking-tight font-headline">
            Traceability matrix
          </h2>
          <p className="text-stitch-muted text-sm mt-2 max-w-2xl">
            Requirement ↔ verification links. Suspect links animate in coral.
          </p>
        </div>
        <RequirementsViewSwitcher />
      </div>
      <TraceabilityGraph projectId={projectId} />
    </div>
  );
}

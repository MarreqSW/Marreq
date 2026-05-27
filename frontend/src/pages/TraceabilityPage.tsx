import { useMemo } from 'react';
import { useOutletContext, useSearchParams } from 'react-router-dom';
import TraceabilityGraph from '@/components/TraceabilityGraph';
import HierarchyGraph from '@/components/HierarchyGraph';
import RequirementsViewSwitcher from '@/components/RequirementsViewSwitcher';
import { useDashboard } from '@/context/DashboardContext';
import type { ProjectOutletContext } from '@/types/projectOutlet';

type Ctx = ProjectOutletContext;

type GraphView = 'coverage' | 'hierarchy';

const VIEW_PARAM = 'view';

function readView(value: string | null): GraphView {
  return value === 'hierarchy' ? 'hierarchy' : 'coverage';
}

export default function TraceabilityPage() {
  const { projectId, basePath } = useOutletContext<Ctx>();
  const { dashboard } = useDashboard();
  const pid = projectId;
  const [searchParams, setSearchParams] = useSearchParams();
  const view = readView(searchParams.get(VIEW_PARAM));

  const projectName = useMemo(() => {
    const p = dashboard?.projects?.find((x) => x.id === pid);
    return p?.name ?? 'Project';
  }, [dashboard?.projects, pid]);

  const setView = (next: GraphView) => {
    const params = new URLSearchParams(searchParams);
    if (next === 'coverage') {
      params.delete(VIEW_PARAM);
    } else {
      params.set(VIEW_PARAM, next);
    }
    setSearchParams(params, { replace: true });
  };

  const subtitle =
    view === 'hierarchy'
      ? 'Parent ↔ child links between requirements and between verifications.'
      : 'Requirement ↔ verification links. Suspect links animate in coral.';

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
          <p className="text-stitch-muted text-sm mt-2 max-w-2xl">{subtitle}</p>
        </div>
        <RequirementsViewSwitcher />
      </div>

      <div className="flex p-1 bg-stitch-surface rounded-lg gap-1 border border-stitch-border w-fit mb-6">
        <SubtabButton active={view === 'coverage'} onClick={() => setView('coverage')}>
          <span className="material-symbols-outlined text-sm">hub</span>
          Coverage
        </SubtabButton>
        <SubtabButton active={view === 'hierarchy'} onClick={() => setView('hierarchy')}>
          <span className="material-symbols-outlined text-sm">account_tree</span>
          Hierarchy
        </SubtabButton>
      </div>

      {view === 'coverage' ? (
        <TraceabilityGraph projectId={projectId} basePath={basePath} />
      ) : (
        <HierarchyGraph projectId={projectId} basePath={basePath} />
      )}
    </div>
  );
}

function SubtabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  const seg = 'flex items-center gap-2 px-4 py-1.5 text-xs font-bold rounded-md transition-colors';
  const inactive = 'text-stitch-muted hover:bg-stitch-higher hover:text-stitch-fg';
  const activeCls =
    'bg-stitch-elevated text-stitch-accent shadow-stitch-inset border border-stitch-border';
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={`${seg} ${active ? activeCls : inactive}`}
    >
      {children}
    </button>
  );
}

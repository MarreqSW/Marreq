import { NavLink, Outlet, useParams } from 'react-router-dom';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';

const tabs: { path: string; label: string }[] = [
  { path: 'categories', label: 'Categories' },
  { path: 'applicability', label: 'Applicability' },
  { path: 'requirement-statuses', label: 'Requirement statuses' },
  { path: 'verification-statuses', label: 'Verification statuses' },
  { path: 'custom-fields', label: 'Custom fields' },
  { path: 'verification-methods', label: 'Verification methods' },
];

export default function ProjectCatalogLayout() {
  const { projectId: projectIdParam } = useParams();
  const pid = Number(projectIdParam);
  const { dashboard } = useDashboard();
  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  return (
    <div>
      <StitchPageHeader
        projectName={projectName}
        section="Catalog"
        title="Project catalog"
        subtitle="Edit categories, applicability, statuses, custom fields, and verification methods for this project."
      />
      <nav className="flex flex-wrap gap-2 mb-8 border-b border-stitch-border pb-4">
        {tabs.map((t) => (
          <NavLink
            key={t.path}
            to={t.path}
            className={({ isActive }) =>
              `px-3 py-2 rounded-md text-xs font-bold uppercase tracking-wide transition-colors ${
                isActive
                  ? 'bg-stitch-accent text-stitch-canvas'
                  : 'text-stitch-muted hover:text-white hover:bg-white/[0.06]'
              }`
            }
          >
            {t.label}
          </NavLink>
        ))}
      </nav>
      <Outlet />
    </div>
  );
}

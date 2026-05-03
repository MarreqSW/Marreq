import { Link, useOutletContext } from 'react-router-dom';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';
import type { ProjectOutletContext } from '@/types/projectOutlet';

const blocks: { title: string; body: string }[] = [
  {
    title: 'Navigation',
    body:
      'Use the sidebar for Dashboard, Requirements, Verifications, Traceability, Matrix, Baselines, Reports, Settings, and Admin (administrators only). Project scope follows the project selected in the header.',
  },
  {
    title: 'Search',
    body:
      'The header search filters the Requirements table and Verifications list on their respective pages. Open a project first, then go to the list you want to filter.',
  },
  {
    title: 'Traceability',
    body:
      'The Graph view has two subtabs. Coverage shows requirement ↔ verification links (suspect links use coral styling). Hierarchy shows parent ↔ child links between requirements (solid blue) and between verifications (dashed green); a Requirements / Verifications / Both filter restricts the view and is remembered in the URL (?kind=).',
  },
  {
    title: 'Creating records',
    body:
      'Use Create Requirement or New verification from the header or list pages. New requirements need at least one verification method configured for the project.',
  },
  {
    title: 'Classic (legacy) UI',
    body:
      'Matrix and baselines have SPA pages with links to the classic HTML views for full parity. ReqIF import/export, bulk Excel import, server logs, and some admin tools remain classic-only; Reports includes links to legacy Excel/PDF exports.',
  },
];

export default function HelpPage() {
  const { projectId, basePath } = useOutletContext<ProjectOutletContext>();
  const pid = projectId;
  const { dashboard } = useDashboard();

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  return (
    <div>
      <StitchPageHeader
        projectName={projectName}
        section="Help"
        title="Help & reference"
        subtitle="Quick orientation for the RVM-style workspace."
      />

      <div className="space-y-4 max-w-3xl">
        {blocks.map((b) => (
          <div
            key={b.title}
            className="rounded-xl border border-stitch-border bg-stitch-surface p-5 shadow-stitch"
          >
            <h3 className="text-sm font-bold text-stitch-accent uppercase tracking-wide mb-2">
              {b.title}
            </h3>
            <p className="text-sm text-stitch-muted leading-relaxed">{b.body}</p>
          </div>
        ))}

        <div className="rounded-xl border border-stitch-border bg-stitch-elevated p-5">
          <h3 className="text-sm font-bold text-stitch-fg uppercase tracking-wide mb-3">Shortcuts</h3>
          <ul className="text-sm text-stitch-muted space-y-2">
            <li>
              <Link to={`${basePath}/dashboard`} className="text-stitch-accent font-semibold hover:underline">
                Dashboard
              </Link>{' '}
              — project KPIs
            </li>
            <li>
              <Link to={`${basePath}/reports`} className="text-stitch-accent font-semibold hover:underline">
                Reports
              </Link>{' '}
              — coverage gaps
            </li>
            <li>
              <Link to={`${basePath}/settings`} className="text-stitch-accent font-semibold hover:underline">
                Settings
              </Link>{' '}
              — permissions & fields
            </li>
          </ul>
        </div>
      </div>
    </div>
  );
}

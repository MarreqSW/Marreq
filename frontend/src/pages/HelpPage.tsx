import { Link, useParams } from 'react-router-dom';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';

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
      'The Graph view shows requirement ↔ verification links. Suspect links appear in Reports and use coral styling on the graph when applicable.',
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
  const { projectId: projectIdParam } = useParams();
  const pid = Number(projectIdParam);
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
              <Link to={`/p/${pid}/dashboard`} className="text-stitch-accent font-semibold hover:underline">
                Dashboard
              </Link>{' '}
              — project KPIs
            </li>
            <li>
              <Link to={`/p/${pid}/reports`} className="text-stitch-accent font-semibold hover:underline">
                Reports
              </Link>{' '}
              — coverage gaps
            </li>
            <li>
              <Link to={`/p/${pid}/settings`} className="text-stitch-accent font-semibold hover:underline">
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

import { useEffect, useState } from 'react';
import { Link, useOutletContext } from 'react-router-dom';
import { getBuildInfo } from '@/api/client';
import type { BuildInfo } from '@/api/types';
import { useDashboard } from '@/context/DashboardContext';
import StitchPageHeader from '@/components/StitchPageHeader';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import {
  getFrontendBuildConstants,
  isVersionInInclusiveRange,
} from '@/utils/semverRange';

const blocks: { title: string; body: string }[] = [
  {
    title: 'Navigation',
    body:
      'Use the sidebar for Dashboard, Requirements, Verifications, Traceability, Matrix, Baselines, Reports, Import, Settings, and Admin (administrators only). Project scope follows the project selected in the header.',
  },
  {
    title: 'Search',
    body:
      'The header search filters the Requirements table and Verifications list on their respective pages. Open a project first, then go to the list you want to filter.',
  },
  {
    title: 'Traceability',
    body:
      'The Graph view has two subtabs. Coverage shows requirement ↔ verification links (suspect links use coral styling). Hierarchy shows parent ↔ child links between requirements (solid blue) and between verifications (dashed green); a Requirements / Verifications / Both filter restricts the view and is remembered in the URL (?kind=). Double-click a node to open the requirement or verification view page.',
  },
  {
    title: 'Creating records',
    body:
      'Use Create Requirement or New verification from the header or list pages. New requirements need at least one verification method configured for the project. Import Excel/CSV or ReqIF from Import in the sidebar (or the create menu).',
  },
  {
    title: 'Classic (legacy) UI',
    body:
      'Some admin tools (server logs, database backup) are not in this workspace yet. Reports includes Excel, PDF, and ReqIF downloads served by the API. Import ReqIF from the Import page.',
  },
];

export default function HelpPage() {
  const { projectId, basePath } = useOutletContext<ProjectOutletContext>();
  const pid = projectId;
  const { dashboard } = useDashboard();
  const ui = getFrontendBuildConstants();
  const [build, setBuild] = useState<BuildInfo | null>(null);
  const [buildError, setBuildError] = useState(false);

  useEffect(() => {
    let alive = true;
    getBuildInfo()
      .then((info) => {
        if (alive) setBuild(info);
      })
      .catch(() => {
        if (alive) setBuildError(true);
      });
    return () => {
      alive = false;
    };
  }, []);

  const projectName =
    dashboard?.projects?.find((p) => p.id === pid)?.name ?? 'Project';

  const compatible =
    build != null &&
    isVersionInInclusiveRange(
      ui.version,
      build.frontend_compatibility.min_version,
      build.frontend_compatibility.max_version,
    ) &&
    isVersionInInclusiveRange(
      build.backend_version,
      ui.requiresBackendMin,
      ui.requiresBackendMax,
    );

  return (
    <div>
      <StitchPageHeader
        projectName={projectName}
        section="Help"
        title="Help & reference"
        subtitle="Quick orientation for the RVM-style workspace."
      />

      <div className="space-y-4 max-w-3xl">
        <div
          className="rounded-xl border border-stitch-border bg-stitch-surface p-5 shadow-stitch"
          data-testid="help-build-info"
        >
          <h3 className="text-sm font-bold text-stitch-accent uppercase tracking-wide mb-2">
            Versions
          </h3>
          <dl className="text-sm text-stitch-muted space-y-1.5">
            <div className="flex gap-2">
              <dt className="font-semibold text-stitch-fg w-36 shrink-0">UI</dt>
              <dd>{ui.version}</dd>
            </div>
            <div className="flex gap-2">
              <dt className="font-semibold text-stitch-fg w-36 shrink-0">API</dt>
              <dd>
                {buildError
                  ? 'unavailable'
                  : build
                    ? build.backend_version
                    : '…'}
              </dd>
            </div>
            <div className="flex gap-2">
              <dt className="font-semibold text-stitch-fg w-36 shrink-0">Deployment</dt>
              <dd>
                {buildError
                  ? 'unavailable'
                  : build
                    ? build.deployment_mode
                    : '…'}
              </dd>
            </div>
            <div className="flex gap-2">
              <dt className="font-semibold text-stitch-fg w-36 shrink-0">Compatible</dt>
              <dd data-testid="help-compatible">
                {buildError ? 'unknown' : build == null ? '…' : compatible ? 'yes' : 'no'}
              </dd>
            </div>
          </dl>
        </div>

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
              <Link to={`${basePath}/import`} className="text-stitch-accent font-semibold hover:underline">
                Import
              </Link>{' '}
              — Excel / CSV
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

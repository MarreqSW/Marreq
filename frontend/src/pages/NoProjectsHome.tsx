import { Link, useNavigate } from 'react-router-dom';
import { useDashboard } from '@/context/DashboardContext';

type NoProjectsHomeProps = {
  isAdmin: boolean;
  /** Shown in the greeting line when present. */
  displayName: string;
};

/**
 * Shown on `/` when the session has no accessible projects (empty membership list for non-admins, or no projects in the instance for admins).
 */
export default function NoProjectsHome({ isAdmin, displayName }: NoProjectsHomeProps) {
  const { logout } = useDashboard();
  const navigate = useNavigate();

  async function handleSignOut() {
    await logout();
    navigate('/login', { replace: true });
  }

  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-6 bg-stitch-canvas">
      <div className="w-full max-w-md rounded-xl border border-stitch-border bg-stitch-surface p-8 shadow-stitch">
        <p className="text-xs font-semibold uppercase tracking-wider text-stitch-muted mb-1">
          Marreq
        </p>
        <h1 className="text-2xl font-bold text-stitch-fg mb-2">No projects available</h1>
        <p className="text-stitch-muted text-sm mb-6">
          {displayName ? `Signed in as ${displayName}. ` : null}
          {isAdmin
            ? 'There is no project to open yet. Create one to get started.'
            : 'You are not a member of any project. Ask a site administrator to add you to a project.'}
        </p>

        <div className="flex flex-col gap-3">
          {isAdmin ? (
            <a
              href="/new_project"
              className="w-full text-center rounded-lg bg-gradient-to-br from-[#000666] to-[#1a237e] text-white font-semibold py-2.5 text-sm hover:opacity-95"
            >
              Create project
            </a>
          ) : null}
          <Link
            to="/groups"
            className="w-full text-center rounded-lg border border-stitch-border bg-stitch-elevated text-stitch-fg font-semibold py-2.5 text-sm hover:bg-stitch-canvas"
          >
            Browse groups
          </Link>
          <button
            type="button"
            onClick={() => {
              void handleSignOut();
            }}
            className="w-full text-center rounded-lg border border-transparent text-stitch-muted text-sm py-2 hover:text-stitch-fg"
          >
            Sign out
          </button>
        </div>
      </div>
    </div>
  );
}

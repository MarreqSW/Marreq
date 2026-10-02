import { useEffect, useState } from 'react';
import { Link, Outlet, useLocation } from 'react-router-dom';
import { getDeploymentInfo } from '@/api/client';
import SubNav, { type SubNavItem } from '@/components/SubNav';
import { ADMIN_BASE } from './adminArea';

const FROM_KEY = 'marreq.admin.from';

/** Where "Back to project" goes: the project the area was opened from, else home. */
function useReturnPath(): string {
  const state = useLocation().state as { from?: string } | null;
  const [from] = useState<string>(() => {
    const given = state?.from;
    try {
      if (given) sessionStorage.setItem(FROM_KEY, given);
      return given ?? sessionStorage.getItem(FROM_KEY) ?? '/';
    } catch {
      return given ?? '/';
    }
  });
  return from;
}

/**
 * Instance administration, outside any project (issue #346): users, system
 * logs, log analytics and (self-hosted only) database backup. Each page still
 * checks access itself.
 */
export default function AdminLayout() {
  const back = useReturnPath();
  const [backupAllowed, setBackupAllowed] = useState(true);

  useEffect(() => {
    let cancelled = false;
    Promise.resolve(getDeploymentInfo())
      .then((d) => {
        if (!cancelled) setBackupAllowed(d?.allows_database_backup !== false);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  const sections: SubNavItem[] = [
    { to: ADMIN_BASE, label: 'Users', icon: 'group', end: true },
    { to: `${ADMIN_BASE}/logs`, label: 'System logs', icon: 'history', end: true },
    { to: `${ADMIN_BASE}/logs/analytics`, label: 'Log analytics', icon: 'monitoring' },
    { to: `${ADMIN_BASE}/backup`, label: 'Backup', icon: 'backup', hidden: !backupAllowed },
  ];

  return (
    <div className="stitch-app min-h-screen bg-stitch-canvas text-stitch-fg text-stitch font-sans antialiased">
      <header className="sticky top-0 z-40 flex items-center justify-between gap-4 px-6 py-3 border-b border-stitch-border bg-stitch-surface/95 backdrop-blur-md shadow-stitch">
        <div className="flex items-center gap-3 min-w-0">
          <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center shrink-0">
            <span className="material-symbols-outlined text-white text-sm">architecture</span>
          </div>
          <div className="min-w-0">
            <p className="text-stitch-accent text-xs uppercase tracking-wider font-bold">Marreq</p>
            <h1 className="text-lg font-bold tracking-tight text-stitch-fg truncate">Administration</h1>
          </div>
        </div>
        <Link
          to={back}
          className="inline-flex items-center gap-1 text-xs font-bold uppercase tracking-wider text-stitch-accent border border-stitch-border rounded-md px-3 py-2 hover:bg-stitch-higher"
        >
          <span className="material-symbols-outlined text-base" aria-hidden>
            arrow_back
          </span>
          Back to project
        </Link>
      </header>
      <main className="p-6 md:p-8 pb-16 max-w-7xl mx-auto">
        <SubNav items={sections} ariaLabel="Administration sections" />
        <Outlet />
      </main>
    </div>
  );
}

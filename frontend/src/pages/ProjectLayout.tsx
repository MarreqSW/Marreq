import { useEffect, useMemo, useRef, useState } from 'react';
import { Link, NavLink, Outlet, useLocation, useNavigate, useParams } from 'react-router-dom';
import { useDashboard } from '@/context/DashboardContext';
import { useTheme, type ThemePreference } from '@/context/ThemeContext';
import { getProjectFromPath } from '@/api/client';
import type { User } from '@/api/types';
import NotificationPanel from '@/components/NotificationPanel';
import type { ProjectOutletContext } from '@/types/projectOutlet';
import { parseUser } from '@/utils/parseUser';
import { useBuildInfo } from '@/hooks/useBuildInfo';
import { getFrontendBuildConstants, shortSha } from '@/utils/semverRange';

const APP_VERSION = 'v0.1.0';

/** Daily work. Configuration lives under Project settings, instance admin under /admin (issue #346). */
const NAV_ITEMS: { path: string; icon: string; label: string; end?: boolean }[] = [
  { path: 'dashboard', icon: 'dashboard', label: 'Dashboard', end: true },
  { path: 'requirements', icon: 'list_alt', label: 'Requirements' },
  { path: 'verifications', icon: 'verified', label: 'Verifications' },
  { path: 'traceability', icon: 'account_tree', label: 'Traceability' },
  { path: 'baselines', icon: 'history_edu', label: 'Baselines' },
  { path: 'reports', icon: 'description', label: 'Reports & exports' },
];

const SIDEBAR_WIDE_KEY = 'marreq.sidebar.wide';

function readSidebarWide(): boolean {
  try {
    return localStorage.getItem(SIDEBAR_WIDE_KEY) !== '0';
  } catch {
    return true;
  }
}

function saveSidebarWide(wide: boolean) {
  try {
    localStorage.setItem(SIDEBAR_WIDE_KEY, wide ? '1' : '0');
  } catch {
    // Private mode / blocked storage: the choice just isn't remembered.
  }
}

function userInitials(u: User): string {
  const n = u.name?.trim();
  if (n) {
    const parts = n.split(/\s+/).filter(Boolean);
    if (parts.length >= 2) {
      return (parts[0]!.slice(0, 1) + parts[parts.length - 1]!.slice(0, 1)).toUpperCase();
    }
    return n.slice(0, 2).toUpperCase();
  }
  return u.username.slice(0, 2).toUpperCase();
}

export default function ProjectLayout() {
  const { projectSlug } = useParams();
  const location = useLocation();
  const navigate = useNavigate();
  const { dashboard, setSelectedProjectId, refresh, logout } = useDashboard();
  const { preference, setPreference } = useTheme();
  const [sidebarWideSetting, setSidebarWideSetting] = useState(readSidebarWide);
  /** Off-canvas sidebar on narrow screens. */
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [projectMenuOpen, setProjectMenuOpen] = useState(false);
  const projectMenuRef = useRef<HTMLDivElement | null>(null);
  const ui = getFrontendBuildConstants();
  const { build } = useBuildInfo();
  const uiSha = shortSha(ui.gitSha);
  const apiSha = shortSha(build?.backend_git_sha);
  const [globalSearch, setGlobalSearch] = useState('');
  const [createMenuOpen, setCreateMenuOpen] = useState(false);
  const createMenuRef = useRef<HTMLDivElement | null>(null);
  const [userMenuOpen, setUserMenuOpen] = useState(false);
  const userMenuRef = useRef<HTMLDivElement | null>(null);
  const [resolvedPid, setResolvedPid] = useState<number | null>(null);

  const projects = dashboard?.projects ?? [];
  const basePath = projectSlug ? `/${projectSlug}` : '';

  const currentProject = projects.find((p) => p.slug === projectSlug);
  const pid = currentProject?.id ?? resolvedPid;

  useEffect(() => {
    if (currentProject || !projectSlug) return;
    getProjectFromPath(projectSlug)
      .then((p) => setResolvedPid(p.id))
      .catch(() => {
        if (projects.length > 0) {
          const fallback = projects[0]!;
          navigate(`${fallback.project_base_path}/dashboard`, { replace: true });
        }
      });
  }, [projectSlug, currentProject, projects, navigate]);

  const onVerificationsSection = /\/verifications(\/|$)/.test(location.pathname);

  const createMenuItems = useMemo(
    () =>
      onVerificationsSection
        ? [
            {
              to: `${basePath}/verifications/new`,
              label: 'Create verification',
              compact: 'Verification',
              icon: 'verified' as const,
            },
            {
              to: `${basePath}/requirements/new`,
              label: 'Create requirement',
              compact: 'Requirement',
              icon: 'list_alt' as const,
            },
            {
              to: `${basePath}/settings/import`,
              label: 'Import',
              compact: 'Import',
              icon: 'upload_file' as const,
            },
          ]
        : [
            {
              to: `${basePath}/requirements/new`,
              label: 'Create requirement',
              compact: 'Requirement',
              icon: 'list_alt' as const,
            },
            {
              to: `${basePath}/verifications/new`,
              label: 'Create verification',
              compact: 'Verification',
              icon: 'verified' as const,
            },
            {
              to: `${basePath}/settings/import`,
              label: 'Import',
              compact: 'Import',
              icon: 'upload_file' as const,
            },
          ],
    [onVerificationsSection, basePath],
  );

  const primaryCreate = createMenuItems[0];

  useEffect(() => {
    if (!createMenuOpen && !userMenuOpen && !projectMenuOpen) return;
    const onDown = (e: MouseEvent) => {
      const target = e.target as Node;
      if (createMenuOpen && !createMenuRef.current?.contains(target)) {
        setCreateMenuOpen(false);
      }
      if (userMenuOpen && !userMenuRef.current?.contains(target)) {
        setUserMenuOpen(false);
      }
      if (projectMenuOpen && !projectMenuRef.current?.contains(target)) {
        setProjectMenuOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setCreateMenuOpen(false);
        setUserMenuOpen(false);
        setProjectMenuOpen(false);
      }
    };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [createMenuOpen, userMenuOpen, projectMenuOpen]);

  // The drawer closes when a link in it is followed.
  useEffect(() => {
    setDrawerOpen(false);
  }, [location.pathname]);

  useEffect(() => {
    if (!drawerOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setDrawerOpen(false);
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [drawerOpen]);

  const invalid = pid == null && projects.length > 0 && !currentProject;
  const user = useMemo(() => parseUser(dashboard?.user), [dashboard?.user]);

  if (pid == null) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-stitch-canvas text-stitch-muted text-sm">
        Loading project…
      </div>
    );
  }

  // In the mobile drawer the sidebar is always shown with labels.
  const sidebarWide = sidebarWideSetting || drawerOpen;
  const toggleSidebarWide = () => {
    setSidebarWideSetting((wide) => {
      saveSidebarWide(!wide);
      return !wide;
    });
  };

  const switchProject = (id: number) => {
    setProjectMenuOpen(false);
    setSelectedProjectId(id);
    void refresh();
    const selectedProject = projects.find((p) => p.id === id);
    if (selectedProject) {
      const subPath = location.pathname.startsWith(basePath)
        ? location.pathname.slice(basePath.length)
        : '/dashboard';
      navigate(`${selectedProject.project_base_path}${subPath || '/dashboard'}`);
    }
  };

  const outletContext: ProjectOutletContext = {
    projectId: pid,
    basePath,
    globalSearch,
    setGlobalSearch,
  };

  const sideLink = (opts: {
    to: string;
    icon: string;
    label: string;
    end?: boolean;
  }) => (
    <NavLink
      to={opts.to}
      end={opts.end}
      className={({ isActive }) =>
        `flex items-center gap-3 px-4 py-3 text-xs uppercase tracking-wider font-bold transition-all rounded-r-md ${
          isActive
            ? 'border-l-4 border-stitch-accent bg-stitch-elevated text-stitch-accent'
            : 'border-l-4 border-transparent text-stitch-muted hover:bg-stitch-elevated hover:text-stitch-fg'
        } ${!sidebarWide ? 'justify-center px-2' : ''}`
      }
      title={!sidebarWide ? opts.label : undefined}
      aria-label={opts.label}
    >
      <span className="material-symbols-outlined text-lg shrink-0" aria-hidden>
        {opts.icon}
      </span>
      {sidebarWide ? <span className="truncate">{opts.label}</span> : null}
    </NavLink>
  );

  return (
    <div className="stitch-app min-h-screen flex bg-stitch-canvas text-stitch-fg text-stitch font-sans antialiased">
      {drawerOpen ? (
        <div
          className="fixed inset-0 z-40 bg-black/40 md:hidden"
          aria-hidden
          data-testid="sidebar-backdrop"
          onClick={() => setDrawerOpen(false)}
        />
      ) : null}
      {/* SideNavBar: sticky column from md up, off-canvas drawer below */}
      <aside
        id="project-sidebar"
        aria-label="Project navigation"
        data-drawer-open={drawerOpen ? 'true' : 'false'}
        className={`flex flex-col h-screen top-0 shrink-0 border-r border-stitch-border bg-stitch-surface z-50 transition-[width,transform] duration-200 fixed left-0 md:sticky md:translate-x-0 ${
          drawerOpen ? 'translate-x-0 shadow-2xl' : '-translate-x-full'
        } ${sidebarWide ? 'w-64' : 'w-18'}`}
      >
        <div className={`px-6 py-8 flex-1 min-h-0 overflow-y-auto ${!sidebarWide ? 'px-3' : ''}`}>
          <div className={`flex items-center gap-3 mb-8 ${!sidebarWide ? 'flex-col' : ''}`}>
            <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center shrink-0">
              <span className="material-symbols-outlined text-white text-sm">architecture</span>
            </div>
            {sidebarWide ? (
              <div className="min-w-0">
                <h2 className="text-stitch-accent font-sans text-xs uppercase tracking-wider font-bold">
                  Marreq
                </h2>
                <p className="text-[10px] text-stitch-muted font-mono">{APP_VERSION}</p>
              </div>
            ) : null}
          </div>
          <nav className="flex flex-col space-y-1">
            {NAV_ITEMS.map((item) => (
              <div key={item.path}>
                {sideLink({
                  to: `${basePath}/${item.path}`,
                  icon: item.icon,
                  label: item.label,
                  end: item.end,
                })}
              </div>
            ))}
          </nav>
        </div>
        <div className={`mt-auto px-6 py-6 space-y-2 border-t border-stitch-border ${!sidebarWide ? 'px-2' : ''}`}>
          {sideLink({
            to: `${basePath}/settings`,
            icon: 'settings',
            label: 'Project settings',
          })}
          {sideLink({
            to: `${basePath}/help`,
            icon: 'help',
            label: 'Help',
          })}
          <button
            type="button"
            onClick={toggleSidebarWide}
            className={`hidden md:block w-full py-2 bg-primary text-white text-[10px] uppercase font-bold tracking-widest rounded-md hover:opacity-90 transition-opacity ${
              !sidebarWide ? 'px-1' : ''
            }`}
            title={sidebarWide ? 'Collapse sidebar' : 'Expand sidebar'}
          >
            {sidebarWide ? 'Collapse' : '»'}
          </button>
          {sidebarWide ? (
            <p
              className="pt-2 text-center text-[10px] text-stitch-muted"
              data-testid="sidebar-version"
              title={`UI ${uiSha ?? 'unknown'} · API ${apiSha ?? 'unknown'}`}
            >
              UI {ui.version}
              {build ? ` · API ${build.backend_version}` : null}
            </p>
          ) : null}
        </div>
      </aside>

      <div className="flex-1 flex flex-col min-w-0 min-h-screen">
        {/* TopNavBar */}
        <header className="sticky top-0 z-40 flex flex-wrap items-center justify-between gap-4 w-full px-6 py-3 border-b border-stitch-border bg-stitch-surface/95 backdrop-blur-md shadow-stitch">
          <div className="flex items-center gap-4 md:gap-8 min-w-0 flex-1">
            <div className="flex items-center gap-3 min-w-0">
              <button
                type="button"
                className="md:hidden p-2 -ml-2 rounded-md text-stitch-muted hover:bg-stitch-elevated hover:text-stitch-fg"
                aria-label="Open navigation"
                aria-controls="project-sidebar"
                aria-expanded={drawerOpen}
                onClick={() => setDrawerOpen(true)}
              >
                <span className="material-symbols-outlined text-xl" aria-hidden>
                  menu
                </span>
              </button>
              <h1 className="text-lg md:text-xl font-bold tracking-tight text-stitch-fg truncate">
                {currentProject?.name ?? 'Project'}
              </h1>
              <div className="relative" ref={projectMenuRef}>
                <button
                  type="button"
                  aria-label="Switch project"
                  aria-haspopup="menu"
                  aria-expanded={projectMenuOpen}
                  onClick={() => setProjectMenuOpen((o) => !o)}
                  className="inline-flex items-center gap-1 border border-stitch-border rounded-md px-2 py-1.5 bg-stitch-elevated text-stitch-fg text-xs hover:bg-stitch-higher"
                >
                  <span className="material-symbols-outlined text-base" aria-hidden>
                    swap_horiz
                  </span>
                  <span className="hidden sm:inline">Projects</span>
                  <span className="material-symbols-outlined text-base" aria-hidden>
                    expand_more
                  </span>
                </button>
                {projectMenuOpen ? (
                  <div
                    role="menu"
                    aria-label="Projects"
                    className="absolute left-0 top-[calc(100%+6px)] min-w-[240px] max-h-[70vh] overflow-y-auto rounded-lg border border-stitch-border bg-stitch-surface shadow-stitch py-1 z-60"
                  >
                    {projects.map((p) => (
                      <button
                        key={p.id}
                        type="button"
                        role="menuitemradio"
                        aria-checked={p.id === pid}
                        onClick={() => switchProject(p.id)}
                        className={`w-full text-left flex items-center gap-2 px-3 py-2 text-sm hover:bg-stitch-elevated ${
                          p.id === pid ? 'font-bold text-stitch-accent' : 'text-stitch-fg'
                        }`}
                      >
                        <span className="material-symbols-outlined text-base" aria-hidden>
                          {p.id === pid ? 'check' : 'folder'}
                        </span>
                        <span className="truncate">{p.name}</span>
                      </button>
                    ))}
                    <div className="my-1 border-t border-stitch-border" role="separator" />
                    {(
                      [
                        ['/groups', 'workspaces', 'Groups'],
                        ['/projects/new', 'add_box', 'New project'],
                        ['/projects/import-bundle', 'upload_file', 'Import project bundle'],
                      ] as const
                    ).map(([to, icon, label]) => (
                      <Link
                        key={to}
                        role="menuitem"
                        to={to}
                        onClick={() => setProjectMenuOpen(false)}
                        className="flex items-center gap-2 px-3 py-2 text-sm font-semibold text-stitch-fg hover:bg-stitch-elevated"
                      >
                        <span className="material-symbols-outlined text-base text-stitch-muted" aria-hidden>
                          {icon}
                        </span>
                        {label}
                      </Link>
                    ))}
                  </div>
                ) : null}
              </div>
            </div>
            <div className="relative hidden md:block min-w-0 flex-1 max-w-md">
              <span className="absolute inset-y-0 left-3 flex items-center text-stitch-muted pointer-events-none">
                <span className="material-symbols-outlined text-sm">search</span>
              </span>
              <input
                type="search"
                value={globalSearch}
                onChange={(e) => setGlobalSearch(e.target.value)}
                placeholder="Global Search…"
                className="w-full pl-10 pr-4 py-1.5 bg-stitch-elevated border border-stitch-border rounded-md text-sm text-stitch-fg placeholder:text-stitch-muted focus:ring-1 focus:ring-stitch-accent focus:border-stitch-accent outline-hidden"
              />
            </div>
          </div>
          <div className="flex items-center gap-4 shrink-0">
            <div
              className="flex items-center rounded-lg border border-stitch-border p-0.5 gap-0.5 shrink-0"
              role="group"
              aria-label="Color scheme"
            >
              {(
                [
                  ['light', 'light_mode', 'Light theme'] as const,
                  ['dark', 'dark_mode', 'Dark theme'] as const,
                  ['system', 'routine', 'Match system'] as const,
                ] as const
              ).map(([pref, icon, title]) => (
                <button
                  key={pref}
                  type="button"
                  title={title}
                  aria-pressed={preference === pref}
                  onClick={() => setPreference(pref as ThemePreference)}
                  className={`p-1.5 rounded-md transition-colors ${
                    preference === pref
                      ? 'bg-stitch-elevated text-stitch-accent'
                      : 'text-stitch-muted hover:bg-stitch-elevated hover:text-stitch-fg'
                  }`}
                >
                  <span className="material-symbols-outlined text-lg">{icon}</span>
                </button>
              ))}
            </div>
            <div className="hidden sm:flex items-center gap-1 text-stitch-muted">
              <NotificationPanel />
            </div>
            <div className="relative inline-flex" ref={createMenuRef}>
              <div className="inline-flex rounded-md shadow-lg overflow-hidden">
                <Link
                  to={primaryCreate.to}
                  title={primaryCreate.label}
                  onClick={() => setCreateMenuOpen(false)}
                  className="bg-linear-to-br from-primary to-primary-container text-white pl-4 pr-3 py-2 text-sm font-semibold flex items-center gap-2 hover:opacity-95 active:scale-[0.99] transition-transform"
                >
                  <span className="material-symbols-outlined text-sm shrink-0">add</span>
                  <span className="hidden lg:inline whitespace-nowrap">{primaryCreate.label}</span>
                  <span className="hidden sm:inline lg:hidden whitespace-nowrap">
                    {primaryCreate.compact}
                  </span>
                </Link>
                <button
                  type="button"
                  title="More create options"
                  aria-expanded={createMenuOpen}
                  aria-haspopup="menu"
                  aria-label="Open create menu"
                  onClick={() => setCreateMenuOpen((o) => !o)}
                  className="bg-linear-to-br from-primary to-primary-container text-white px-2 py-2 border-l border-white/25 hover:opacity-95 flex items-center justify-center shrink-0"
                >
                  <span
                    className={`material-symbols-outlined text-xl transition-transform ${createMenuOpen ? 'rotate-180' : ''}`}
                    aria-hidden
                  >
                    expand_more
                  </span>
                </button>
              </div>
              {createMenuOpen ? (
                <div
                  role="menu"
                  className="absolute right-0 top-[calc(100%+6px)] min-w-[220px] rounded-lg border border-stitch-border bg-stitch-surface shadow-stitch py-1 z-60"
                >
                  {createMenuItems.map((item) => (
                    <Link
                      key={item.to}
                      role="menuitem"
                      to={item.to}
                      onClick={() => setCreateMenuOpen(false)}
                      className="flex items-center gap-2 px-3 py-2.5 text-sm font-semibold text-stitch-fg hover:bg-stitch-elevated transition-colors"
                    >
                      <span className="material-symbols-outlined text-stitch-accent text-lg">
                        {item.icon}
                      </span>
                      {item.label}
                    </Link>
                  ))}
                </div>
              ) : null}
            </div>
            <div className="relative" ref={userMenuRef}>
              <button
                type="button"
                className="w-8 h-8 rounded-full border-2 border-stitch-accent/50 bg-stitch-elevated flex items-center justify-center text-[10px] font-bold text-stitch-fg hover:opacity-90"
                title={user ? `${user.name} (${user.username})` : 'User'}
                aria-label="User menu"
                aria-expanded={userMenuOpen}
                aria-haspopup="menu"
                onClick={() => setUserMenuOpen((o) => !o)}
              >
                {user ? userInitials(user) : '?'}
              </button>
              {userMenuOpen ? (
                <div
                  role="menu"
                  className="absolute right-0 top-[calc(100%+6px)] min-w-[180px] rounded-lg border border-stitch-border bg-stitch-surface shadow-stitch py-1 z-60"
                >
                  <Link
                    role="menuitem"
                    to="/account"
                    onClick={() => setUserMenuOpen(false)}
                    className="flex items-center gap-2 px-3 py-2.5 text-sm font-semibold text-stitch-fg hover:bg-stitch-elevated transition-colors"
                  >
                    Account settings
                  </Link>
                  {user?.is_admin ? (
                    <Link
                      role="menuitem"
                      to="/admin"
                      state={{ from: basePath }}
                      onClick={() => setUserMenuOpen(false)}
                      className="flex items-center gap-2 px-3 py-2.5 text-sm font-semibold text-stitch-fg hover:bg-stitch-elevated transition-colors"
                    >
                      Administration
                    </Link>
                  ) : null}
                  <Link
                    role="menuitem"
                    to="/change-password"
                    onClick={() => setUserMenuOpen(false)}
                    className="flex items-center gap-2 px-3 py-2.5 text-sm font-semibold text-stitch-fg hover:bg-stitch-elevated transition-colors"
                  >
                    Change password
                  </Link>
                  <button
                    type="button"
                    role="menuitem"
                    onClick={() => {
                      setUserMenuOpen(false);
                      void logout().then(() => navigate('/login', { replace: true }));
                    }}
                    className="w-full text-left flex items-center gap-2 px-3 py-2.5 text-sm font-semibold text-stitch-muted hover:bg-stitch-elevated hover:text-stitch-fg transition-colors"
                  >
                    Sign out
                  </button>
                </div>
              ) : null}
            </div>
          </div>
        </header>

        {/* No overflow here: it would become the containing scrollport for sticky page action bars. */}
        <main className="flex-1 p-6 md:p-8 pb-16">
          <Outlet context={outletContext} />
        </main>

        <footer className="shrink-0 border-t border-stitch-border bg-stitch-surface px-4 py-2 flex flex-wrap justify-between items-center gap-2 font-mono text-[10px] tracking-tight text-stitch-muted">
          <div>© {new Date().getFullYear()} Marreq</div>
          <div className="flex gap-4 md:gap-6">
            <span className="hover:text-stitch-accent cursor-default">Privacy Policy</span>
            <span className="hover:text-stitch-accent cursor-default">Documentation</span>
            <span className="flex items-center gap-1">
              <span className="w-1.5 h-1.5 bg-emerald-500 rounded-full animate-pulse" />
              System Status
            </span>
          </div>
        </footer>
      </div>
    </div>
  );
}

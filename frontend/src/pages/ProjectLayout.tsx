import { useEffect, useMemo, useRef, useState } from 'react';
import { Link, NavLink, Outlet, useLocation, useNavigate, useParams } from 'react-router-dom';
import { useDashboard } from '@/context/DashboardContext';
import { useTheme, type ThemePreference } from '@/context/ThemeContext';
import type { User } from '@/api/types';
import type { ProjectOutletContext } from '@/types/projectOutlet';

const APP_VERSION = 'v0.1.0';

function parseUser(u: unknown): User | null {
  if (u && typeof u === 'object' && 'username' in u) {
    return u as User;
  }
  return null;
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
  const { projectId } = useParams();
  const location = useLocation();
  const pid = Number(projectId);
  const navigate = useNavigate();
  const { dashboard, setSelectedProjectId, refresh, logout } = useDashboard();
  const { preference, setPreference } = useTheme();
  const [sidebarWide, setSidebarWide] = useState(true);
  const [globalSearch, setGlobalSearch] = useState('');
  const [createMenuOpen, setCreateMenuOpen] = useState(false);
  const createMenuRef = useRef<HTMLDivElement | null>(null);

  const onVerificationsSection = /\/verifications(\/|$)/.test(location.pathname);

  const createMenuItems = useMemo(
    () =>
      onVerificationsSection
        ? [
            {
              to: `/p/${pid}/verifications/new`,
              label: 'Create verification',
              compact: 'Verification',
              icon: 'verified' as const,
            },
            {
              to: `/p/${pid}/requirements/new`,
              label: 'Create requirement',
              compact: 'Requirement',
              icon: 'list_alt' as const,
            },
          ]
        : [
            {
              to: `/p/${pid}/requirements/new`,
              label: 'Create requirement',
              compact: 'Requirement',
              icon: 'list_alt' as const,
            },
            {
              to: `/p/${pid}/verifications/new`,
              label: 'Create verification',
              compact: 'Verification',
              icon: 'verified' as const,
            },
          ],
    [onVerificationsSection, pid],
  );

  const primaryCreate = createMenuItems[0];

  useEffect(() => {
    if (!createMenuOpen) return;
    const onDown = (e: MouseEvent) => {
      if (createMenuRef.current?.contains(e.target as Node)) return;
      setCreateMenuOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setCreateMenuOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [createMenuOpen]);

  const projects = dashboard?.projects ?? [];
  const invalid =
    Number.isNaN(pid) || (projects.length > 0 && !projects.some((p) => p.id === pid));

  const currentProject = projects.find((p) => p.id === pid);
  const user = useMemo(() => parseUser(dashboard?.user), [dashboard?.user]);

  if (invalid && projects.length > 0) {
    const fallback = projects[0]!.id;
    navigate(`/p/${fallback}/dashboard`, { replace: true });
    return null;
  }

  const outletContext: ProjectOutletContext = {
    projectId: pid,
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
    >
      <span className="material-symbols-outlined text-lg shrink-0">{opts.icon}</span>
      {sidebarWide ? <span className="truncate">{opts.label}</span> : null}
    </NavLink>
  );

  return (
    <div className="stitch-app min-h-screen flex bg-stitch-canvas text-stitch-fg text-stitch font-sans antialiased">
      {/* SideNavBar — structure from Image 2.html, Stitch dark palette */}
      <aside
        className={`flex flex-col h-screen sticky top-0 shrink-0 border-r border-stitch-border bg-stitch-surface z-50 transition-[width] duration-200 ${
          sidebarWide ? 'w-64' : 'w-[4.5rem]'
        }`}
      >
        <div className={`px-6 py-8 flex-1 min-h-0 overflow-y-auto ${!sidebarWide ? 'px-3' : ''}`}>
          <div className={`flex items-center gap-3 mb-8 ${!sidebarWide ? 'flex-col' : ''}`}>
            <div className="w-8 h-8 bg-[#000666] rounded-lg flex items-center justify-center shrink-0">
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
            {sideLink({
              to: `/p/${pid}/dashboard`,
              icon: 'dashboard',
              label: 'Dashboard',
              end: true,
            })}
            {sideLink({
              to: `/p/${pid}/requirements`,
              icon: 'list_alt',
              label: 'Requirements',
            })}
            {sideLink({
              to: `/p/${pid}/verifications`,
              icon: 'verified',
              label: 'Verification',
            })}
            {sideLink({
              to: `/p/${pid}/traceability`,
              icon: 'account_tree',
              label: 'Traceability',
            })}
            {sideLink({
              to: `/p/${pid}/matrix`,
              icon: 'grid_on',
              label: 'Matrix',
            })}
            {sideLink({
              to: `/p/${pid}/baselines`,
              icon: 'history_edu',
              label: 'Baselines',
            })}
            {sideLink({
              to: `/p/${pid}/reports`,
              icon: 'description',
              label: 'Reports',
            })}
            {sideLink({
              to: `/p/${pid}/catalog`,
              icon: 'tune',
              label: 'Catalog',
            })}
            {sideLink({
              to: `/p/${pid}/admin`,
              icon: 'admin_panel_settings',
              label: 'Admin',
            })}
          </nav>
        </div>
        <div className={`mt-auto px-6 py-6 space-y-2 border-t border-stitch-border ${!sidebarWide ? 'px-2' : ''}`}>
          {sideLink({
            to: `/p/${pid}/settings`,
            icon: 'settings',
            label: 'Settings',
          })}
          {sideLink({
            to: `/p/${pid}/help`,
            icon: 'help',
            label: 'Help',
          })}
          <button
            type="button"
            onClick={() => setSidebarWide((w) => !w)}
            className={`mt-4 w-full py-2 bg-[#000666] text-white text-[10px] uppercase font-bold tracking-widest rounded-md hover:opacity-90 transition-opacity ${
              !sidebarWide ? 'px-1' : ''
            }`}
            title={sidebarWide ? 'Collapse sidebar' : 'Expand sidebar'}
          >
            {sidebarWide ? 'Collapse' : '»'}
          </button>
        </div>
      </aside>

      <div className="flex-1 flex flex-col min-w-0 min-h-screen">
        {/* TopNavBar */}
        <header className="sticky top-0 z-40 flex flex-wrap items-center justify-between gap-4 w-full px-6 py-3 border-b border-stitch-border bg-stitch-surface/95 backdrop-blur-md shadow-stitch">
          <div className="flex items-center gap-4 md:gap-8 min-w-0 flex-1">
            <div className="flex items-center gap-3 min-w-0">
              <h1 className="text-lg md:text-xl font-bold tracking-tight text-stitch-fg truncate">
                {currentProject?.name ?? 'Project'}
              </h1>
              <select
                className="text-stitch max-w-[160px] sm:max-w-[220px] border border-stitch-border rounded-md px-2 py-1.5 bg-stitch-elevated text-stitch-fg text-xs focus:outline-none focus:ring-1 focus:ring-stitch-accent/50"
                value={Number.isFinite(pid) ? pid : ''}
                onChange={(e) => {
                  const id = Number(e.target.value);
                  setSelectedProjectId(id);
                  void refresh();
                  const sub = location.pathname.replace(/^\/p\/\d+/, `/p/${id}`);
                  navigate(sub || `/p/${id}/requirements`);
                }}
              >
                {projects.map((p) => (
                  <option key={p.id} value={p.id} className="bg-stitch-surface text-stitch-fg">
                    {p.name}
                  </option>
                ))}
              </select>
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
                className="w-full pl-10 pr-4 py-1.5 bg-stitch-elevated border border-stitch-border rounded-md text-sm text-stitch-fg placeholder:text-stitch-muted focus:ring-1 focus:ring-stitch-accent focus:border-stitch-accent outline-none"
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
              <button
                type="button"
                className="hover:bg-stitch-elevated p-2 rounded-full transition-colors"
                title="Notifications"
              >
                <span className="material-symbols-outlined text-xl">notifications</span>
              </button>
              <Link
                to={`/p/${pid}/settings`}
                className="hover:bg-stitch-elevated p-2 rounded-full transition-colors text-stitch-muted"
                title="Settings"
              >
                <span className="material-symbols-outlined text-xl">settings</span>
              </Link>
            </div>
            <div className="relative inline-flex" ref={createMenuRef}>
              <div className="inline-flex rounded-md shadow-lg overflow-hidden">
                <Link
                  to={primaryCreate.to}
                  title={primaryCreate.label}
                  onClick={() => setCreateMenuOpen(false)}
                  className="bg-gradient-to-br from-[#000666] to-[#1a237e] text-white pl-4 pr-3 py-2 text-sm font-semibold flex items-center gap-2 hover:opacity-95 active:scale-[0.99] transition-transform"
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
                  className="bg-gradient-to-br from-[#000666] to-[#1a237e] text-white px-2 py-2 border-l border-white/25 hover:opacity-95 flex items-center justify-center shrink-0"
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
                  className="absolute right-0 top-[calc(100%+6px)] min-w-[220px] rounded-lg border border-stitch-border bg-stitch-surface shadow-stitch py-1 z-[60]"
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
            <div
              className="w-8 h-8 rounded-full border-2 border-stitch-accent/50 bg-stitch-elevated flex items-center justify-center text-[10px] font-bold text-stitch-fg"
              title={user ? `${user.name} (${user.username})` : 'User'}
            >
              {user ? userInitials(user) : '?'}
            </div>
            <button
              type="button"
              onClick={() => void logout().then(() => navigate('/login', { replace: true }))}
              className="text-xs text-stitch-muted hover:text-stitch-fg transition-colors hidden sm:block"
            >
              Sign out
            </button>
          </div>
        </header>

        <main className="flex-1 overflow-auto p-6 md:p-8 pb-16">
          <Outlet context={outletContext} />
        </main>

        <footer className="shrink-0 border-t border-stitch-border bg-stitch-surface px-4 py-2 flex flex-wrap justify-between items-center gap-2 font-mono text-[10px] tracking-tight text-stitch-muted">
          <div>© {new Date().getFullYear()} RVM Systems | Marreq</div>
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

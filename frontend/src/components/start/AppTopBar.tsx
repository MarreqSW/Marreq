import { useEffect, useMemo, useRef, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import NotificationPanel from '@/components/NotificationPanel';
import { useDashboard } from '@/context/DashboardContext';
import { useTheme, type ThemePreference } from '@/context/ThemeContext';
import { parseUser } from '@/utils/parseUser';

const THEMES = [
  ['light', 'light_mode', 'Light theme'],
  ['dark', 'dark_mode', 'Dark theme'],
  ['system', 'routine', 'Match system'],
] as const;

/** Top bar for pages outside a project, such as the start screen (issue #387). */
export default function AppTopBar() {
  const { dashboard, logout } = useDashboard();
  const { preference, setPreference } = useTheme();
  const navigate = useNavigate();
  const user = useMemo(() => parseUser(dashboard?.user), [dashboard?.user]);
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!menuOpen) return;
    const onDown = (e: MouseEvent) => {
      if (!menuRef.current?.contains(e.target as Node)) setMenuOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setMenuOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [menuOpen]);

  const initials = user
    ? (user.name || user.username)
        .split(/\s+/)
        .filter(Boolean)
        .slice(0, 2)
        .map((w) => w[0]!.toUpperCase())
        .join('')
    : '?';
  const itemClass =
    'flex w-full items-center gap-2 px-3 py-2.5 text-sm font-semibold text-stitch-fg hover:bg-stitch-elevated transition-colors';

  return (
    <header className="h-14 shrink-0 border-b border-stitch-border bg-stitch-surface px-4 sm:px-6 flex items-center justify-between">
      <Link to="/" className="flex items-center gap-3" aria-label="Marreq start screen">
        <span className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
          <span className="material-symbols-outlined text-white text-sm" aria-hidden>
            architecture
          </span>
        </span>
        <span className="text-stitch-accent text-xs uppercase tracking-wider font-bold">Marreq</span>
      </Link>
      <div className="flex items-center gap-4 text-stitch-muted">
        <div
          className="hidden sm:flex items-center rounded-lg border border-stitch-border p-0.5 gap-0.5"
          role="group"
          aria-label="Color scheme"
        >
          {THEMES.map(([pref, icon, title]) => (
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
        <NotificationPanel />
        <div className="relative" ref={menuRef}>
          <button
            type="button"
            aria-label="User menu"
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            title={user ? `${user.name} (${user.username})` : 'User'}
            onClick={() => setMenuOpen((o) => !o)}
            className="w-8 h-8 rounded-full border-2 border-stitch-accent/50 bg-stitch-elevated flex items-center justify-center text-[10px] font-bold text-stitch-fg hover:opacity-90"
          >
            {initials}
          </button>
          {menuOpen ? (
            <div
              role="menu"
              className="absolute right-0 top-[calc(100%+6px)] min-w-[180px] rounded-lg border border-stitch-border bg-stitch-surface shadow-stitch py-1 z-60"
            >
              <Link role="menuitem" to="/account" className={itemClass}>
                Account settings
              </Link>
              {user?.is_admin ? (
                <Link role="menuitem" to="/admin" className={itemClass}>
                  Administration
                </Link>
              ) : null}
              <Link role="menuitem" to="/change-password" className={itemClass}>
                Change password
              </Link>
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  setMenuOpen(false);
                  void logout().then(() => navigate('/login', { replace: true }));
                }}
                className={`${itemClass} text-stitch-muted text-left`}
              >
                Sign out
              </button>
            </div>
          ) : null}
        </div>
      </div>
    </header>
  );
}

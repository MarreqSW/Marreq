import type { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import { useTheme, type ThemePreference } from '@/context/ThemeContext';

type AuthLayoutProps = {
  title: string;
  subtitle: string;
  children: ReactNode;
  footer?: ReactNode;
};

export default function AuthLayout({
  title,
  subtitle,
  children,
  footer,
}: AuthLayoutProps) {
  const { preference, setPreference } = useTheme();

  return (
    <div className="min-h-screen flex flex-col items-center justify-center p-6 bg-stitch-canvas relative">
      <div
        className="absolute top-4 right-4 flex items-center rounded-lg border border-stitch-border p-0.5 gap-0.5"
        role="group"
        aria-label="Color scheme"
      >
        {(
          [
            ['light', 'light_mode', 'Light'] as const,
            ['dark', 'dark_mode', 'Dark'] as const,
            ['system', 'routine', 'Auto'] as const,
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
      <div className="w-full max-w-md rounded-xl border border-stitch-border bg-stitch-surface p-8 shadow-stitch">
        <h1 className="text-2xl font-bold text-stitch-fg mb-1">{title}</h1>
        <p className="text-stitch-muted text-sm mb-6">{subtitle}</p>
        {children}
        {footer && <div className="mt-6">{footer}</div>}
      </div>
      <p className="mt-8 text-xs text-stitch-muted">
        <Link to="/" className="hover:text-stitch-accent">
          © Marreq
        </Link>
      </p>
    </div>
  );
}

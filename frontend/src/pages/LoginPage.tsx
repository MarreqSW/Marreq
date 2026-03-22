import { FormEvent, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { getCsrfToken, loginJson } from '@/api/client';
import { useTheme, type ThemePreference } from '@/context/ThemeContext';

export default function LoginPage() {
  const navigate = useNavigate();
  const { preference, setPreference } = useTheme();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      const csrf = await getCsrfToken();
      await loginJson(username, password, csrf);
      navigate('/', { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Login failed');
    } finally {
      setSubmitting(false);
    }
  }

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
        <h1 className="text-2xl font-bold text-stitch-fg mb-1">Welcome to Marreq</h1>
        <p className="text-stitch-muted text-sm mb-6">Sign in to continue</p>

        <form onSubmit={onSubmit} className="space-y-4">
          {error && (
            <div className="rounded-lg bg-red-500/10 border border-red-500/25 px-3 py-2 text-sm text-red-800 dark:text-red-200">
              {error}
            </div>
          )}
          <div>
            <label
              htmlFor="username"
              className="block text-xs font-semibold text-stitch-muted uppercase mb-1"
            >
              Username
            </label>
            <input
              id="username"
              autoComplete="username"
              className="w-full rounded-lg border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg focus:outline-none focus:ring-2 focus:ring-stitch-accent/50"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              required
            />
          </div>
          <div>
            <label
              htmlFor="password"
              className="block text-xs font-semibold text-stitch-muted uppercase mb-1"
            >
              Password
            </label>
            <input
              id="password"
              type="password"
              autoComplete="current-password"
              className="w-full rounded-lg border border-stitch-border bg-stitch-elevated px-3 py-2 text-sm text-stitch-fg focus:outline-none focus:ring-2 focus:ring-stitch-accent/50"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
            />
          </div>
          <button
            type="submit"
            disabled={submitting}
            className="w-full rounded-lg bg-gradient-to-br from-[#000666] to-[#1a237e] text-white font-semibold py-2.5 text-sm hover:opacity-95 disabled:opacity-60"
          >
            {submitting ? 'Signing in…' : 'Sign in'}
          </button>
        </form>

        <p className="mt-6 text-center text-xs text-stitch-muted">
          Default: alice / ChangeMe123!
        </p>
      </div>
      <p className="mt-8 text-xs text-stitch-muted">© Marreq</p>
    </div>
  );
}

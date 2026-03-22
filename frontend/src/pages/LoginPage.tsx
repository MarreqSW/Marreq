import { FormEvent, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { getCsrfToken, loginJson } from '@/api/client';

export default function LoginPage() {
  const navigate = useNavigate();
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
    <div className="min-h-screen flex flex-col items-center justify-center p-6 bg-slate-100">
      <div className="w-full max-w-md rounded-xl border border-slate-200 bg-white p-8 shadow-sm">
        <h1 className="text-2xl font-bold text-slate-900 mb-1">Welcome to Marreq</h1>
        <p className="text-slate-600 text-sm mb-6">Sign in to continue</p>

        <form onSubmit={onSubmit} className="space-y-4">
          {error && (
            <div className="rounded-lg bg-rose-50 border border-rose-100 px-3 py-2 text-sm text-rose-800">
              {error}
            </div>
          )}
          <div>
            <label htmlFor="username" className="block text-xs font-semibold text-slate-500 uppercase mb-1">
              Username
            </label>
            <input
              id="username"
              autoComplete="username"
              className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              required
            />
          </div>
          <div>
            <label htmlFor="password" className="block text-xs font-semibold text-slate-500 uppercase mb-1">
              Password
            </label>
            <input
              id="password"
              type="password"
              autoComplete="current-password"
              className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
            />
          </div>
          <button
            type="submit"
            disabled={submitting}
            className="w-full rounded-lg bg-indigo-600 text-white font-semibold py-2.5 text-sm hover:bg-indigo-700 disabled:opacity-60"
          >
            {submitting ? 'Signing in…' : 'Sign in'}
          </button>
        </form>

        <p className="mt-6 text-center text-xs text-slate-500">
          Default: alice / ChangeMe123!
        </p>
      </div>
      <p className="mt-8 text-xs text-slate-400">© Marreq</p>
    </div>
  );
}

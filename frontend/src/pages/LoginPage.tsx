import { FormEvent, useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { getCsrfToken, getDeploymentInfo, loginJson } from '@/api/client';
import type { DeploymentInfo } from '@/api/types';
import AuthLayout from '@/components/AuthLayout';

export default function LoginPage() {
  const navigate = useNavigate();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [deployment, setDeployment] = useState<DeploymentInfo | null>(null);

  useEffect(() => {
    let alive = true;
    getDeploymentInfo()
      .then((info) => {
        if (alive) setDeployment(info);
      })
      .catch(() => {
        if (alive) setDeployment(null);
      });
    return () => {
      alive = false;
    };
  }, []);

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

  const showSelfService = deployment?.allows_self_registration === true;

  return (
    <AuthLayout
      title="Welcome to Marreq"
      subtitle="Sign in to continue"
      footer={
        <>
          {showSelfService ? (
            <div className="space-y-3 text-center text-sm">
              <Link to="/forgot-password" className="text-stitch-accent hover:underline">
                Forgot your password?
              </Link>
              <p className="text-stitch-muted">
                New to Marreq?{' '}
                <Link to="/register" className="text-stitch-accent hover:underline">
                  Create an account
                </Link>
              </p>
            </div>
          ) : (
            <p className="text-center text-xs text-stitch-muted">
              Default: alice / ChangeMe123!
            </p>
          )}
        </>
      }
    >
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
    </AuthLayout>
  );
}

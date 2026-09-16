import { useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { getAuthProviders, getCsrfToken, getDeploymentInfo, loginJson } from '@/api/client';
import type { AuthProviderDiscovery, DeploymentInfo } from '@/api/types';
import AuthLayout from '@/components/AuthLayout';
import { useFormSubmit } from '@/hooks/useFormSubmit';
import { getFrontendBuildConstants } from '@/utils/semverRange';

export default function LoginPage() {
  const navigate = useNavigate();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [deployment, setDeployment] = useState<DeploymentInfo | null>(null);
  const [providers, setProviders] = useState<AuthProviderDiscovery>({ password_enabled: true, external: [] });
  const uiVersion = getFrontendBuildConstants().version;

  const { error, submitting, onSubmit } = useFormSubmit(async () => {
    const csrf = await getCsrfToken();
    await loginJson(username, password, csrf);
    navigate('/', { replace: true });
  });

  useEffect(() => {
    let alive = true;
    getDeploymentInfo()
      .then((info) => { if (alive) setDeployment(info); })
      .catch(() => { if (alive) setDeployment(null); });
    getAuthProviders()
      .then((info) => { if (alive) setProviders(info); })
      .catch(() => { if (alive) setProviders({ password_enabled: true, external: [] }); });
    return () => { alive = false; };
  }, []);

  const showSelfService = deployment?.allows_self_registration === true;

  return (
    <AuthLayout
      title="Welcome to Marreq"
      subtitle="Sign in to continue"
      footer={
        <div className="space-y-3 text-center text-sm">
          {showSelfService && (
            <>
              <Link to="/forgot-password" className="text-stitch-accent hover:underline">
                Forgot your password?
              </Link>
              <p className="text-stitch-muted">
                New to Marreq?{' '}
                <Link to="/register" className="text-stitch-accent hover:underline">
                  Create an account
                </Link>
              </p>
            </>
          )}
          <p className="text-xs text-stitch-muted" data-testid="login-ui-version">
            UI {uiVersion}
          </p>
        </div>
      }
    >
        <div className="space-y-4">
          {error && (
            <div className="rounded-lg bg-red-500/10 border border-red-500/25 px-3 py-2 text-sm text-red-800 dark:text-red-200">
              {error}
            </div>
          )}
          {providers.external.map((provider) => (
            <a
              key={provider.id}
              href={`/api/auth/external/${encodeURIComponent(provider.id)}/start`}
              className="block w-full rounded-lg border border-stitch-border bg-stitch-elevated px-3 py-2.5 text-center text-sm font-semibold text-stitch-fg hover:bg-stitch-muted/10"
            >
              Continue with {provider.display_name}
            </a>
          ))}
          {providers.external.length > 0 && providers.password_enabled && (
            <div className="flex items-center gap-3 text-xs text-stitch-muted" aria-label="or">
              <span className="h-px flex-1 bg-stitch-border" />or<span className="h-px flex-1 bg-stitch-border" />
            </div>
          )}
          {providers.password_enabled && <form onSubmit={onSubmit} className="space-y-4">
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
          </form>}
        </div>
    </AuthLayout>
  );
}

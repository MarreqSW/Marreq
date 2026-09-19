import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { disconnectIdentity, getAuthProviders, getConnectedApplications, getConnectedIdentities, getCsrfToken, revokeConnectedApplication, startIdentityLink } from '@/api/client';
import type { AuthProviderDiscovery, ConnectedApplication, ConnectedIdentities } from '@/api/types';
import AuthLayout from '@/components/AuthLayout';
import { useDashboard } from '@/context/DashboardContext';

export default function AccountPage() {
  const { csrfToken } = useDashboard();
  const [providers, setProviders] = useState<AuthProviderDiscovery>({ password_enabled: true, external: [] });
  const [accounts, setAccounts] = useState<ConnectedIdentities | null>(null);
  const [applications, setApplications] = useState<ConnectedApplication[]>([]);
  const [error, setError] = useState<string | null>(null);

  const reload = async () => {
    const [available, connected, delegated] = await Promise.all([getAuthProviders(), getConnectedIdentities(), getConnectedApplications()]);
    setProviders(available); setAccounts(connected); setApplications(delegated);
  };
  useEffect(() => { void reload().catch((reason) => setError(reason instanceof Error ? reason.message : 'Failed to load account')); }, []);

  const token = async () => csrfToken ?? getCsrfToken();
  const connect = async (provider: string) => {
    try { window.location.assign(await startIdentityLink(provider, await token())); } catch (reason) { setError(reason instanceof Error ? reason.message : 'Failed to connect account'); }
  };
  const disconnect = async (id: number) => {
    try { await disconnectIdentity(id, await token()); await reload(); } catch (reason) { setError(reason instanceof Error ? reason.message : 'Failed to disconnect account'); }
  };
  const revoke = async (id: number) => {
    try { await revokeConnectedApplication(id, await token()); await reload(); } catch (reason) { setError(reason instanceof Error ? reason.message : 'Failed to revoke application'); }
  };

  return <AuthLayout title="Account settings" subtitle="Manage the ways you sign in to Marreq" footer={<Link to="/" className="text-stitch-accent hover:underline">Back to home</Link>}>
    <div className="space-y-4">
      {error && <div className="rounded-lg bg-red-500/10 border border-red-500/25 px-3 py-2 text-sm text-red-800 dark:text-red-200">{error}</div>}
      <h2 className="text-sm font-semibold text-stitch-fg">Connected accounts</h2>
      {providers.external.map((provider) => {
        const connected = accounts?.identities.find((identity) => identity.provider === provider.id);
        return <div key={provider.id} className="flex items-center justify-between rounded-lg border border-stitch-border px-3 py-3">
          <div><div className="font-semibold text-stitch-fg">{provider.display_name}</div><div className="text-xs text-stitch-muted">{connected ? 'Connected' : 'Not connected'}</div></div>
          {connected ? <button type="button" onClick={() => void disconnect(connected.id)} className="text-sm text-red-700 dark:text-red-300 hover:underline">Disconnect</button>
            : <button type="button" onClick={() => void connect(provider.id)} className="text-sm text-stitch-accent hover:underline">Connect</button>}
        </div>;
      })}
      {accounts?.password_configured && <Link to="/change-password" className="block text-sm text-stitch-accent hover:underline">Change password</Link>}
      <h2 className="pt-3 text-sm font-semibold text-stitch-fg">Connected applications</h2>
      {applications.length === 0 && <p className="text-sm text-stitch-muted">No applications are connected.</p>}
      {applications.map((application) => <div key={application.id} className="rounded-lg border border-stitch-border px-3 py-3">
        <div className="flex items-start justify-between gap-3"><div><div className="font-semibold text-stitch-fg">{application.application_name}</div><div className="mt-1 text-xs text-stitch-muted">Authorized {new Date(application.created_at).toLocaleString()}</div></div><button type="button" onClick={() => void revoke(application.id)} className="text-sm text-red-700 dark:text-red-300 hover:underline">Revoke</button></div>
        <div className="mt-2 flex flex-wrap gap-1">{application.scopes.map((scope) => <span key={scope} className="rounded bg-stitch-muted/10 px-2 py-1 text-xs text-stitch-muted">{scope}</span>)}</div>
      </div>)}
    </div>
  </AuthLayout>;
}

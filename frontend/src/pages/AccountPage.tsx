import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { disconnectIdentity, getAuthProviders, getConnectedIdentities, getCsrfToken, startIdentityLink } from '@/api/client';
import type { AuthProviderDiscovery, ConnectedIdentities } from '@/api/types';
import AuthLayout from '@/components/AuthLayout';
import { useDashboard } from '@/context/DashboardContext';

export default function AccountPage() {
  const { csrfToken } = useDashboard();
  const [providers, setProviders] = useState<AuthProviderDiscovery>({ password_enabled: true, external: [] });
  const [accounts, setAccounts] = useState<ConnectedIdentities | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = async () => {
    const [available, connected] = await Promise.all([getAuthProviders(), getConnectedIdentities()]);
    setProviders(available); setAccounts(connected);
  };
  useEffect(() => { void reload().catch((reason) => setError(reason instanceof Error ? reason.message : 'Failed to load account')); }, []);

  const token = async () => csrfToken ?? getCsrfToken();
  const connect = async (provider: string) => {
    try { window.location.assign(await startIdentityLink(provider, await token())); } catch (reason) { setError(reason instanceof Error ? reason.message : 'Failed to connect account'); }
  };
  const disconnect = async (id: number) => {
    try { await disconnectIdentity(id, await token()); await reload(); } catch (reason) { setError(reason instanceof Error ? reason.message : 'Failed to disconnect account'); }
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
    </div>
  </AuthLayout>;
}

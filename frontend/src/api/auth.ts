import type { AuthProviderDiscovery, ChangePasswordBody, ConnectedApplication, ConnectedIdentities, DeploymentInfo } from './types';
import { fetchJson, JSON_HEADERS } from './transport';

export async function getCsrfToken(): Promise<string> {
  const data = await fetchJson<{ csrf_token: string }>('/api/auth/csrf');
  return data.csrf_token;
}

export async function loginJson(
  username: string,
  password: string,
  csrfToken: string,
): Promise<void> {
  await fetchJson<{ status: string }>('/api/auth/login', {
    method: 'POST',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: JSON.stringify({ username, password }),
  });
}

export async function logoutJson(csrfToken: string): Promise<void> {
  await fetchJson<{ status: string }>('/api/auth/logout', {
    method: 'POST',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: '{}',
  });
}

export async function changePassword(
  body: ChangePasswordBody,
  csrfToken: string,
): Promise<void> {
  await fetchJson<{ status: string }>('/api/auth/change-password', {
    method: 'POST',
    headers: {
      ...JSON_HEADERS,
      'X-CSRF-Token': csrfToken,
    },
    body: JSON.stringify(body),
  });
}

export async function getDeploymentInfo(): Promise<DeploymentInfo> {
  return fetchJson<DeploymentInfo>('/api/meta/deployment');
}

export async function getAuthProviders(): Promise<AuthProviderDiscovery> {
  return fetchJson<AuthProviderDiscovery>('/api/auth/providers');
}

export async function getConnectedIdentities(): Promise<ConnectedIdentities> {
  return fetchJson<ConnectedIdentities>('/api/auth/identities');
}

export async function startIdentityLink(provider: string, csrfToken: string): Promise<string> {
  const response = await fetchJson<{ authorization_url: string }>(`/api/auth/external/${encodeURIComponent(provider)}/link`, {
    method: 'POST', headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken }, body: JSON.stringify({ return_to: '/account' }),
  });
  return response.authorization_url;
}

export async function disconnectIdentity(identityId: number, csrfToken: string): Promise<void> {
  await fetchJson<void>(`/api/auth/identities/${identityId}`, { method: 'DELETE', headers: { 'X-CSRF-Token': csrfToken } });
}

export async function getConnectedApplications(): Promise<ConnectedApplication[]> {
  const response = await fetchJson<{ grants: ConnectedApplication[] }>('/api/oauth/grants');
  return response.grants;
}

export async function revokeConnectedApplication(grantId: number, csrfToken: string): Promise<void> {
  await fetchJson<void>(`/api/oauth/grants/${grantId}`, { method: 'DELETE', headers: { 'X-CSRF-Token': csrfToken } });
}

import type { DeploymentInfo } from './types';
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

export async function getDeploymentInfo(): Promise<DeploymentInfo> {
  return fetchJson<DeploymentInfo>('/api/meta/deployment');
}

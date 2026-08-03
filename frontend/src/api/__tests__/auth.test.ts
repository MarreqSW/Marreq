import { afterEach, describe, expect, it, vi } from 'vitest';
import { getCsrfToken, getDeploymentInfo, loginJson, logoutJson } from '../auth';
import { lastFetchCall, mockFetchOk } from './fetchTestUtils';

describe('auth api', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('getCsrfToken unwraps csrf_token', async () => {
    vi.stubGlobal('fetch', mockFetchOk({ csrf_token: 'abc' }));
    await expect(getCsrfToken()).resolves.toBe('abc');
  });

  it('loginJson POSTs credentials with CSRF and JSON headers', async () => {
    vi.stubGlobal('fetch', mockFetchOk({ status: 'ok' }));
    await loginJson('u', 'p', 'csrf');
    const [url, init] = lastFetchCall();
    expect(url).toBe('/api/auth/login');
    expect(init?.method).toBe('POST');
    expect(init?.headers).toMatchObject({
      'Content-Type': 'application/json',
      'X-CSRF-Token': 'csrf',
    });
    expect(JSON.parse(String(init?.body))).toEqual({ username: 'u', password: 'p' });
  });

  it('logoutJson POSTs empty body with CSRF', async () => {
    vi.stubGlobal('fetch', mockFetchOk({ status: 'ok' }));
    await logoutJson('csrf');
    const [url, init] = lastFetchCall();
    expect(url).toBe('/api/auth/logout');
    expect(init?.method).toBe('POST');
    expect(init?.headers).toMatchObject({ 'X-CSRF-Token': 'csrf' });
    expect(init?.body).toBe('{}');
  });

  it('getDeploymentInfo fetches meta endpoint', async () => {
    vi.stubGlobal('fetch', mockFetchOk({ mode: 'saas' }));
    await expect(getDeploymentInfo()).resolves.toEqual({ mode: 'saas' });
    expect(lastFetchCall()[0]).toBe('/api/meta/deployment');
  });
});

import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  registerAccount,
  requestPasswordReset,
  resetPassword,
  verifyEmail,
} from '../publicAuth';
import { lastFetchCall, mockFetchOk } from './fetchTestUtils';

describe('publicAuth api', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('verifyEmail encodes the token in the query', async () => {
    vi.stubGlobal('fetch', mockFetchOk({ status: 'ok' }));
    await verifyEmail('a/b+c=');
    expect(lastFetchCall()[0]).toBe(
      `/api/auth/verify-email?token=${encodeURIComponent('a/b+c=')}`,
    );
  });

  it('registerAccount POSTs JSON without CSRF', async () => {
    vi.stubGlobal('fetch', mockFetchOk({ status: 'ok' }));
    await registerAccount({
      username: 'u',
      name: 'U',
      email: 'u@e.com',
      password: 'secret',
    });
    const [url, init] = lastFetchCall();
    expect(url).toBe('/api/auth/register');
    expect(init?.headers).toEqual({ 'Content-Type': 'application/json' });
  });

  it('requestPasswordReset and resetPassword POST JSON bodies', async () => {
    vi.stubGlobal('fetch', mockFetchOk({ status: 'ok' }));
    await requestPasswordReset({ email: 'a@b.c' });
    expect(lastFetchCall()[0]).toBe('/api/auth/forgot-password');
    await resetPassword({ token: 't', new_password: 'x' });
    expect(lastFetchCall()[0]).toBe('/api/auth/reset-password');
  });
});

import { afterEach, describe, vi } from 'vitest';
import { CSRF, itSendsEachRequest } from '@/test/apiHarness';
import {
  changePassword,
  disconnectIdentity,
  getAuthProviders,
  getConnectedApplications,
  getConnectedIdentities,
  getCsrfToken,
  getDeploymentInfo,
  getMe,
  loginJson,
  logoutJson,
  revokeConnectedApplication,
  startIdentityLink,
  updateMyProfile,
} from '../auth';
import { getBuildInfo } from '../meta';
import { registerAccount, requestPasswordReset, resetPassword, verifyEmail } from '../publicAuth';

describe('auth API', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    {
      name: 'getCsrfToken unwraps the token',
      call: () => getCsrfToken(),
      method: 'GET',
      url: '/api/auth/csrf',
      response: { json: { csrf_token: 'abc' } },
      returns: 'abc',
    },
    {
      name: 'loginJson',
      call: () => loginJson('alice', 'secret', CSRF),
      method: 'POST',
      url: '/api/auth/login',
      body: { username: 'alice', password: 'secret' },
    },
    {
      name: 'logoutJson sends an empty JSON object',
      call: () => logoutJson(CSRF),
      method: 'POST',
      url: '/api/auth/logout',
      body: {},
      json: true,
    },
    {
      name: 'changePassword',
      call: () =>
        changePassword({ current_password: 'a', new_password: 'b', confirm_password: 'b' } as never, CSRF),
      method: 'POST',
      url: '/api/auth/change-password',
      body: { current_password: 'a', new_password: 'b', confirm_password: 'b' },
    },
    { name: 'getDeploymentInfo', call: () => getDeploymentInfo(), method: 'GET', url: '/api/meta/deployment' },
    { name: 'getAuthProviders', call: () => getAuthProviders(), method: 'GET', url: '/api/auth/providers' },
    {
      name: 'getConnectedIdentities',
      call: () => getConnectedIdentities(),
      method: 'GET',
      url: '/api/auth/identities',
    },
    {
      name: 'startIdentityLink returns the provider URL',
      call: () => startIdentityLink('git hub', CSRF),
      method: 'POST',
      url: '/api/auth/external/git%20hub/link',
      body: { return_to: '/account' },
      response: { json: { authorization_url: 'https://idp.example/authorize' } },
      returns: 'https://idp.example/authorize',
    },
    {
      name: 'disconnectIdentity',
      call: () => disconnectIdentity(3, CSRF),
      method: 'DELETE',
      url: '/api/auth/identities/3',
    },
    {
      name: 'getConnectedApplications unwraps the grants',
      call: () => getConnectedApplications(),
      method: 'GET',
      url: '/api/oauth/grants',
      response: { json: { grants: [{ id: 1 }] } },
      returns: [{ id: 1 }],
    },
    {
      name: 'revokeConnectedApplication',
      call: () => revokeConnectedApplication(1, CSRF),
      method: 'DELETE',
      url: '/api/oauth/grants/1',
    },
    { name: 'getMe', call: () => getMe(), method: 'GET', url: '/api/auth/me' },
    {
      name: 'updateMyProfile',
      call: () => updateMyProfile({ name: 'Alice', email: 'alice@example.test' } as never, CSRF),
      method: 'PUT',
      url: '/api/auth/me',
      body: { name: 'Alice', email: 'alice@example.test' },
    },
    { name: 'getBuildInfo', call: () => getBuildInfo(), method: 'GET', url: '/api/meta/build' },
  ]);
});

describe('public auth API (no session, no CSRF)', () => {
  afterEach(() => vi.unstubAllGlobals());

  itSendsEachRequest([
    {
      name: 'registerAccount',
      call: () => registerAccount({ username: 'bob', email: 'bob@example.test' } as never),
      method: 'POST',
      url: '/api/auth/register',
      body: { username: 'bob', email: 'bob@example.test' },
      csrf: false,
    },
    {
      name: 'verifyEmail encodes the token',
      call: () => verifyEmail('a+b/c'),
      method: 'GET',
      url: '/api/auth/verify-email?token=a%2Bb%2Fc',
    },
    {
      name: 'requestPasswordReset',
      call: () => requestPasswordReset({ email: 'bob@example.test' } as never),
      method: 'POST',
      url: '/api/auth/forgot-password',
      body: { email: 'bob@example.test' },
      csrf: false,
    },
    {
      name: 'resetPassword',
      call: () => resetPassword({ token: 't', new_password: 'x', confirm_password: 'x' } as never),
      method: 'POST',
      url: '/api/auth/reset-password',
      body: { token: 't', new_password: 'x', confirm_password: 'x' },
      csrf: false,
    },
  ]);
});

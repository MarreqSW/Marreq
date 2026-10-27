// Real HTTP producer/consumer smoke test. Run only through test-api-contracts.sh:
// it provides an isolated fixture DB and loopback-only Rocket process.
import assert from 'node:assert/strict';
import { appendFileSync } from 'node:fs';
import { MarreqClient } from '../mcp-server/dist/client.js';
import { fetchJson } from '../frontend/src/api/transport.ts';
import { getCsrfToken, loginJson, getMe, logoutJson } from '../frontend/src/api/auth.ts';
import { requestPasswordReset } from '../frontend/src/api/publicAuth.ts';

const base = process.env.MARREQ_CONTRACT_BACKEND_URL;
assert.match(base ?? '', /^http:\/\/127\.0\.0\.1:\d+$/);
assert.ok(process.env.MARREQ_CONTRACT_TRANSCRIPT);
const mode = process.env.MARREQ_CONTRACT_VARIANT ?? 'server';
const cookies = new Map();
const networkFetch = globalThis.fetch;
globalThis.fetch = async (input, init = {}) => {
  const url = new URL(input, base);
  assert.equal(url.origin, base, 'contract smoke must stay on the isolated backend');
  const headers = new Headers(init.headers);
  headers.set('Origin', base);
  if (cookies.size) headers.set('Cookie', [...cookies].map(([k,v]) => `${k}=${v}`).join('; '));
  const response = await networkFetch(url, {...init, headers});
  for (const value of response.headers.getSetCookie()) {
    const cookie = value.split(';')[0]; const equal = cookie.indexOf('=');
    cookies.set(cookie.slice(0,equal), cookie.slice(equal+1));
  }
  const text = await response.clone().text();
  let body = null;
  try { body = JSON.parse(text); } catch { /* Empty guard and 204 bodies are valid. */ }
  // Authorization headers/cookies are never retained in the transcript.
  appendFileSync(process.env.MARREQ_CONTRACT_TRANSCRIPT, JSON.stringify({
    variant: mode, method: init.method ?? 'GET', path: url.pathname + url.search,
    status: response.status,
    request: url.pathname === '/api/auth/login' ? {username:'[test-user]',password:'[redacted]'} : typeof init.body === 'string' ? JSON.parse(init.body) : null,
    response: body,
  }) + '\n');
  return response;
};

// Execute the actual frontend transport against the running Rocket binary.
const build = await fetchJson('/api/meta/build');
assert.equal(build.deployment_mode, mode);
const capabilities = await fetchJson('/api/meta/deployment');
assert.equal(capabilities.allows_self_registration, mode === 'cloud');
assert.equal(capabilities.allows_database_backup, mode === 'server');
assert.equal(capabilities.allows_self_administered_user_creation, mode === 'server');
const discovery = await fetchJson('/.well-known/oauth-authorization-server');
assert.ok(discovery.scopes_supported.includes('use_cases:write'));
assert.deepEqual(discovery.token_endpoint_auth_methods_supported, ['none']);
const csrf = await getCsrfToken();
await loginJson('manager', 'Contract-only-fixture-58!z', csrf);
assert.equal((await getMe()).username, 'manager');
await logoutJson(await getCsrfToken());
await assert.rejects(() => getMe(), (error) => error.status === 401);
if (mode === 'cloud') await requestPasswordReset({email:'absent-contract-user@example.test'});

// These delegated fixture credentials exist only in the ephemeral DB.
const context = { baseUrl: base, apiToken: 'uc-write', projectId: 1, mode: 'draft_write', traceWrite: false };
const writer = new MarreqClient(context);
const reader = new MarreqClient({ ...context, apiToken: 'uc-read' });
assert.equal((await reader.getPrincipal()).authentication_type, 'delegated_oauth');
assert.ok(Array.isArray(await reader.listUseCases()));
const created = await writer.createUseCase({ title: 'Network contract use case', primary_actor: 'Tester' }, `network-contract-create-key-${mode}`);
assert.equal(typeof created.id, 'number');
const document = { schema: 'marreq.use-case.v1', title: 'Network contract use case', main: [{ id: 's_net01', text: 'Exercise the real client against Rocket' }] };
const saved = await writer.putUseCaseDraft(created.id, created.draft.revision, document);
assert.equal(saved.document.main[0].id, 's_net01');
const detail = await reader.getUseCase(created.id);
assert.equal(detail.id, created.id);
await assert.rejects(() => reader.createUseCase({ title: 'Denied write' }, `network-contract-denied-key-${mode}`), (error) => error.status === 403 && error.challenge?.includes('use_cases:write'));
await assert.rejects(() => reader.withProject(2).listUseCases(), (error) => error.status === 403 || /403/.test(error.message));
console.log('Real frontend transport and MCP HTTP read/write/scope/project-boundary checks passed.');

// Real HTTP producer/consumer smoke test. Run only through test-backend.sh:
// it provides an isolated fixture DB and loopback-only Rocket process.
import assert from 'node:assert/strict';
import { appendFileSync } from 'node:fs';
import { MarreqClient } from '../../mcp-server/dist/client.js';
import { fetchJson } from '../../frontend/src/api/transport.ts';

const base = process.env.MARREQ_CONTRACT_BACKEND_URL;
assert.match(base ?? '', /^http:\/\/127\.0\.0\.1:\d+$/);
assert.ok(process.env.MARREQ_CONTRACT_TRANSCRIPT);
const networkFetch = globalThis.fetch;
globalThis.fetch = async (input, init = {}) => {
  const url = new URL(input, base);
  assert.equal(url.origin, base, 'contract smoke must stay on the isolated backend');
  const response = await networkFetch(url, init);
  const text = await response.clone().text();
  let body = null;
  try { body = JSON.parse(text); } catch { /* Empty guard and 204 bodies are valid. */ }
  // Authorization headers/cookies are never retained in the transcript.
  appendFileSync(process.env.MARREQ_CONTRACT_TRANSCRIPT, JSON.stringify({
    method: init.method ?? 'GET', path: url.pathname + url.search,
    status: response.status,
    request: typeof init.body === 'string' ? JSON.parse(init.body) : null,
    response: body,
  }) + '\n');
  return response;
};

// Execute the actual frontend transport against the running Rocket binary.
const build = await fetchJson('/api/meta/build');
assert.equal(build.deployment_mode, 'server');
const capabilities = await fetchJson('/api/meta/deployment');
assert.equal(capabilities.allows_self_registration, false);

// These delegated fixture credentials exist only in the ephemeral DB.
const context = { baseUrl: base, apiToken: 'uc-write', projectId: 1, mode: 'draft_write', traceWrite: false };
const writer = new MarreqClient(context);
const reader = new MarreqClient({ ...context, apiToken: 'uc-read' });
assert.ok(Array.isArray(await reader.listUseCases()));
const created = await writer.createUseCase({ title: 'Network contract use case', primary_actor: 'Tester' }, 'network-contract-create-key');
assert.equal(typeof created.id, 'number');
const document = { schema: 'marreq.use-case.v1', title: 'Network contract use case', main: [{ id: 's_net01', text: 'Exercise the real client against Rocket' }] };
const saved = await writer.putUseCaseDraft(created.id, created.draft.revision, document);
assert.equal(saved.document.main[0].id, 's_net01');
const detail = await reader.getUseCase(created.id);
assert.equal(detail.id, created.id);
await assert.rejects(() => reader.createUseCase({ title: 'Denied write' }, 'network-contract-denied-key'), (error) => error.status === 403);
await assert.rejects(() => reader.withProject(2).listUseCases(), (error) => error.status === 403 || /403/.test(error.message));
console.log('Real frontend transport and MCP HTTP read/write/scope/project-boundary checks passed.');

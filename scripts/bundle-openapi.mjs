// Redocly resolves OpenAPI references; this adapter enforces local-only inputs.
// Exact npm version and integrity are pinned in package.json/package-lock.json.
import { realpathSync } from 'node:fs';
import { dirname, resolve, sep } from 'node:path';
import { pathToFileURL } from 'node:url';
import { BaseResolver, bundle, createConfig } from '@redocly/openapi-core';

export async function bundleOpenApi(input) {
  const entry = realpathSync(input);
  const directory = dirname(entry);
  class LocalResolver extends BaseResolver {
    async loadExternalRef(reference) {
      if (/^[a-z][a-z\d+.-]*:/i.test(reference)) throw new Error(`Network/URI references are forbidden: ${reference}`);
      const file = realpathSync(reference);
      if (file !== entry && !file.startsWith(directory + sep)) throw new Error(`Reference escapes contract directory: ${reference}`);
      return super.loadExternalRef(file);
    }
  }
  const result = await bundle({
    ref: entry,
    config: await createConfig({}),
    externalRefResolver: new LocalResolver(),
  });
  if (result.problems.length) {
    throw new Error(result.problems.map(p => `${p.location?.[0]?.source?.absoluteRef ?? entry}: ${p.message}`).join('\n'));
  }
  // Historical comparison accepts only a self-contained JSON document. No
  // resolver or network access is needed when verifying retained bundles.
  function selfContained(value) {
    if (!value || typeof value !== 'object') return;
    if ('$ref' in value && !value.$ref.startsWith('#/')) throw new Error(`Unresolved reference: ${value.$ref}`);
    for (const child of Object.values(value)) selfContained(child);
  }
  selfContained(result.bundle.parsed);
  return JSON.stringify(result.bundle.parsed, null, 2) + '\n';
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try { process.stdout.write(await bundleOpenApi(process.argv[2] ?? 'docs/developer/openapi/root.yaml')); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
}

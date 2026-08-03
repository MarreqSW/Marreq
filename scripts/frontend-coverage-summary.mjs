#!/usr/bin/env node
/**
 * Build a Markdown coverage summary from Vitest's json-summary report,
 * rolled up by top-level module under frontend/src (api, pages, hooks, …).
 *
 * Usage:
 *   node scripts/frontend-coverage-summary.mjs [summaryJson] [outMd]
 *
 * Defaults:
 *   summaryJson = frontend/coverage/coverage-summary.json
 *   outMd       = frontend-coverage-results.md
 */

import fs from 'node:fs';
import path from 'node:path';

const summaryPath =
  process.argv[2] ?? path.join('frontend', 'coverage', 'coverage-summary.json');
const outPath = process.argv[3] ?? 'frontend-coverage-results.md';

if (!fs.existsSync(summaryPath)) {
  console.error(`Coverage summary not found: ${summaryPath}`);
  process.exit(1);
}

const data = JSON.parse(fs.readFileSync(summaryPath, 'utf8'));

/** @type {Map<string, { stmts: number; covered: number; branches: number; branchesCovered: number; funcs: number; funcsCovered: number; lines: number; linesCovered: number }>} */
const modules = new Map();

function moduleKey(filePath) {
  // Vitest keys are absolute or cwd-relative; normalize to frontend/src/...
  const normalized = filePath.replace(/\\/g, '/');
  const marker = '/frontend/src/';
  const idx = normalized.indexOf(marker);
  const rel =
    idx >= 0
      ? normalized.slice(idx + marker.length)
      : normalized.startsWith('src/')
        ? normalized.slice('src/'.length)
        : normalized.replace(/^.*\/src\//, '');
  if (!rel || rel === 'total' || filePath === 'total') return null;
  const parts = rel.split('/');
  // Files directly under src/ (App.tsx, statusAuthorDefaults.ts)
  if (parts.length === 1) return 'src (root)';
  // Keep catalog/groups as their own modules under pages/
  if (parts[0] === 'pages' && (parts[1] === 'catalog' || parts[1] === 'groups')) {
    return `pages/${parts[1]}`;
  }
  if (parts[0] === 'components' && (parts[1] === 'graph' || parts[1] === 'reports' || parts[1] === 'table')) {
    return `components/${parts[1]}`;
  }
  return parts[0];
}

for (const [filePath, metrics] of Object.entries(data)) {
  if (filePath === 'total') continue;
  const key = moduleKey(filePath);
  if (!key) continue;
  const bucket = modules.get(key) ?? {
    stmts: 0,
    covered: 0,
    branches: 0,
    branchesCovered: 0,
    funcs: 0,
    funcsCovered: 0,
    lines: 0,
    linesCovered: 0,
  };
  bucket.stmts += metrics.statements?.total ?? 0;
  bucket.covered += metrics.statements?.covered ?? 0;
  bucket.branches += metrics.branches?.total ?? 0;
  bucket.branchesCovered += metrics.branches?.covered ?? 0;
  bucket.funcs += metrics.functions?.total ?? 0;
  bucket.funcsCovered += metrics.functions?.covered ?? 0;
  bucket.lines += metrics.lines?.total ?? 0;
  bucket.linesCovered += metrics.lines?.covered ?? 0;
  modules.set(key, bucket);
}

function pct(covered, total) {
  if (!total) return 100;
  return (100 * covered) / total;
}

function health(linePct) {
  if (linePct >= 80) return '✔';
  if (linePct >= 50) return '➖';
  return '❌';
}

const total = data.total?.lines ?? { total: 0, covered: 0, pct: 0 };
const totalPct = total.pct ?? pct(total.covered, total.total);
const badgeColor =
  totalPct >= 80 ? 'success' : totalPct >= 50 ? 'yellow' : 'critical';

const rows = [...modules.entries()]
  .map(([name, m]) => {
    const linePct = pct(m.linesCovered, m.lines);
    return {
      name,
      linePct,
      lines: `${m.linesCovered} / ${m.lines}`,
      branchPct: pct(m.branchesCovered, m.branches),
      funcPct: pct(m.funcsCovered, m.funcs),
      health: health(linePct),
    };
  })
  .sort((a, b) => a.name.localeCompare(b.name));

const md = [
  '<!-- add-pr-comment:frontend-coverage -->',
  '',
  `![Frontend Code Coverage](https://img.shields.io/badge/Frontend%20Coverage-${Math.round(totalPct)}%25-${badgeColor}?style=flat)`,
  '',
  '## Frontend SPA coverage (by module)',
  '',
  'Module | Line Rate | Branch Rate | Function Rate | Health',
  '-------- | --------- | ----------- | ------------- | ------',
  ...rows.map(
    (r) =>
      `\`${r.name}\` | **${r.linePct.toFixed(1)}%** (${r.lines}) | ${r.branchPct.toFixed(1)}% | ${r.funcPct.toFixed(1)}% | ${r.health}`,
  ),
  `**Summary** | **${totalPct.toFixed(1)}%** (${total.covered} / ${total.total}) | ${(data.total?.branches?.pct ?? 0).toFixed(1)}% | ${(data.total?.functions?.pct ?? 0).toFixed(1)}% | ${health(totalPct)}`,
  '',
  '_Generated from Vitest `@vitest/coverage-v8` (`frontend/src`)._',
  '',
].join('\n');

fs.writeFileSync(outPath, md, 'utf8');
console.log(`Wrote ${outPath}`);
console.log(md);

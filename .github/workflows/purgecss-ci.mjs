import fs from 'node:fs';
import { PurgeCSS } from 'purgecss';

const TOLERANCE = 0;

const cfg = (await import('./purgecss.config.cjs')).default ?? (await import('./purgecss.config.cjs'));
const results = await new PurgeCSS().purge({
  ...cfg,
  rejected: true,           // ask PurgeCSS to list what it would remove
  defaultExtractor: (c) => c.match(/[\w-/:%.@]+(?<!:)/g) || [],
});

let total = 0;
let report = `# Unused CSS Report\n\n`;

for (const r of results) {
  const rejected = r.rejected ?? [];
  if (!rejected.length) continue;
  total += rejected.length;
  report += `**${r.file || '(inline)'}** — ${rejected.length} unused selectors\n`;
  const sample = rejected.slice(0, 100).map(s => `- \`${s}\``).join('\n');
  report += `${sample}\n\n`;
}

report += `**Total unused selectors:** ${total}\n`;

const summary = process.env.GITHUB_STEP_SUMMARY;
if (summary) fs.appendFileSync(summary, `${report}\n`);
console.log(report);

if (total > TOLERANCE) {
  console.error(`Unused CSS found: ${total} > tolerance ${TOLERANCE}`);
  process.exit(1);
}

#!/usr/bin/env node
/**
 * Capture the screenshots of the Marreq User Manual (docs/user-manual/screenshots/).
 *
 * Needs a running SPA with demo data (marreq-core/scripts/init_complete.sql),
 * e.g. the Vite dev server in front of a backend:
 *
 *   MARREQ_API_PROXY_TARGET=http://127.0.0.1:<backend port> npx vite --port <port>   (in frontend/)
 *   MARREQ_URL=http://127.0.0.1:<port> npm run screenshots:manual
 *
 * Environment:
 *   MARREQ_URL                 SPA address (default http://localhost:5173)
 *   MARREQ_USER / MARREQ_PASS  account to sign in with (default alice / ChangeMe123!)
 *   MARREQ_PROJECT_BASE_PATH   project to show (default /space-project)
 *   MARREQ_CHROME              Chrome/Chromium executable, if Playwright's own browser is not installed
 *
 * Use a throwaway database: the script only reads, but it signs in.
 */

import { chromium } from 'playwright';
import { fileURLToPath } from 'url';
import path from 'path';
import { mkdir } from 'fs/promises';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const OUT_DIR = path.join(__dirname, 'screenshots');
const BASE_URL = (process.env.MARREQ_URL || 'http://localhost:5173').replace(/\/$/, '');
const USER = process.env.MARREQ_USER || 'alice';
const PASS = process.env.MARREQ_PASS || 'ChangeMe123!';
const PROJECT = process.env.MARREQ_PROJECT_BASE_PATH || '/space-project';

async function main() {
  await mkdir(OUT_DIR, { recursive: true });
  const browser = await chromium.launch({
    headless: true,
    executablePath: process.env.MARREQ_CHROME || undefined,
  });
  const context = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    colorScheme: 'light',
  });
  const page = await context.newPage();

  const shot = async (name) => {
    // Let fonts, icons and transitions settle before taking the picture.
    await page.waitForLoadState('networkidle');
    await page.evaluate(() => document.fonts.ready);
    await page.waitForTimeout(300);
    const file = path.join(OUT_DIR, `${name}.png`);
    await page.screenshot({ path: file, fullPage: false });
    console.log('Saved', file);
  };
  const open = async (route, readyText) => {
    await page.goto(`${BASE_URL}${route}`, { waitUntil: 'domcontentloaded' });
    if (readyText) await page.getByText(readyText).first().waitFor({ timeout: 15000 });
  };

  try {
    await open('/login');
    await page.locator('#username').waitFor();
    await shot('login');
    await page.fill('#username', USER);
    await page.fill('#password', PASS);
    await page.click('button[type="submit"]');
    await page.waitForURL((u) => !u.pathname.startsWith('/login'), { timeout: 15000 });

    await open(`${PROJECT}/dashboard`, 'Project overview');
    await shot('dashboard');

    await open(`${PROJECT}/requirements`, /Requirements found/);
    await shot('requirements-list');

    await open(`${PROJECT}/requirements/1`, 'Requirement statement');
    await shot('requirement-detail');

    await open(`${PROJECT}/requirements/new`, 'Create requirement');
    await page.locator('footer').last().scrollIntoViewIfNeeded();
    await shot('requirement-create');

    await open(`${PROJECT}/verifications`, 'Pass rate');
    await shot('verifications-list');

    await open(`${PROJECT}/traceability?view=matrix`, 'Clear all filters');
    await shot('matrix');

    // Requirements only; select the node nearest the middle of the canvas and
    // zoom in on it (a whole project is too small to read at fit-to-view).
    await open(`${PROJECT}/traceability?view=hierarchy&kind=reqs`);
    const nodes = page.locator('.react-flow__node');
    await nodes.first().waitFor({ timeout: 15000 });
    const pane = await page.locator('.react-flow__pane').boundingBox();
    const middle = pane.y + pane.height / 2;
    let best = 0;
    let bestDistance = Infinity;
    for (let i = 0; i < (await nodes.count()); i += 1) {
      const box = await nodes.nth(i).boundingBox();
      const distance = box ? Math.abs(box.y + box.height / 2 - middle) : Infinity;
      if (distance < bestDistance) [best, bestDistance] = [i, distance];
    }
    const node = nodes.nth(best);
    await node.click({ force: true });
    await page.getByText('Add child').first().waitFor();
    const box = await node.boundingBox();
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    for (let i = 0; i < 7; i += 1) {
      await page.mouse.wheel(0, -300);
      await page.waitForTimeout(120);
    }
    await shot('hierarchy');

    await open(`${PROJECT}/traceability?view=dsm`, 'Link types');
    await shot('dsm');

    await open(`${PROJECT}/baselines`, 'Create baseline');
    await shot('baselines-list');

    await open(`${PROJECT}/reports`, 'Coverage & gaps');
    await shot('reports');

    await open(`${PROJECT}/settings/members`, 'Save reviewer list');
    await shot('settings-members');
  } catch (err) {
    console.error('Capture failed:', err.message);
    process.exitCode = 1;
  } finally {
    await browser.close();
  }
}

main();

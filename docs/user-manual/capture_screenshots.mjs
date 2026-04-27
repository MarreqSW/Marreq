#!/usr/bin/env node
/**
 * Capture screenshots for the Marreq User Manual.
 * Run with the Marreq app and DB up: cargo run -p marreq-server (and docker compose -f docker/docker-compose.yml up -d db, ./marreq-core/scripts/db_setup.sh --seed).
 * Then: node docs/user-manual/capture_screenshots.mjs
 * Or: node capture_screenshots.mjs from docs/user-manual.
 *
 * Requires: npm install -D playwright && npx playwright install chromium
 */

import { chromium } from 'playwright';
import { fileURLToPath } from 'url';
import path from 'path';
import { mkdir } from 'fs/promises';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const OUT_DIR = path.join(__dirname, 'screenshots');
const BASE_URL = process.env.MARREQ_URL || 'http://localhost:8000';
const USER = process.env.MARREQ_USER || 'alice';
const PASS = process.env.MARREQ_PASS || 'ChangeMe123!';
const PROJECT_BASE_PATH =
  process.env.MARREQ_PROJECT_BASE_PATH || '/dr_smith/space-project';

async function main() {
  await mkdir(OUT_DIR, { recursive: true });

  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1200, height: 800 },
    ignoreHTTPSErrors: true,
  });
  const page = await context.newPage();

  const shot = async (name) => {
    const file = path.join(OUT_DIR, `${name}.png`);
    await page.screenshot({ path: file, fullPage: false });
    console.log('Saved', file);
  };

  try {
    // Login page (before logging in)
    await page.goto(BASE_URL, { waitUntil: 'networkidle' });
    await page.waitForSelector('input[name="username"]', { timeout: 10000 }).catch(() => null);
    if (await page.locator('input[name="username"]').isVisible()) {
      await shot('login');
      await page.fill('input[name="username"]', USER);
      await page.fill('input[name="password"]', PASS);
      await page.click('button[type="submit"]');
      await page.waitForURL(u => !u.pathname.endsWith('/login') || u.search.includes('error'), { timeout: 5000 }).catch(() => {});
    }

    // Home (after login)
    await page.goto(BASE_URL, { waitUntil: 'networkidle' });
    await page.waitForTimeout(800);
    await shot('home');

    // Projects list
    await page.goto(`${BASE_URL}/projects`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(500);
    await shot('projects');

    // Project detail
    await page.goto(`${BASE_URL}${PROJECT_BASE_PATH}`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(500);
    await shot('project-detail');

    // Requirements list
    await page.goto(`${BASE_URL}${PROJECT_BASE_PATH}/requirements`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(800);
    await shot('requirements-list');

    // Requirement detail (assume requirement 1 exists in the selected project)
    await page.goto(`${BASE_URL}${PROJECT_BASE_PATH}/requirements/show/1`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(500);
    await shot('requirement-detail');

    // Verifications list
    await page.goto(`${BASE_URL}${PROJECT_BASE_PATH}/verifications`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(500);
    await shot('tests-list');

    // Matrix
    await page.goto(`${BASE_URL}${PROJECT_BASE_PATH}/matrix`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(800);
    await shot('matrix');

    // Baselines list
    await page.goto(`${BASE_URL}${PROJECT_BASE_PATH}/baselines`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(500);
    await shot('baselines-list');

    // Reports
    await page.goto(`${BASE_URL}${PROJECT_BASE_PATH}/reports`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(500);
    await shot('reports');
  } catch (e) {
    console.error(e);
    process.exitCode = 1;
  } finally {
    await browser.close();
  }
}

main();

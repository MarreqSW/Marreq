#!/usr/bin/env node
/**
 * Capture screenshots for the ReqMan User Manual.
 * Run with the ReqMan app and DB up: cargo run --bin req_man (and docker compose up -d db, setup_database.sh).
 * Then: node doc/UserManual/capture_screenshots.mjs
 * Or: node capture_screenshots.mjs from doc/UserManual.
 *
 * Requires: npm install -D playwright && npx playwright install chromium
 */

import { chromium } from 'playwright';
import { fileURLToPath } from 'url';
import path from 'path';
import { mkdir } from 'fs/promises';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const OUT_DIR = path.join(__dirname, 'screenshots');
const BASE_URL = process.env.REQMAN_URL || 'http://localhost:8000';
const USER = process.env.REQMAN_USER || 'alice';
const PASS = process.env.REQMAN_PASS || 'ChangeMe123!';

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

    // Project detail (assume project 1 exists in demo data)
    await page.goto(`${BASE_URL}/p/1`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(500);
    await shot('project-detail');

    // Requirements list
    await page.goto(`${BASE_URL}/p/1/requirements`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(800);
    await shot('requirements-list');

    // Requirement detail (assume requirement 1 exists)
    await page.goto(`${BASE_URL}/p/1/requirements/show/1`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(500);
    await shot('requirement-detail');

    // Tests list
    await page.goto(`${BASE_URL}/p/1/tests`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(500);
    await shot('tests-list');

    // Matrix
    await page.goto(`${BASE_URL}/p/1/matrix`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(800);
    await shot('matrix');

    // Baselines list
    await page.goto(`${BASE_URL}/p/1/baselines`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(500);
    await shot('baselines-list');

    // Reports
    await page.goto(`${BASE_URL}/p/1/reports`, { waitUntil: 'networkidle' });
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

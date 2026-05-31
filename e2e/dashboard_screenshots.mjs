import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const serverBin = process.env.CORTEX_SERVER_BIN || 'target/debug/cortex-server';
const outputDir = resolve(process.env.DASHBOARD_SCREENSHOT_DIR || 'target/dashboard');

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function freePort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      if (!address || typeof address === 'string') {
        server.close(() => reject(new Error('failed to allocate TCP port')));
        return;
      }
      server.close(() => resolve(address.port));
    });
  });
}

async function waitForHealth(baseUrl, server) {
  const deadline = Date.now() + 10_000;
  let lastError;
  while (Date.now() < deadline) {
    if (server.exitCode !== null) {
      throw new Error(`server exited before readiness: ${server.exitCode}`);
    }
    try {
      const response = await fetch(`${baseUrl}/v1/health`);
      if (response.ok) return;
    } catch (error) {
      lastError = error;
    }
    await sleep(100);
  }
  throw new Error(`server did not become ready: ${String(lastError)}`);
}

async function stopServer(server) {
  if (server.exitCode !== null) return;
  const exited = new Promise(resolve => server.once('exit', resolve));
  server.kill('SIGTERM');
  await Promise.race([exited, sleep(2_000)]);
  if (server.exitCode === null) server.kill('SIGKILL');
}

async function seedAndSearch(page, baseUrl, cellId, note) {
  await page.goto(`${baseUrl}/dashboard`, { waitUntil: 'networkidle' });
  await page.locator('#output').waitFor({ state: 'visible' });
  await page.getByRole('link', { name: 'Cells' }).click();
  await page.locator('#cell-id').fill(String(cellId));
  await page.locator('#cell-payload').fill([
    'scope=project:investments',
    'status=ready',
    'type=fact',
    'source=dashboard-screenshot',
    '',
    note,
  ].join('\n'));
  await page.getByRole('button', { name: 'Run Cell Operation' }).click();
  await page.waitForFunction(
    text => document.querySelector('#output')?.textContent?.includes(text),
    '"seq"',
    { timeout: 5_000 },
  );

  await page.getByRole('link', { name: 'Search' }).click();
  await page.locator('#search-query').fill('budget');
  await page.getByRole('button', { name: /^Search$/ }).click();
  await page.waitForFunction(
    text => document.querySelector('#output')?.textContent?.includes(text),
    note,
    { timeout: 5_000 },
  );
}

async function main() {
  mkdirSync(outputDir, { recursive: true });
  const root = mkdtempSync(join(tmpdir(), 'cortexdb-dashboard-shot-'));
  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const stderr = [];
  const server = spawn(serverBin, [root, `127.0.0.1:${port}`], {
    cwd: process.cwd(),
    env: { ...process.env, RUST_LOG: 'warn' },
    stdio: ['ignore', 'ignore', 'pipe'],
  });
  server.stderr?.on('data', chunk => stderr.push(String(chunk)));

  let browser;
  try {
    await waitForHealth(baseUrl, server);
    browser = await chromium.launch();

    const desktopContext = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
    const desktopPage = await desktopContext.newPage();
    await seedAndSearch(desktopPage, baseUrl, 92001, 'Dashboard screenshot budget note');
    const desktopPath = join(outputDir, 'dashboard-desktop.png');
    await desktopPage.screenshot({ path: desktopPath, fullPage: true });
    await desktopContext.close();

    const mobileContext = await browser.newContext({
      viewport: { width: 390, height: 900 },
      deviceScaleFactor: 2,
      isMobile: true,
    });
    const mobilePage = await mobileContext.newPage();
    await seedAndSearch(mobilePage, baseUrl, 92002, 'Dashboard mobile screenshot budget note');
    const mobilePath = join(outputDir, 'dashboard-mobile.png');
    await mobilePage.screenshot({ path: mobilePath, fullPage: true });
    await mobileContext.close();

    const summary = {
      generated_by: 'e2e/dashboard_screenshots.mjs',
      screenshots: [
        { name: 'dashboard-desktop', file: 'dashboard-desktop.png', viewport: '1440x1000' },
        { name: 'dashboard-mobile', file: 'dashboard-mobile.png', viewport: '390x900@2x' },
      ],
    };
    writeFileSync(join(outputDir, 'summary.json'), JSON.stringify(summary, null, 2));
  } catch (error) {
    throw new Error(`${error.message}\nserver stderr:\n${stderr.join('')}`);
  } finally {
    if (browser) await browser.close();
    await stopServer(server);
    rmSync(root, { recursive: true, force: true });
  }
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});

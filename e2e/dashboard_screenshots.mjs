import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdirSync, mkdtempSync, readdirSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const serverBin = process.env.CORTEX_SERVER_BIN || 'target/debug/cortex-server';
const outputDir = resolve(process.env.DASHBOARD_SCREENSHOT_DIR || 'target/dashboard');

const DASHBOARD_ROUTE_SCREENSHOTS = [
  { name: "overview", path: "/dashboard/overview", actions: ["operational-status"] },
  { name: "permissions", path: "/dashboard/permissions", actions: [] },
  { name: "cells", path: "/dashboard/cells", actions: [] },
  { name: "search", path: "/dashboard/search", actions: ["search-budget"] },
  { name: "ann-eval", path: "/dashboard/ann-eval", actions: ["ann-eval"] },
  { name: "aql", path: "/dashboard/aql", actions: ["aql"] },
  { name: "context", path: "/dashboard/context", actions: ["context"] },
  { name: "verify", path: "/dashboard/verify", actions: ["verify"] },
  { name: "ingest", path: "/dashboard/ingest", actions: ["ingest"] },
  { name: "storage", path: "/dashboard/storage", actions: ["storage"] },
  { name: "cluster", path: "/dashboard/cluster", actions: ["cluster"] },
];

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

async function runRouteWorkflow(page, routeId, note) {
  switch (routeId) {
    case "health":
      await page.getByRole('button', { name: 'Health' }).click();
      break;
    case "operational-status":
      await page.getByRole('button', { name: 'Refresh Status' }).click();
      await page.waitForFunction(
        text => document.querySelector('#status-report')?.textContent?.includes(text),
        'No dashboard-visible incidents',
        { timeout: 5_000 },
      );
      break;
    case "search-budget":
      await page.locator('#search-query').fill('budget');
      await page.locator('#search-query').dispatchEvent('input');
      await page.getByRole('button', { name: /^Search$/ }).click();
      await page.waitForFunction(
        text => document.querySelector('#output')?.textContent?.includes(text),
        note,
        { timeout: 5_000 },
      );
      break;
    case "ann-eval":
      await page.locator('#ann-scope').fill('project:investments');
      await page.locator('#ann-vector').fill('0,10');
      await page.locator('#ann-limit').fill('20');
      await page.getByRole('button', { name: 'Evaluate ANN' }).click();
      await page.waitForFunction(
        text => document.querySelector('#output')?.textContent?.includes(text),
        '\"available\"',
        { timeout: 5_000 },
      );
      break;
    case "aql":
      await page.getByRole('button', { name: 'Run AQL' }).click();
      await page.waitForFunction(
        text => document.querySelector('#output')?.textContent?.includes(text),
        note,
        { timeout: 5_000 },
      );
      break;
    case "context":
      await page.getByRole('button', { name: 'Build Context Pack' }).click();
      await page.waitForFunction(
        text => document.querySelector('#output')?.textContent?.includes(text),
        '\"schema_version\"',
        { timeout: 5_000 },
      );
      break;
    case "verify":
      await page.getByRole('button', { name: 'Verify Fact' }).click();
      await page.waitForFunction(
        text => document.querySelector('#output')?.textContent?.includes(text),
        '\"verdict\"',
        { timeout: 5_000 },
      );
      break;
    case "ingest":
      await page.getByRole('button', { name: 'Ingest', exact: true }).click();
      await page.waitForFunction(
        text => document.querySelector('#output')?.textContent?.includes(text),
        '\"chunks_ingested\"',
        { timeout: 5_000 },
      );
      break;
    case "storage":
      await page.getByRole('button', { name: 'Validate' }).click();
      await page.waitForFunction(
        text => document.querySelector('#output')?.textContent?.includes(text),
        '\"manifest_ok\"',
        { timeout: 5_000 },
      );
      break;
    case "cluster":
      await page.getByRole('button', { name: 'Cluster Status' }).click();
      await page.waitForFunction(
        text => document.querySelector('#output')?.textContent?.includes(text),
        'distributed_enabled',
        { timeout: 5_000 },
      );
      break;
    default:
      break;
  }
}

async function captureRouteShots(page, baseUrl, route, note) {
  await page.goto(`${baseUrl}${route.path}`, { waitUntil: 'networkidle' });
  await page.locator('#output').waitFor({ state: 'visible' });
  for (const action of route.actions) {
    await runRouteWorkflow(page, action, note);
  }
}

async function captureViewportShots(page, baseUrl, tag, viewport, note) {
  const shots = [];
  for (const route of DASHBOARD_ROUTE_SCREENSHOTS) {
    await captureRouteShots(page, baseUrl, route, note);
    const file = join(outputDir, `dashboard-${tag}-${viewport}-${route.name}.png`);
    await page.screenshot({ path: file, fullPage: true });
    shots.push({ name: route.name, file: `dashboard-${tag}-${viewport}-${route.name}.png`, viewport });
  }
  return shots;
}

function cleanupLegacyArtifacts() {
  if (!readdirSync(outputDir).includes('dashboard-desktop.png')) return;
  const legacyDesktop = join(outputDir, 'dashboard-desktop.png');
  const legacyMobile = join(outputDir, 'dashboard-mobile.png');
  if (readdirSync(outputDir).includes('dashboard-desktop.png')) {
    unlinkSync(legacyDesktop);
  }
  if (readdirSync(outputDir).includes('dashboard-mobile.png')) {
    unlinkSync(legacyMobile);
  }
}

async function main() {
  mkdirSync(outputDir, { recursive: true });
  cleanupLegacyArtifacts();
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
    const desktopShots = await captureViewportShots(
      desktopPage,
      baseUrl,
      'desktop',
      '1440x1000',
      'Dashboard screenshot budget note',
    );
    await desktopContext.close();

    const mobileContext = await browser.newContext({
      viewport: { width: 390, height: 900 },
      deviceScaleFactor: 2,
      isMobile: true,
    });
    const mobilePage = await mobileContext.newPage();
    await seedAndSearch(mobilePage, baseUrl, 92002, 'Dashboard mobile screenshot budget note');
    const mobileShots = await captureViewportShots(
      mobilePage,
      baseUrl,
      'mobile',
      '390x900@2x',
      'Dashboard mobile screenshot budget note',
    );
    await mobileContext.close();

    const summary = {
      generated_by: 'e2e/dashboard_screenshots.mjs',
      screenshots: [
        ...desktopShots,
        ...mobileShots,
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

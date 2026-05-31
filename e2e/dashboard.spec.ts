import { expect, test } from '@playwright/test';
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer } from 'node:net';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

async function freePort(): Promise<number> {
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

async function waitForHealth(baseUrl: string, server: ChildProcess): Promise<void> {
  const deadline = Date.now() + 10_000;
  let lastError: unknown;
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
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error(`server did not become ready: ${String(lastError)}`);
}

test('dashboard loads versioned assets and drives core forms', async ({ page, request }) => {
  const root = mkdtempSync(join(tmpdir(), 'cortexdb-dashboard-e2e-'));
  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const server = spawn('target/debug/cortex-server', [root, `127.0.0.1:${port}`], {
    cwd: process.cwd(),
    env: { ...process.env, RUST_LOG: 'warn' },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  const stderr: string[] = [];
  server.stderr?.on('data', chunk => stderr.push(String(chunk)));

  try {
    await waitForHealth(baseUrl, server);

    const style = await request.get(`${baseUrl}/dashboard/assets/v1/style.css`);
    expect(style.ok()).toBeTruthy();
    expect(style.headers()['content-type']).toContain('text/css');
    expect(await style.text()).toContain('.panel.active');

    const script = await request.get(`${baseUrl}/dashboard/assets/v1/app.js`);
    expect(script.ok()).toBeTruthy();
    expect(script.headers()['content-type']).toContain('application/javascript');
    expect(await script.text()).toContain('run("stats"');

    const consoleErrors: string[] = [];
    page.on('console', message => {
      if (message.type() === 'error') consoleErrors.push(message.text());
    });
    page.on('pageerror', error => consoleErrors.push(error.message));

    await page.goto(`${baseUrl}/dashboard/search`);
    await expect(page).toHaveTitle('Search | CortexDB Console');
    await expect(page.locator('#search')).toBeVisible();

    await page.goto(`${baseUrl}/dashboard`);
    await expect(page).toHaveTitle('Overview | CortexDB Console');
    await expect(page.getByRole('heading', { name: 'CortexDB Console' })).toBeVisible();
    await expect(page.locator('link[href="/dashboard/assets/v1/style.css"]')).toHaveCount(1);
    await expect(page.locator('script[src="/dashboard/assets/v1/app.js"]')).toHaveCount(1);
    await expect(page.locator('#output')).toContainText('current_seq');

    await page.getByRole('link', { name: 'Cells' }).click();
    await expect(page).toHaveURL(/\/dashboard\/cells$/);
    await expect(page).toHaveTitle('Cells | CortexDB Console');
    await expect(page.locator('#cells')).toBeVisible();
    await page.locator('#cell-id').fill('91001');
    await page.locator('#cell-payload').fill([
      'scope=project:investments',
      'status=ready',
      'type=fact',
      'source=dashboard-smoke',
      '',
      'Dashboard smoke budget note',
    ].join('\n'));
    await page.getByRole('button', { name: 'Run Cell Operation' }).click();
    await expect(page.locator('#output')).toContainText('"seq"');

    await page.locator('#cell-op').selectOption('get');
    await page.getByRole('button', { name: 'Run Cell Operation' }).click();
    await expect(page.locator('#output')).toContainText('Dashboard smoke budget note');

    await page.getByRole('link', { name: 'Search' }).click();
    await expect(page).toHaveURL(/\/dashboard\/search$/);
    await expect(page.locator('#search')).toBeVisible();
    await page.locator('#search-query').fill('budget');
    await page.getByRole('button', { name: 'Search', exact: true }).click();
    await expect(page.locator('#output')).toContainText('Dashboard smoke budget note');

    await page.getByRole('button', { name: 'Explain Search' }).click();
    await expect(page.locator('#output')).toContainText('query_terms');
    await expect(page.locator('#output')).toContainText('Dashboard smoke budget note');

    await page.getByRole('link', { name: 'Storage' }).click();
    await expect(page).toHaveURL(/\/dashboard\/storage$/);
    await expect(page.locator('#storage')).toBeVisible();
    await page.getByRole('button', { name: 'Validate' }).click();
    await expect(page.locator('#output')).toContainText('manifest_ok');

    await page.getByRole('link', { name: 'Cluster' }).click();
    await expect(page).toHaveURL(/\/dashboard\/cluster$/);
    await expect(page.locator('#cluster')).toBeVisible();
    await page.getByRole('button', { name: 'Cluster Status' }).click();
    await expect(page.locator('#output')).toContainText('distributed_enabled');

    await page.getByRole('link', { name: 'Cells' }).click();
    await page.locator('#cell-op').selectOption('get');
    await page.locator('#cell-id').fill('not-a-number');
    await page.getByRole('button', { name: 'Run Cell Operation' }).click();
    await expect(page.locator('#request-status')).toContainText('ERR get cell');
    await expect(page.locator('#output')).toContainText('bad_request');

    await expect(page.locator('#history')).toContainText('OK search');
    await expect(page.locator('#history')).toContainText('OK cluster status');
    await expect(page.locator('#history li').first()).toContainText('ERR get cell');
    expect(consoleErrors.filter(message => !message.includes('400 (Bad Request)'))).toEqual([]);
  } finally {
    server.kill('SIGTERM');
    rmSync(root, { recursive: true, force: true });
    if (server.exitCode && server.exitCode !== 0) {
      throw new Error(`server exited with ${server.exitCode}: ${stderr.join('')}`);
    }
  }
});

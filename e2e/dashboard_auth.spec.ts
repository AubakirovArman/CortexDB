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

async function waitForHealth(baseUrl: string, server: ChildProcess, token = ""): Promise<void> {
  const deadline = Date.now() + 10_000;
  let lastError: unknown;
  const headers = token ? { Authorization: `Bearer ${token}` } : undefined;
  while (Date.now() < deadline) {
    if (server.exitCode !== null) {
      throw new Error(`server exited before readiness: ${server.exitCode}`);
    }
    try {
      const response = await fetch(`${baseUrl}/v1/health`, headers ? { headers } : {});
      if (response.ok) return;
    } catch (error) {
      lastError = error;
    }
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error(`server did not become ready: ${String(lastError)}`);
}

test('dashboard role-aware auth gating', async ({ page, request }) => {
  const root = mkdtempSync(join(tmpdir(), 'cortexdb-dashboard-auth-'));
  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const server = spawn('target/debug/cortex-server', [root, `127.0.0.1:${port}`], {
    cwd: process.cwd(),
    env: {
      ...process.env,
      RUST_LOG: 'warn',
      CORTEXDB_AUTH_TOKENS: 'admin:adm,data:data-token',
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  const consoleErrors: string[] = [];
  const failedAuthChecks: string[] = [];

  try {
    await waitForHealth(baseUrl, server, 'adm');

    let requestAuthToken = 'adm';
    const setRequestAuthToken = (token: string) => {
      requestAuthToken = token;
    };
    const clearTraces = () => {
      consoleErrors.length = 0;
      failedAuthChecks.length = 0;
    };
    const unexpectedAuthFailures = () =>
      failedAuthChecks.filter((entry) => !entry.includes('/v1/stats'));

    await page.route('**/*', async route => {
      const request = route.request();
      const url = new URL(request.url());

      if (!url.pathname.startsWith('/dashboard') && !url.pathname.startsWith('/v1')) {
        await route.continue();
        return;
      }

      const headers = { ...request.headers() } as Record<string, string>;
      if (requestAuthToken.length > 0) {
        delete headers.Authorization;
        delete headers.authorization;
        headers.authorization = `Bearer ${requestAuthToken}`;
      } else {
        delete headers.authorization;
        delete headers.Authorization;
      }
      await route.continue({ headers });
    });

    page.on('console', message => {
      if (message.type() === 'error') consoleErrors.push(message.text());
    });
    page.on('pageerror', error => {
      consoleErrors.push(error.message);
    });
    page.on('response', response => {
      if (response.url().startsWith(baseUrl) && response.status() === 403) {
        failedAuthChecks.push(`${response.status()} ${response.url()}`);
      }
    });
    page.on('requestfailed', request => {
      if (request.url().startsWith(baseUrl)) {
        failedAuthChecks.push(`failed ${request.url()}`);
      }
    });

    await page.goto(`${baseUrl}/dashboard`);
    clearTraces();
    await expect(page.locator('#session-role')).toContainText('Access level: admin');
    await expect(page.locator('#permission-report')).toContainText('Admin access');
    await expect(page.locator('a[data-route="storage"]')).toBeVisible();
    expect(consoleErrors).toEqual([]);
    expect(unexpectedAuthFailures()).toEqual([]);

    const deniedDashboardResponse = await request.get(`${baseUrl}/dashboard`, {
      headers: { Authorization: 'Bearer data-token' },
    });
    expect(deniedDashboardResponse.status()).toBe(403);

    setRequestAuthToken('data-token');
    await page.locator('#token').fill('data-token');
    await page.getByRole('button', { name: 'Apply' }).click();

    await expect(page.locator('#session-role')).toContainText('Access level: data');
    await expect(page.locator('#permission-report')).toContainText('Data access');
    await expect(page.locator('#permission-report')).toContainText('Storage maintenance is hidden');
    await expect(page.locator('a[data-route="storage"]')).toBeHidden();
    await expect(page.locator('button[data-action="compact"]')).toBeHidden();
    await expect(page.locator('#ann-eval button[data-action="ann-metrics"]')).toBeHidden();
    await page.getByRole('button', { name: 'Health' }).click();
    await expect(page.locator('#output')).toContainText('"status": "ok"');
    expect(unexpectedAuthFailures()).toEqual([]);
    clearTraces();

    await page.evaluate(async () => {
      const response = await fetch('/v1/stats', { headers: { Authorization: 'Bearer data-token' } });
      const body = await response.json();
      (window as any).CortexDashboardReports.renderRequestIssue({
        ...body,
        http_status: response.status,
      }, 'stats');
    });
    await expect(page.locator('#error-report')).toContainText('Request');
    await expect(page.locator('#error-report')).toContainText('stats');
    await expect(page.locator('#error-report')).toContainText('HTTP 403');
    await expect(page.locator('#error-report')).toContainText('Use an admin token');
    failedAuthChecks.length = 0;

    await page.getByRole('button', { name: 'Clear' }).click();
    setRequestAuthToken('adm');
    await page.locator('#token').fill('adm');
    await page.getByRole('button', { name: 'Apply' }).click();
    await page.waitForTimeout(300);

    await expect(page.locator('#session-role')).toContainText('Access level: admin');
    await expect(page.locator('#permission-report')).toContainText('Admin access');
    await expect(page.locator('a[data-route="storage"]')).toBeVisible();
    expect(unexpectedAuthFailures()).toEqual([]);
    clearTraces();

    await page.getByRole('link', { name: 'Overview' }).click();
    await expect(page).toHaveURL(/\/dashboard\/overview$/);
    await expect(page.locator('#overview button[data-action="stats"]')).toBeVisible();

    await page.getByRole('link', { name: 'Cluster' }).click();
    await expect(page).toHaveURL(/\/dashboard\/cluster$/);
    await expect(page.locator('#cluster button[data-action="ann-metrics"]')).toBeVisible();

    await page.getByRole('link', { name: 'ANN' }).click();
    await expect(page).toHaveURL(/\/dashboard\/ann-eval$/);
    await expect(page.locator('#ann-eval button[data-action="ann-metrics"]')).toBeVisible();

    await page.getByRole('link', { name: 'Storage' }).click();
    await expect(page).toHaveURL(/\/dashboard\/storage$/);

    // Final phase should be fully elevated; only explicit HTTP 403s should be impossible now.
    expect(consoleErrors).toEqual([]);
    expect(unexpectedAuthFailures()).toEqual([]);
    expect(failedAuthChecks).toEqual([]);
  } finally {
    server.kill('SIGTERM');
    rmSync(root, { recursive: true, force: true });
  }
});

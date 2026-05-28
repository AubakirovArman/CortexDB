import { test, expect } from '@playwright/test';

const port = process.env.CORTEX_PORT || '8090';
const baseURL = `http://127.0.0.1:${port}`;

test('dashboard serves HTML with correct title', async ({ page }) => {
  await page.goto(`${baseURL}/dashboard`);
  await expect(page).toHaveTitle(/CortexDB Console/);
});

test('health check button is visible', async ({ page }) => {
  await page.goto(`${baseURL}/dashboard`);
  const healthBtn = page.locator('button:has-text("Health")');
  await expect(healthBtn).toBeVisible();
});

test('search smoke test', async ({ page }) => {
  await page.goto(`${baseURL}/dashboard`);

  // Put a cell with searchable content
  await page.locator('button[data-tab="cells"]').click();
  await page.locator('#cell-op').selectOption('put');
  await page.locator('#cell-payload').fill('scope=default\nstatus=ready\ntype=fact\nsource=e2e\n\nSolar budget approved');
  await page.locator('#cell-form button[type="submit"]').click();
  await expect(page.locator('#output')).toContainText('"seq"');

  // Search for the content
  await page.locator('button[data-tab="search"]').click();
  await page.locator('#search-query').fill('Solar');
  await page.locator('#search-form button[type="submit"]').click();
  await expect(page.locator('#output')).toContainText('"results"');
});

test('context smoke test', async ({ page }) => {
  await page.goto(`${baseURL}/dashboard`);

  // Put a cell with scope metadata
  await page.locator('button[data-tab="cells"]').click();
  await page.locator('#cell-op').selectOption('put');
  await page.locator('#cell-payload').fill('scope=default\nstatus=ready\ntype=fact\nsource=e2e\n\nWind Farm status ready');
  await page.locator('#cell-form button[type="submit"]').click();
  await expect(page.locator('#output')).toContainText('"seq"');

  // Build context pack
  await page.locator('button[data-tab="context"]').click();
  await page.locator('#context-scope').fill('default');
  await page.locator('#context-query').fill('RETRIEVE CONTEXT FOR TASK "wind" IN BRAIN default WHERE space = default AND status = "ready" LIMIT 10 CANDIDATES;');
  await page.locator('#context-form button[type="submit"]').click();
  await expect(page.locator('#output')).toContainText('"cells"');
});

test('verify smoke test', async ({ page }) => {
  await page.goto(`${baseURL}/dashboard`);

  // Put a cell with a fact
  await page.locator('button[data-tab="cells"]').click();
  await page.locator('#cell-op').selectOption('put');
  await page.locator('#cell-payload').fill('scope=default\nstatus=ready\ntype=fact\nsource=e2e\n\nSolar Plant budget is 1.2B KZT');
  await page.locator('#cell-form button[type="submit"]').click();
  await expect(page.locator('#output')).toContainText('"seq"');

  // Verify the fact
  await page.locator('button[data-tab="verify"]').click();
  await page.locator('#verify-scope').fill('default');
  await page.locator('#verify-query').fill('VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN default;');
  await page.locator('#verify-form button[type="submit"]').click();
  await expect(page.locator('#output')).toContainText('"fact"');
});

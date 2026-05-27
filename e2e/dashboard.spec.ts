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

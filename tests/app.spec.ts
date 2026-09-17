import { expect, test } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

test('add, edit, favorite, and remove a connection without losing the original', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'A little closer.' })).toBeVisible();
  await page.getByRole('button', { name: 'Add connection', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByLabel('Connection name').fill('Build server');
  await dialog.getByLabel('Hostname or IP address').fill('build.example.test');
  await dialog.getByLabel('Username').fill('dev');
  await dialog.getByLabel('Port', { exact: true }).fill('2222');
  await dialog.getByRole('button', { name: 'Add connection', exact: true }).click();
  const server = page.getByRole('article', { name: 'Build server', exact: true });
  await expect(server).toContainText('dev@build.example.test:2222');
  await server.getByRole('button', { name: 'Make Build server your favorite' }).click();
  await expect(server.getByText('FAVORITE', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Edit Build server' }).click();
  await dialog.getByLabel('Connection name').fill('Builder');
  await dialog.getByRole('button', { name: 'Save changes' }).click();
  await expect(page.getByRole('article', { name: 'Builder', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Edit Builder' }).click();
  await dialog.getByRole('button', { name: 'Remove connection', exact: true }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Remove connection', exact: true }).click();
  await expect(page.getByRole('article', { name: 'Builder', exact: true })).toHaveCount(0);
  await expect(page.getByRole('article', { name: 'Zbook Studio', exact: true })).toContainText('FAVORITE');
});

test('pet preferences retain state between views', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Your pet', exact: true }).click();
  const visibility = page.getByRole('switch', { name: 'Desktop companion' });
  await expect(visibility).toBeChecked();
  await visibility.click();
  await expect(visibility).not.toBeChecked();
  await page.getByRole('switch', { name: 'Quiet movements' }).click();
  await expect(page.locator('body')).toHaveClass(/reduce-motion/);
  await page.getByRole('button', { name: 'Connections', exact: false }).first().click();
  await page.getByRole('button', { name: 'Your pet', exact: true }).click();
  await expect(visibility).not.toBeChecked();
  await expect(page.getByRole('switch', { name: 'Quiet movements' })).toBeChecked();
});

test('preview clearly reports that it cannot open an SSH session', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Connect', exact: true }).click();
  await expect(page.getByRole('status')).toContainText('This is a browser preview');
  await expect(page.getByRole('button', { name: 'Connect', exact: true })).toBeEnabled();
});

test('connection names are displayed as text, not markup', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Add connection', exact: true }).click();
  const dialog = page.getByRole('dialog');
  const name = '<img src=x onerror=alert(1)>';
  await dialog.getByLabel('Connection name').fill(name);
  await dialog.getByLabel('Hostname or IP address').fill('example.test');
  await dialog.getByRole('button', { name: 'Add connection', exact: true }).click();
  await expect(page.getByRole('heading', { name, exact: true })).toBeVisible();
  await expect(page.locator('article img')).toHaveCount(0);
});

test('empty state and cancel keep destructive actions reversible', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Edit Zbook Studio' }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Remove connection', exact: true }).click();
  await page.getByRole('button', { name: 'Keep connection' }).click();
  await expect(page.getByRole('article')).toHaveCount(1);
  await page.getByRole('button', { name: 'Edit Zbook Studio' }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Remove connection', exact: true }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Remove connection', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Somewhere to go?' })).toBeVisible();
});

test('minimum supported window has no horizontal overflow', async ({ page }) => {
  await page.setViewportSize({ width: 700, height: 580 });
  await page.goto('/');
  for (const view of ['Connections', 'Your pet', 'Preferences']) {
    await page.getByRole('navigation').getByRole('button', { name: new RegExp(view) }).click();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
    expect(overflow).toBe(false);
  }
});

test('pet mode has a transparent background and an accessible launcher', async ({ page }) => {
  await page.goto('/?window=pet');
  await expect(page.getByRole('button', { name: /Droplet: click to open connections/ })).toBeVisible();
  await expect(page.locator('body')).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  await page.getByRole('button', { name: /Droplet: click to open connections/ }).click();
  await expect(page.getByRole('heading', { name: 'A little closer.' })).toBeVisible();
});

test('dragging the pet does not accidentally open the launcher', async ({ page }) => {
  await page.goto('/?window=pet');
  const button = page.getByRole('button', { name: /Droplet: click to open connections/ });
  const bounds = await button.boundingBox();
  if (!bounds) throw new Error('Pet did not render');
  await page.mouse.move(bounds.x + 50, bounds.y + 70);
  await page.mouse.down();
  await page.mouse.move(bounds.x + 80, bounds.y + 85, { steps: 8 });
  await page.mouse.up();
  await expect(button).toBeVisible();
  await page.waitForTimeout(600); // Longer than the single-click timer; this guards against an accidental launch.
  await expect(page).toHaveURL(/window=pet/);
});

test('main views and the connection form pass automated accessibility checks', async ({ page }) => {
  await page.goto('/');
  for (const view of ['Connections', 'Your pet', 'Preferences']) {
    await page.getByRole('navigation').getByRole('button', { name: new RegExp(view) }).click();
    const result = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(result.violations, `${view}: ${JSON.stringify(result.violations.map(v => ({ id: v.id, targets: v.nodes.map(n => n.target) })))}`).toEqual([]);
  }
  await page.getByRole('navigation').getByRole('button', { name: /Connections/ }).click();
  await page.getByRole('button', { name: 'Add connection', exact: true }).click();
  const result = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
  expect(result.violations).toEqual([]);
});

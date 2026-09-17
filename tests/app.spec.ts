import { expect, test } from './fixtures';
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
  await expect(page.getByRole('status')).toContainText('This action requires the desktop app.');
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
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.getByRole('article')).toHaveCount(1);
  await page.getByRole('button', { name: 'Edit Zbook Studio' }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Remove connection', exact: true }).click();
  await page.getByRole('dialog').getByRole('button', { name: 'Remove connection', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'No connections to show' })).toBeVisible();
});

test('minimum supported window has no horizontal overflow', async ({ page }) => {
  await page.setViewportSize({ width: 700, height: 580 });
  await page.goto('/');
  for (const view of ['Connections', 'Your pet', 'Security', 'Preferences']) {
    await page.getByRole('navigation').getByRole('button', { name: new RegExp(view) }).click();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
    expect(overflow).toBe(false);
  }
});

test('pet mode has a transparent background and an accessible launcher', async ({ page }) => {
  await page.goto('/?window=pet');
  await expect(page.getByRole('button', { name: /Droplet: open connections/ })).toBeVisible();
  await expect(page.locator('body')).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  await expect(page.locator('.pet-tooltip')).toHaveText('Double-click → Zbook Studio');
});

test('dragging the pet does not accidentally open the launcher', async ({ page, ipcRequests }) => {
  await page.goto('/?window=pet');
  const button = page.getByRole('button', { name: /Droplet: open connections/ });
  const bounds = await button.boundingBox();
  if (!bounds) throw new Error('Pet did not render');
  await page.mouse.move(bounds.x + 50, bounds.y + 70);
  await page.mouse.down();
  await page.mouse.move(bounds.x + 80, bounds.y + 85, { steps: 8 });
  await page.mouse.up();
  await expect(button).toBeVisible();
  await page.waitForTimeout(600); // Longer than the single-click timer; this guards against an accidental launch.
  await expect(page).toHaveURL(/window=pet/);
  expect(ipcRequests.some(call => call.request.type === 'beginDrag')).toBe(true);
  expect(ipcRequests.some(call => call.request.type === 'endDrag')).toBe(true);
  expect(ipcRequests.some(call => ['openLauncher', 'connectFavorite'].includes(call.request.type))).toBe(false);
});

test('main views and the connection form pass automated accessibility checks', async ({ page }) => {
  await page.goto('/');
  for (const view of ['Connections', 'Your pet', 'Security', 'Preferences']) {
    await page.getByRole('navigation').getByRole('button', { name: new RegExp(view) }).click();
    const result = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
    expect(result.violations, `${view}: ${JSON.stringify(result.violations.map(v => ({ id: v.id, targets: v.nodes.map(n => n.target) })))}`).toEqual([]);
  }
  await page.getByRole('navigation').getByRole('button', { name: /Connections/ }).click();
  await page.getByRole('button', { name: 'Add connection', exact: true }).click();
  const result = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
  expect(result.violations).toEqual([]);
});

test('Rust launch lock controls connect buttons and survives a renderer reload', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Security', exact: true }).click();
  await page.getByRole('switch', { name: 'Pause SSH launches' }).click();
  await expect(page.getByRole('switch', { name: 'Pause SSH launches' })).toBeChecked();
  await page.reload();
  await expect(page.getByRole('button', { name: 'Paused', exact: true })).toBeDisabled();
  await page.getByRole('button', { name: 'Security', exact: true }).click();
  await expect(page.getByText('launches paused', { exact: true })).toBeVisible();
  await page.getByRole('switch', { name: 'Pause SSH launches' }).click();
  await page.getByRole('navigation').getByRole('button', { name: /Connections/ }).click();
  await expect(page.getByRole('button', { name: 'Connect', exact: true })).toBeEnabled();
});

test('raw form values are validated by Rust', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Add connection', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByLabel('Connection name').fill('Bad port');
  await dialog.getByLabel('Hostname or IP address').fill('test.local');
  await dialog.getByLabel('Port', { exact: true }).fill('not-a-port');
  await dialog.getByRole('button', { name: 'Add connection', exact: true }).click();
  await expect(dialog.getByRole('alert')).toContainText('Enter a port from 1 to 65535');
  await expect(page.getByRole('article')).toHaveCount(1);
});

test('import rejects remote commands in the Rust parser', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Add connection', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('button', { name: 'Paste SSH command' }).click();
  await dialog.getByLabel('Connection name').fill('Unsafe import');
  await dialog.locator('textarea[name=command]').fill('ssh test.local touch /tmp/should-never-run');
  await dialog.getByRole('button', { name: 'Import connection' }).click();
  await expect(dialog.getByRole('alert')).not.toBeEmpty();
  await expect(page.getByRole('article')).toHaveCount(1);
});

test('diagnostics render Rust results without implying an authenticated session', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Check', exact: true }).click();
  const report = page.getByRole('region', { name: 'Connection check results' });
  await expect(report).toContainText('Checks completed');
  await expect(report).toContainText('Checks do not authenticate');
  await expect(report).toContainText('No network connection was made');
});

test('search comes from the Rust view model', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('searchbox').fill('DOES-NOT-EXIST');
  await expect(page.getByRole('article')).toHaveCount(0);
  await page.getByRole('searchbox').fill('STUDIO');
  await expect(page.getByRole('article', { name: 'Zbook Studio', exact: true })).toBeVisible();
});


test('pet double-click requests only the Rust-owned favorite', async ({ page, ipcRequests }) => {
  await page.goto('/?window=pet');
  await page.getByRole('button', { name: 'Droplet: open connections' }).dblclick();
  await page.waitForTimeout(400);
  expect(ipcRequests.filter(call => call.request.type === 'connectFavorite')).toEqual([{ surface: 'pet', request: { type: 'connectFavorite' } }]);
  expect(ipcRequests.some(call => call.request.type === 'openLauncher')).toBe(false);
});

test('pet single-click requests the launcher without launching SSH', async ({ page, ipcRequests }) => {
  await page.goto('/?window=pet');
  await page.getByRole('button', { name: 'Droplet: open connections' }).click();
  await expect.poll(() => ipcRequests.filter(call => call.request.type === 'openLauncher').length).toBe(1);
  expect(ipcRequests.some(call => call.request.type === 'connectFavorite')).toBe(false);
});

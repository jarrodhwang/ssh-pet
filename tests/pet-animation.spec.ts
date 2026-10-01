import { expect, test } from './fixtures';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';

async function selectPikachu(page: import('@playwright/test').Page) {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.goto('/');
  await page.getByRole('button', { name: 'Your pet', exact: true }).click();
  await page.getByRole('group', { name: 'Pet themes', exact: true }).getByRole('button', { name: 'Pokémon', exact: true }).click();
  await page.getByRole('button', { name: 'Select Pikachu', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Select Pikachu', exact: true })).toHaveAttribute('aria-pressed', 'true');
}

test('all shipped character images match their recorded source preparation', async () => {
  const manifest = JSON.parse(readFileSync('public/pets/sources.json', 'utf8')) as { assets: { name: string; frameCount: number; durationMs: number; sourceSha256: string; poster: string; animation: string | null; files: Record<string, { sha256: string; bytes: number }> }[] };
  for (const name of ['remy', 'emile', 'pikachu', 'eevee', 'bulbasaur', 'charmander', 'squirtle', 'jigglypuff', 'snoopy', 'woodstock', 'belle', 'remy-cheese', 'remy-strawberry', 'emile-snack', 'snoopy-dance']) {
    const asset = manifest.assets.find(a => a.name === name)!;
    expect(asset, name).toBeTruthy();
  }
  for (const asset of manifest.assets) {
    expect(asset.sourceSha256).toMatch(/^[a-f0-9]{64}$/);
    for (const [file, record] of Object.entries(asset.files)) {
      const bytes = readFileSync(resolve('public/pets', file));
      expect(bytes.length).toBe(record.bytes);
      expect(createHash('sha256').update(bytes).digest('hex')).toBe(record.sha256);
    }
    if (!['remy', 'emile'].includes(asset.name)) expect(asset.frameCount).toBeGreaterThan(1);
  }
  for (const name of ['pikachu', 'eevee', 'bulbasaur', 'charmander', 'squirtle', 'jigglypuff', 'snoopy', 'woodstock', 'belle']) {
    const quiet = manifest.assets.find(a => a.name === `${name}-quiet`)!;
    expect(quiet, `${name} has a gentle animation`).toBeTruthy();
    expect(quiet.frameCount).toBeGreaterThan(1);
    expect(quiet.durationMs).toBeGreaterThanOrEqual(5000);
    expect(readFileSync(resolve('public', quiet.animation!.slice(1))).includes(Buffer.from('ANIM'))).toBe(true);
  }
});

test('original art loads for every pet and selection cards never decode animations', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Your pet', exact: true }).click();
  for (const theme of ['Remy', 'Pokémon', 'Snoopy']) {
    await page.getByRole('group', { name: 'Pet themes', exact: true }).getByRole('button', { name: theme, exact: true }).click();
    for (const button of await page.locator('.pet-option').all()) {
      const image = button.locator('img');
      await expect(image).toHaveAttribute('src', /\/pets\/[a-z-]+-poster\.webp$/);
      await expect.poll(() => image.evaluate((img: HTMLImageElement) => img.complete && img.naturalWidth > 0)).toBe(true);
      await button.click();
      await expect.poll(() => page.locator('.pet-stage img.character-image').evaluate((img: HTMLImageElement) => img.complete && img.naturalWidth > 0)).toBe(true);
      await expect(page.locator('.pet-stage .character-art')).toHaveAttribute('data-motion', 'still');
      await expect(page.locator('[data-asset-error="true"]')).toHaveCount(0);
    }
  }
});

test('Pikachu previews replay each move locally, return to idle, and retain SSH isolation', async ({ page, ipcRequests }) => {
  await selectPikachu(page);
  await page.clock.install();
  const art = page.locator('.pet-stage .character-art');
  await expect(art).toHaveAttribute('data-motion', 'playing');
  await expect(art.locator('img')).toHaveAttribute('src', '/pets/pikachu-idle.webp');
  for (const [label, action, duration] of [['Electricity', 'spark', 3200], ['Volt Tackle', 'volt-tackle', 3600], ['Iron Tail', 'iron-tail', 3000]] as const) {
    await page.getByRole('button', { name: `Preview ${label}`, exact: true }).click();
    await expect(art).toHaveAttribute('data-action', action);
    await expect(art.locator('.pet-effects')).toBeVisible();
    await page.clock.runFor(duration + 100);
    await expect(art).toHaveAttribute('data-action', 'idle');
  }
  await page.getByRole('button', { name: 'Preview Electricity', exact: true }).click();
  await expect(art).toHaveAttribute('data-action', 'spark');
  await page.getByRole('switch', { name: 'Quiet movements' }).click();
  await expect(art).toHaveAttribute('data-motion', 'gentle');
  await expect(art).toHaveAttribute('data-action', 'idle');
  await expect(art.locator('img')).toHaveAttribute('src', '/pets/pikachu-quiet.webp');
  await expect(page.getByRole('button', { name: 'Preview Volt Tackle', exact: true })).toBeEnabled();
  await page.clock.runFor(80000);
  await expect(art).toHaveAttribute('data-action', 'idle');
  await expect(art).toHaveAttribute('data-motion', 'gentle');
  await page.getByRole('button', { name: 'Preview Volt Tackle', exact: true }).click();
  await expect(art).toHaveAttribute('data-action', 'volt-tackle');
  expect(ipcRequests.some(call => ['connect', 'connectFavorite', 'openLauncher'].includes(call.request.type))).toBe(false);
});

test('desktop pet scheduler cycles moves, respects system quiet mode, and pauses while held', async ({ page, ipcRequests }) => {
  await selectPikachu(page);
  await page.clock.install();
  await page.goto('/?window=pet');
  const art = page.locator('.character-art');
  const button = page.getByRole('button', { name: 'Pikachu: open connections', exact: true });
  await expect(art).toHaveAttribute('data-motion', 'playing');
  await page.clock.runFor(39000);
  // Observe the bounded automatic cycle without skipping a short move.
  let found = false;
  const seen = new Set<string>();
  for (let i = 0; i < 110; i++) {
    seen.add((await art.getAttribute('data-action'))!);
    if (await art.getAttribute('data-action') === 'volt-tackle') { found = true; break; }
    await page.clock.runFor(1000);
  }
  expect(seen.has('spark')).toBe(true);
  expect(found).toBe(true);
  const bounds = await button.boundingBox();
  if (!bounds) throw new Error('Pet did not render');
  await page.mouse.move(bounds.x + 100, bounds.y + 130);
  await page.mouse.down();
  await expect(art).toHaveAttribute('data-motion', 'still');
  await expect(art.locator('img')).toHaveAttribute('src', '/pets/pikachu-idle-poster.webp');
  await page.mouse.move(bounds.x + 120, bounds.y + 140);
  await page.mouse.up();
  await expect(art).toHaveAttribute('data-motion', 'playing');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await expect(art).toHaveAttribute('data-motion', 'still');
  await expect(art).toHaveAttribute('data-action', 'idle');
  await page.clock.runFor(30000);
  await expect(art).toHaveAttribute('data-action', 'idle');
  expect(ipcRequests.some(call => ['openLauncher', 'connectFavorite'].includes(call.request.type))).toBe(false);
});

test('Quiet movements keeps a gentle animated pet and makes automatic big moves rarer', async ({ page, ipcRequests }) => {
  await selectPikachu(page);
  await page.getByRole('switch', { name: 'Quiet movements' }).click();
  await expect(page.getByRole('switch', { name: 'Quiet movements' })).toBeChecked();
  // Install before mounting the pet so its first long timeout uses the test clock.
  await page.clock.install();
  await page.goto('/?window=pet');
  const art = page.locator('.character-art');
  await expect(art).toHaveAttribute('data-motion', 'gentle');
  await expect(art.locator('img')).toHaveAttribute('src', '/pets/pikachu-quiet.webp');
  await page.clock.runFor(85000);
  await expect(art).toHaveAttribute('data-action', 'idle');
  let found = false;
  for (let i = 0; i < 70; i++) {
    if (await art.getAttribute('data-action') === 'spark') { found = true; break; }
    await page.clock.runFor(1000);
  }
  expect(found, 'Quiet still allows an occasional big move').toBe(true);
  await page.clock.runFor(3300);
  await expect(art).toHaveAttribute('data-action', 'idle');
  await expect(art).toHaveAttribute('data-motion', 'gentle');
  await expect(art.locator('img')).toHaveAttribute('src', '/pets/pikachu-quiet.webp');
  expect(ipcRequests.some(call => ['connect', 'connectFavorite', 'openLauncher'].includes(call.request.type))).toBe(false);
});

test('a slow pet save protects the selected pet while Quiet movements changes', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.goto('/');
  await page.getByRole('button', { name: 'Your pet', exact: true }).click();
  await page.getByRole('group', { name: 'Pet themes', exact: true }).getByRole('button', { name: 'Pokémon', exact: true }).click();
  await page.evaluate(() => {
    const original = window.__DROPLET_TEST_INVOKE__!;
    let release!: () => void;
    const saved = new Promise<void>(resolve => { release = resolve; });
    (window as Window & { releasePetSave: () => void }).releasePetSave = release;
    window.__DROPLET_TEST_INVOKE__ = async (surface, request) => {
      if ((request as { type: string }).type === 'preferences') await saved;
      return original(surface, request);
    };
  });
  await page.getByRole('button', { name: 'Select Pikachu', exact: true }).click();
  const quiet = page.getByRole('switch', { name: 'Quiet movements' });
  await expect(quiet).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Select Eevee', exact: true })).toBeDisabled();
  await page.evaluate(() => (window as Window & { releasePetSave: () => void }).releasePetSave());
  await expect(quiet).toBeEnabled();
  await expect(page.getByRole('button', { name: 'Select Pikachu', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await quiet.click();
  await expect(quiet).toBeChecked();
  await page.goto('/?window=pet');
  await expect(page.locator('.character-art')).toHaveAttribute('data-pet', 'pikachu');
  await expect(page.locator('.character-art')).toHaveAttribute('data-motion', 'gentle');
});

test('Remy food and Snoopy dance use original clips, and changing pets clears a running clip', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'no-preference' });
  await page.goto('/');
  await page.getByRole('button', { name: 'Your pet', exact: true }).click();
  await page.getByRole('group', { name: 'Pet themes', exact: true }).getByRole('button', { name: 'Remy', exact: true }).click();
  await page.getByRole('button', { name: 'Select Remy', exact: true }).click();
  const art = page.locator('.pet-stage .character-art');
  for (const [name, src] of [['Cheese', 'remy-cheese'], ['Strawberry', 'remy-strawberry']]) {
    await page.getByRole('button', { name: `Preview ${name}`, exact: true }).click();
    const image = art.locator('.pet-film img');
    await expect(image).toHaveAttribute('src', `/pets/${src}.webp`);
    await expect.poll(() => image.evaluate((img: HTMLImageElement) => img.complete && img.naturalWidth > 0)).toBe(true);
  }
  await page.getByRole('group', { name: 'Pet themes', exact: true }).getByRole('button', { name: 'Snoopy', exact: true }).click();
  await page.getByRole('button', { name: 'Select Snoopy', exact: true }).click();
  await expect(art).toHaveAttribute('data-action', 'idle');
  await expect(art.locator('.pet-film')).toHaveCount(0);
  await page.getByRole('button', { name: 'Preview Happy dance', exact: true }).click();
  await expect(art.locator('.pet-film img')).toHaveAttribute('src', '/pets/snoopy-dance.webp');
  await expect.poll(() => art.locator('.pet-film img').evaluate((img: HTMLImageElement) => img.complete && img.naturalWidth > 0)).toBe(true);
});

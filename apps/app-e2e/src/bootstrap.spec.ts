import { expect, test } from '@playwright/test';

import {
  RUSTY_ENGINE_REVISION,
  RUSTY_PROCGEN_REVISION,
} from '@rusty-roguelike/protocol';

test('real Rust host mounts the exact-pinned retained renderer shell', async ({
  page,
}, testInfo) => {
  await page.goto('/');
  await expect(
    page.getByRole('heading', { name: 'Rusty Roguelike' }),
  ).toBeVisible();
  await expect(
    page.locator('[data-renderer-backend="rusty-engine-three"]'),
  ).toHaveAttribute('data-renderer-status', 'ready');
  await expect(page.getByTestId('engine-revision')).toHaveText(
    RUSTY_ENGINE_REVISION,
  );
  await expect(page.getByTestId('procgen-revision')).toHaveText(
    RUSTY_PROCGEN_REVISION,
  );
  await expect(page.locator('canvas')).toBeVisible();

  const overflow = await page.evaluate(() => ({
    width: document.documentElement.scrollWidth,
    viewport: document.documentElement.clientWidth,
  }));
  expect(overflow.width).toBeLessThanOrEqual(overflow.viewport);

  await testInfo.attach(`bootstrap-${testInfo.project.name}.png`, {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });
});

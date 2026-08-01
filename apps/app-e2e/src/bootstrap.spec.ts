import { expect, test, type Locator, type Page } from '@playwright/test';

import {
  RUSTY_ENGINE_REVISION,
  RUSTY_PROCGEN_REVISION,
} from '@rusty-roguelike/protocol';

const ROUTE_TO_FIRST_ENCOUNTER = `
  right right right right forward forward right right forward forward
  right right right right right right right right right backward
  right right right right right right right right right right
  right right right right right right right right right right
  right right right right right right backward
  right right right right right right right right right right
  right right forward
`
  .trim()
  .split(/\s+/);

test('real Rust host supports the renderer-first expedition on desktop and mobile', async ({
  page,
  request,
}, testInfo) => {
  const pageErrors: string[] = [];
  const consoleErrors: string[] = [];
  const failedRequests: string[] = [];
  page.on('pageerror', (error) => pageErrors.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error') {
      consoleErrors.push(message.text());
    }
  });
  page.on('requestfailed', (failed) =>
    failedRequests.push(`${failed.url()} ${failed.failure()?.errorText}`),
  );

  const restarted = await request.post('/api/v1/session/restart');
  expect(restarted.ok()).toBe(true);
  if (testInfo.project.name === 'mobile-chromium') {
    await page.emulateMedia({ reducedMotion: 'reduce' });
  }
  await page.goto('/');

  const stage = page.locator('.stage[data-session-revision]');
  await expect(stage).toHaveAttribute('data-session-revision', '0');
  await expect(
    page.locator('[data-renderer-backend="rusty-engine-three"]'),
  ).toHaveAttribute('data-renderer-status', 'ready');
  await expect(
    page.getByRole('navigation', { name: 'Initiative order' }),
  ).toBeVisible();
  await expect(
    page.getByRole('region', { name: 'Available actions' }),
  ).toBeVisible();
  await expect(
    page.getByRole('navigation', { name: 'Movement and facing' }),
  ).toBeVisible();
  await expect(page.getByRole('region', { name: 'Rules log' })).toBeVisible();
  await expect(page.getByTestId('engine-revision')).toHaveText(
    RUSTY_ENGINE_REVISION,
  );
  await expect(page.getByTestId('procgen-revision')).toHaveText(
    RUSTY_PROCGEN_REVISION,
  );
  await expect(page.locator('canvas')).toBeVisible();

  const partyTrigger = page.getByRole('button', { name: 'Party' });
  await partyTrigger.click();
  await expect(partyTrigger).toBeFocused();
  await expect(partyTrigger).toHaveAttribute('aria-expanded', 'true');
  const party = page.getByRole('region', { name: 'Party quick view' });
  await expect(party).toContainText('Kestrel');
  await expect(party).toContainText('Vitality');
  await page.keyboard.press('Escape');
  await expect(party).toBeHidden();
  await expect(partyTrigger).toBeFocused();
  await expect(partyTrigger).toHaveAttribute('aria-expanded', 'false');
  const packsTrigger = page.getByRole('button', { name: 'Packs' });
  await packsTrigger.click();
  const packs = page.getByRole('region', { name: 'Field packs' });
  await expect(packs).toContainText('Shortbow');
  await page.getByRole('button', { name: 'Close panel' }).click();
  await expect(packsTrigger).toBeFocused();

  if (testInfo.project.name === 'desktop-chromium') {
    for (const step of ROUTE_TO_FIRST_ENCOUNTER) {
      await issueAndWait(
        page,
        stage,
        step === 'forward'
          ? 'Step forward'
          : step === 'backward'
            ? 'Step backward'
            : step === 'left'
              ? 'Step left'
              : 'Step right',
      );
    }
    await expect(stage).toHaveAttribute('data-visible-enemies', '1');
    await issueAndWait(page, stage, 'Turn right');
    await expect(stage).toHaveAttribute('data-visible-enemies', '2');

    const action = page.getByRole('button', { name: '1 · Arcane Bolt' });
    await expect(action).toBeEnabled();
    await action.click();
    await expect(action).toHaveAttribute('aria-pressed', 'true');
    await expect(
      page.getByRole('button', { name: 'Goblin Scrapper' }),
    ).toBeVisible();

    const beforePick = Number(
      await stage.getAttribute('data-session-revision'),
    );
    const canvas = page.locator('canvas');
    const bounds = await canvas.boundingBox();
    if (bounds === null) {
      throw new Error('renderer canvas has no browser bounds');
    }
    await page.mouse.click(
      bounds.x + bounds.width / 2,
      bounds.y + bounds.height / 2,
    );
    await expect(stage).toHaveAttribute(
      'data-session-revision',
      String(beforePick + 1),
    );

    const rulesLog = page.getByRole('region', { name: 'Rules log' });
    await expect(rulesLog.locator('details')).not.toHaveCount(0);
    expect(
      await rulesLog.evaluate(
        (element) =>
          element.scrollHeight - element.scrollTop - element.clientHeight,
      ),
    ).toBeLessThanOrEqual(2);
    const latest = rulesLog.locator('details').last();
    await latest.locator('summary').click();
    await expect(latest).toContainText(/d20 .* defense/);
  } else {
    await page.keyboard.press('e');
    await expect(stage).toHaveAttribute('data-session-revision', '1');
    await assertSeparated(
      page.getByRole('region', { name: 'Available actions' }),
      page.getByRole('navigation', { name: 'Movement and facing' }),
      page.getByRole('region', { name: 'Rules log' }),
    );
    const controls = page.locator('button:visible');
    for (let index = 0; index < (await controls.count()); index += 1) {
      const box = await controls.nth(index).boundingBox();
      expect(box?.height ?? 0).toBeGreaterThanOrEqual(44);
    }
  }

  const overflow = await page.evaluate(() => ({
    height: document.documentElement.scrollHeight,
    viewportHeight: document.documentElement.clientHeight,
    viewportWidth: document.documentElement.clientWidth,
    width: document.documentElement.scrollWidth,
  }));
  expect(overflow.width).toBeLessThanOrEqual(overflow.viewportWidth);
  expect(overflow.height).toBeLessThanOrEqual(overflow.viewportHeight);
  expect(pageErrors).toEqual([]);
  expect(failedRequests).toEqual([]);
  expect(consoleErrors).toEqual([]);

  await testInfo.attach(`expedition-${testInfo.project.name}.png`, {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });
});

async function issueAndWait(
  page: Page,
  stage: Locator,
  accessibleName: string,
): Promise<void> {
  const revision = Number(await stage.getAttribute('data-session-revision'));
  const button = page.getByRole('button', { name: accessibleName });
  await expect(button).toBeEnabled();
  await button.click();
  await expect(stage).toHaveAttribute(
    'data-session-revision',
    String(revision + 1),
  );
}

async function assertSeparated(
  left: Locator,
  middle: Locator,
  right: Locator,
): Promise<void> {
  const [leftBox, middleBox, rightBox] = await Promise.all([
    left.boundingBox(),
    middle.boundingBox(),
    right.boundingBox(),
  ]);
  if (leftBox === null || middleBox === null || rightBox === null) {
    throw new Error('responsive HUD panels must have browser bounds');
  }
  expect(leftBox.x + leftBox.width).toBeLessThanOrEqual(middleBox.x);
  expect(middleBox.x + middleBox.width).toBeLessThanOrEqual(rightBox.x);
}

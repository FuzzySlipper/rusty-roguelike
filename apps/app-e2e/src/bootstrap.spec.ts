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
  test.setTimeout(240_000);
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
    page.getByRole('heading', { name: 'Prepare the expedition' }),
  ).toBeVisible();
  await expect(
    page.getByRole('navigation', { name: 'Initiative order' }),
  ).toBeHidden();
  await equipParty(page, stage, testInfo.project.name === 'desktop-chromium');
  await expect(
    page.getByRole('button', { name: 'Begin expedition' }),
  ).toBeEnabled();
  await saveAndWait(page);
  await issueAndWait(page, stage, 'Begin expedition');
  await reopenAndWait(page);
  await expect(stage).toHaveAttribute('data-session-revision', '7');
  await expect(
    page.getByRole('heading', { name: 'Prepare the expedition' }),
  ).toBeVisible();
  await testInfo.attach(`preparation-${testInfo.project.name}.png`, {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });
  await issueAndWait(page, stage, 'Begin expedition');
  await expect(stage).toHaveAttribute('data-session-revision', '8');
  await expect(
    page.getByRole('navigation', { name: 'Initiative order' }),
  ).toBeVisible();
  const objective = page.getByRole('status', { name: 'Floor objective' });
  await expect(objective).toContainText('Purge the ember den');
  await expect(objective).toContainText('both dormant raiders');
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
  await saveAndWait(page);
  await issueAndWait(page, stage, 'Step right');
  await reopenAndWait(page);
  await expect(stage).toHaveAttribute('data-session-revision', '8');

  const partyTrigger = page.getByRole('button', { name: 'Party' });
  await partyTrigger.click();
  await expect(partyTrigger).toBeFocused();
  await expect(partyTrigger).toHaveAttribute('aria-expanded', 'true');
  const party = page.getByRole('region', { name: 'Party quick view' });
  await expect(party).toContainText('Kestrel');
  await expect(party).toContainText('Level 1');
  await expect(party).toContainText('XP');
  const kestrelPartyTab = party.getByRole('tab', { name: 'Kestrel' });
  await kestrelPartyTab.click();
  await expect(kestrelPartyTab).toHaveAttribute('aria-selected', 'true');
  await page.keyboard.press('ArrowRight');
  await expect(party.getByRole('tab', { name: 'Mira' })).toBeFocused();
  await expect(party).toContainText('Arcane Focus');
  await page.keyboard.press('Escape');
  await expect(party).toBeHidden();
  await expect(partyTrigger).toBeFocused();
  await expect(partyTrigger).toHaveAttribute('aria-expanded', 'false');
  const packsTrigger = page.getByRole('button', { name: 'Packs' });
  await packsTrigger.click();
  const packs = page.getByRole('region', { name: 'Field packs' });
  await packs.getByRole('tab', { name: 'Kestrel' }).click();
  await expect(packs).toContainText('Shortbow');
  await packs.getByRole('tab', { name: 'Mira' }).click();
  await expect(packs).toContainText('Focus Orb');
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
    const initiative = page.getByRole('navigation', {
      name: 'Initiative order',
    });
    await expect(initiative).toContainText('Goblin Scrapper');
    await expect(initiative).not.toContainText('Ember Watcher');
    const combatRevision = await stage.getAttribute('data-session-revision');
    await saveAndWait(page);
    await issueAndWait(page, stage, 'Turn right');
    await reopenAndWait(page);
    await expect(stage).toHaveAttribute(
      'data-session-revision',
      String(combatRevision),
    );
    await issueAndWait(page, stage, 'Turn right');
    await expect(stage).toHaveAttribute('data-visible-enemies', '2');
    await expect(initiative).not.toContainText('Ember Watcher');

    const action = page.getByRole('button', { name: /Mind Spike/u });
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

    expect(await settleFloorToVictory(page, stage, initiative)).toBe(true);
    await expect(stage).toHaveAttribute('data-session-outcome', 'victory');
    await expect(objective).toContainText('Ember den secured');
    await expect(objective).toContainText('floor is complete');
    await expect(rulesLog).toContainText(/was targeted/u);
    await expect(rulesLog).toContainText(/round-robin-living/u);

    const terminalRevision = await stage.getAttribute('data-session-revision');
    await saveAndWait(page);
    await reopenAndWait(page);
    await expect(stage).toHaveAttribute(
      'data-session-revision',
      String(terminalRevision),
    );
    await expect(stage).toHaveAttribute('data-session-outcome', 'victory');
    await expect(objective).toContainText('Ember den secured');
  } else {
    await page.keyboard.press('e');
    await expect(stage).toHaveAttribute('data-session-revision', '9');
    await assertVerticallySeparated(
      objective,
      page.getByRole('complementary', { name: 'Party vitality' }),
    );
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

const LOADOUT = [
  { member: 'Brann', item: 'Longsword', slot: 'Weapon' },
  { member: 'Brann', item: 'Scale Mail', slot: 'Body' },
  { member: 'Kestrel', item: 'Shortbow', slot: 'Weapon' },
  { member: 'Kestrel', item: 'Leather Armor', slot: 'Body' },
  { member: 'Mira', item: 'Ash Staff', slot: 'Weapon' },
  { member: 'Mira', item: 'Focus Orb', slot: 'Focus' },
  { member: 'Mira', item: 'Traveling Robes', slot: 'Body' },
] as const;

async function equipParty(
  page: Page,
  stage: Locator,
  dragAndDrop: boolean,
): Promise<void> {
  const stash = page.getByRole('region', { name: 'Shared stash' });
  for (const assignment of LOADOUT) {
    await page
      .getByRole('button', {
        name: new RegExp(`^${assignment.member} ·`, 'u'),
      })
      .click();
    const member = page.getByRole('region', {
      name: `${assignment.member} loadout`,
    });
    const item = stash.getByRole('button', {
      name: new RegExp(`^${assignment.item}`, 'u'),
    });
    const destination = member.getByRole('button', {
      name: `${assignment.slot}: empty`,
    });
    const revision = Number(await stage.getAttribute('data-session-revision'));
    if (dragAndDrop) {
      await item.dragTo(destination);
    } else {
      await item.click();
      await expect(item).toHaveAttribute('aria-pressed', 'true');
      await destination.click();
    }
    await expect(stage).toHaveAttribute(
      'data-session-revision',
      String(revision + 1),
    );
    await expect(
      member.getByRole('button', {
        name: `${assignment.slot}: ${assignment.item}`,
      }),
    ).toBeVisible();
  }
  await expect(stash).toContainText('0 / 32');
}

async function settleFloorToVictory(
  page: Page,
  stage: Locator,
  initiative: Locator,
): Promise<boolean> {
  const actions = page.getByRole('region', { name: 'Available actions' });
  let emberJoined = false;
  for (let activation = 0; activation < 80; activation += 1) {
    emberJoined ||=
      (await initiative.textContent())?.includes('Ember Watcher') ?? false;
    if ((await stage.getAttribute('data-session-outcome')) === 'victory') {
      return emberJoined;
    }
    const legalActions = actions.locator('button:enabled');
    if ((await legalActions.count()) > 0) {
      const revision = Number(
        await stage.getAttribute('data-session-revision'),
      );
      await legalActions.last().click();
      const target = actions.locator('.target-row button:enabled').first();
      await expect(target).toBeVisible();
      await target.click();
      await expect(stage).toHaveAttribute(
        'data-session-revision',
        String(revision + 1),
      );
    } else {
      await issueAndWait(page, stage, 'Turn right');
    }
  }
  throw new Error('bounded browser expedition did not reach victory');
}

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

async function saveAndWait(page: Page): Promise<void> {
  await persistenceRequestAndWait(
    page,
    'Save',
    '/api/v1/session/save',
    'Session saved.',
  );
}

async function reopenAndWait(page: Page): Promise<void> {
  await persistenceRequestAndWait(
    page,
    'Reopen',
    '/api/v1/session/reopen',
    'Saved session reopened.',
  );
}

async function persistenceRequestAndWait(
  page: Page,
  action: 'Reopen' | 'Save',
  pathname: string,
  notice: string,
): Promise<void> {
  const response = page.waitForResponse(
    (candidate) =>
      new URL(candidate.url()).pathname === pathname &&
      candidate.request().method() === 'POST',
  );
  await page.getByRole('button', { name: action, exact: true }).click();
  expect((await response).ok()).toBe(true);
  await expect(
    page.getByRole('button', { name: 'Save', exact: true }),
  ).toBeEnabled();
  await expect(
    page.getByRole('button', { name: 'Reopen', exact: true }),
  ).toBeEnabled();
  await expect(page.locator('.persistence-notice')).toHaveText(notice);
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

async function assertVerticallySeparated(
  top: Locator,
  bottom: Locator,
): Promise<void> {
  const [topBox, bottomBox] = await Promise.all([
    top.boundingBox(),
    bottom.boundingBox(),
  ]);
  if (topBox === null || bottomBox === null) {
    throw new Error('responsive status panels must have browser bounds');
  }
  expect(topBox.y + topBox.height).toBeLessThanOrEqual(bottomBox.y);
}

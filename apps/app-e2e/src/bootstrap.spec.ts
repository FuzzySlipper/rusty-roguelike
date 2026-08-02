import {
  expect,
  test,
  type Locator,
  type Page,
  type TestInfo,
} from '@playwright/test';

import {
  decodeSessionView,
  RUSTY_ENGINE_REVISION,
  RUSTY_PROCGEN_REVISION,
  type RelativeStep,
  type SessionCommandDto,
  type SessionView,
  type WorldCell,
} from '@rusty-roguelike/protocol';

const ENEMY_NAMES = [
  'Ash Skirmisher',
  'Cinder Eye',
  'Cinder Stalker',
  'Clinker Knife',
  'Coal Sentry',
  'Ember Watcher',
  'Ember Seer',
  'Flare Watcher',
  'Furnace Lookout',
  'Goblin Scrapper',
  'Ruin Scuttler',
  'Slag Cutpurse',
  'Slag Runner',
  'Soot Stalker',
  'Tunnel Runner',
] as const;

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
  test.setTimeout(420_000);
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
  const torchAsset = page.waitForResponse((response) =>
    response.url().endsWith('/assets/torch/medieval-torch.glb'),
  );
  await page.goto('/');
  expect((await torchAsset).ok()).toBe(true);

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
  await verifyReadyParty(
    page,
    stage,
    testInfo.project.name === 'desktop-chromium',
  );
  await expect(
    page.getByRole('button', { name: 'Begin expedition' }),
  ).toBeEnabled();
  await assertGameMenu(page);
  await restartAndWait(page, stage);
  const savedPreparationRevision = Number(
    await stage.getAttribute('data-session-revision'),
  );
  await saveAndWait(page);
  await issueAndWait(page, stage, 'Begin expedition');
  await reopenAndWait(page);
  await expect(stage).toHaveAttribute(
    'data-session-revision',
    String(savedPreparationRevision),
  );
  await expect(
    page.getByRole('heading', { name: 'Prepare the expedition' }),
  ).toBeVisible();
  await testInfo.attach(`preparation-${testInfo.project.name}.png`, {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });
  await issueAndWait(page, stage, 'Begin expedition');
  await expect(stage).toHaveAttribute(
    'data-session-revision',
    String(savedPreparationRevision + 1),
  );
  await expect(
    page.getByRole('navigation', { name: 'Initiative order' }),
  ).toBeVisible();
  const objective = page.getByRole('status', { name: 'Floor objective' });
  await expect(objective).toContainText('Purge the ember den');
  await expect(objective).toContainText('all fifteen dormant raiders');
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
  const beforeWait = await readSession(page);
  const waitingActor = beforeWait.decision?.actorEntityId;
  if (waitingActor === undefined) {
    throw new Error('expedition omitted its initial party wait decision');
  }
  const waitButton = page.getByRole('button', { name: 'Wait (Space)' });
  await expect(waitButton).toBeEnabled();
  if (testInfo.project.name === 'desktop-chromium') {
    await page.evaluate(() => {
      globalThis.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: ' ',
          code: 'Space',
          repeat: true,
          bubbles: true,
          cancelable: true,
        }),
      );
    });
    await expect(stage).toHaveAttribute(
      'data-session-revision',
      String(beforeWait.revision),
    );
    const afterRepeatedSpace = await readSession(page);
    expect(afterRepeatedSpace.revision).toBe(beforeWait.revision);
    expect(afterRepeatedSpace.log).toEqual(beforeWait.log);
    const partyTrigger = page.getByRole('button', { name: 'Party' });
    await partyTrigger.focus();
    await page.keyboard.press('Space');
    await expect(
      page.getByRole('region', { name: 'Party quick view' }),
    ).toBeVisible();
    await expect(stage).toHaveAttribute(
      'data-session-revision',
      String(beforeWait.revision),
    );
    await page.keyboard.press('Space');
    await expect(stage).toHaveAttribute(
      'data-session-revision',
      String(beforeWait.revision),
    );
    await page.keyboard.press('Escape');
    await expect(partyTrigger).toBeFocused();
    await page.evaluate(() => (document.activeElement as HTMLElement)?.blur());
    await page.keyboard.press('Space');
  } else {
    await waitButton.click();
  }
  await expect(stage).toHaveAttribute(
    'data-session-revision',
    String(beforeWait.revision + 1),
  );
  const afterWait = await readSession(page);
  expect(afterWait.latestReceipts[0]).toEqual({
    kind: 'partyWaited',
    actorEntityId: waitingActor,
  });
  expect(afterWait.current?.side).toBe('party');
  expect(afterWait.current?.entityId).not.toBe(waitingActor);
  expect(
    afterWait.log
      .filter((entry) => entry.revision === afterWait.revision)
      .map((entry) => entry.receipt),
  ).toEqual(afterWait.latestReceipts);
  const expeditionRevision = afterWait.revision;
  const minimap = page.locator('rr-minimap [role="img"]');
  const mapToolbar = page.locator('.map-toolbar');
  const initialView = await readSession(page);
  const initialDiscoveredCells = initialView.world.minimap.cells.length;
  const initialVisibleCells = initialView.world.minimap.cells.filter(
    (cell) => cell.visible,
  ).length;
  const initialVisibleEnemies = initialView.world.visibleActors.length;
  const initialVisibleTorches = initialView.world.scenePlacements.filter(
    (placement) => placement.content.kind === 'prop',
  ).length;
  const initialVisibleLights = initialView.world.scenePlacements.filter(
    (placement) => placement.content.kind === 'point_light',
  ).length;
  expect(initialVisibleEnemies).toBe(1);
  expect(initialVisibleLights).toBe(initialVisibleTorches);
  expect(initialView.world.cells.length).toBeGreaterThan(initialVisibleCells);
  const viewport = page.locator('[data-renderer-backend="rusty-engine-three"]');
  await expect(viewport).toHaveAttribute(
    'data-scene-cells',
    String(initialView.world.cells.length),
  );
  await expect(viewport).toHaveAttribute(
    'data-visible-torches',
    String(initialVisibleTorches),
  );
  await expect(viewport).toHaveAttribute(
    'data-visible-lights',
    String(initialVisibleLights),
  );
  await expect(viewport).toHaveAttribute(
    'data-lighting-world-default',
    'disabled',
  );
  await expect(viewport).toHaveAttribute(
    'data-lighting-viewmodel-default',
    'neutral',
  );
  await expect(viewport).toHaveAttribute(
    'data-retained-light-count',
    String(initialVisibleLights),
  );
  await expect(viewport).toHaveAttribute(
    'data-view-camera',
    'camera.dungeon-local-overview',
  );
  await expect(viewport).toHaveAttribute('data-view-target-count', '1');
  await expect(viewport).toHaveAttribute('data-view-target-revision', '1');
  await expect(viewport).toHaveAttribute(
    'data-view-target-size',
    testInfo.project.name === 'desktop-chromium' ? '256' : '128',
  );
  await expect(viewport).toHaveAttribute('data-view-target-status', 'current');
  const overviewMetrics = await analyzeLocalOverview(
    page,
    testInfo.project.name === 'mobile-chromium',
  );
  expect(overviewMetrics.distinctPixelRatio).toBeGreaterThan(0.05);
  expect(overviewMetrics.insetLuminanceRange).toBeGreaterThan(5);
  await testInfo.attach(`local-overview-${testInfo.project.name}.json`, {
    body: Buffer.from(JSON.stringify(overviewMetrics, null, 2)),
    contentType: 'application/json',
  });
  await testInfo.attach(`local-overview-${testInfo.project.name}.png`, {
    body: await page.locator('canvas').screenshot(),
    contentType: 'image/png',
  });
  if (testInfo.project.name === 'desktop-chromium') {
    const beforeResize = await readSession(page);
    const originalViewport = page.viewportSize();
    if (originalViewport === null)
      throw new Error('desktop viewport size is unavailable');
    await page.setViewportSize({ width: 600, height: 800 });
    await expect(viewport).toHaveAttribute('data-view-target-revision', '2');
    await expect(viewport).toHaveAttribute('data-view-target-size', '128');
    await expect(viewport).toHaveAttribute(
      'data-view-target-status',
      'current',
    );
    await page.setViewportSize(originalViewport);
    await expect(viewport).toHaveAttribute('data-view-target-revision', '3');
    await expect(viewport).toHaveAttribute('data-view-target-size', '256');
    await expect(viewport).toHaveAttribute(
      'data-view-target-status',
      'current',
    );
    const afterResize = await readSession(page);
    expect(afterResize.revision).toBe(beforeResize.revision);
    expect(afterResize.world.minimap).toEqual(beforeResize.world.minimap);
  }
  await expect(minimap).toHaveAttribute(
    'data-minimap-revision',
    String(expeditionRevision),
  );
  await expect(minimap).toHaveAttribute(
    'data-discovered-cells',
    String(initialDiscoveredCells),
  );
  await expect(minimap).toHaveAttribute(
    'data-visible-cells',
    String(initialVisibleCells),
  );
  await expect(minimap).toHaveAttribute(
    'data-visible-enemies',
    String(initialVisibleEnemies),
  );
  await expect(minimap).toHaveAttribute(
    'aria-label',
    new RegExp(
      `Party facing ${initialView.world.facing}.*${initialDiscoveredCells} discovered cells`,
      'u',
    ),
  );
  await expect(minimap.locator('.feature')).not.toHaveCount(0);
  await minimap.focus();
  await expect(minimap).toBeFocused();
  await assertAbove(mapToolbar, minimap);
  await assertTransparentGapReachesCanvas(page, mapToolbar, minimap);
  await testInfo.attach(`lit-scene-${testInfo.project.name}.png`, {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });

  await page.route(
    '**/api/v1/session/commands',
    async (route) =>
      route.fulfill({
        status: 409,
        contentType: 'application/json',
        body: JSON.stringify({
          code: 'session_stale',
          detail: 'Rejected minimap publication proof.',
        }),
      }),
    { times: 1 },
  );
  await page.getByRole('button', { name: 'Step right' }).click();
  await expect(page.getByRole('alert')).toContainText('session_stale');
  await expect
    .poll(
      () =>
        consoleErrors.filter((message) => message.includes('status of 409'))
          .length,
    )
    .toBe(1);
  consoleErrors.splice(
    consoleErrors.findIndex((message) => message.includes('status of 409')),
    1,
  );
  await expect(stage).toHaveAttribute(
    'data-session-revision',
    String(expeditionRevision),
  );
  await expect(minimap).toHaveAttribute(
    'data-minimap-revision',
    String(expeditionRevision),
  );
  await expect(minimap).toHaveAttribute(
    'data-discovered-cells',
    String(initialDiscoveredCells),
  );
  const beforeMenuFailure = await readSession(page);
  await page.route(
    '**/api/v1/session/reopen',
    async (route) =>
      route.fulfill({
        status: 409,
        contentType: 'application/json',
        body: JSON.stringify({
          code: 'session_load_failed',
          detail: 'Saved session could not be loaded for this proof.',
        }),
      }),
    { times: 1 },
  );
  const menuTrigger = page.locator('.map-toolbar').getByRole('button', {
    name: 'Menu',
    exact: true,
  });
  await menuTrigger.click();
  const failureMenu = page.getByRole('dialog', { name: 'Game menu' });
  const failureResponse = page.waitForResponse(
    (candidate) =>
      new URL(candidate.url()).pathname === '/api/v1/session/reopen' &&
      candidate.request().method() === 'POST',
  );
  await failureMenu.getByRole('button', { name: 'Load saved session' }).click();
  expect((await failureResponse).status()).toBe(409);
  await expect
    .poll(
      () =>
        consoleErrors.filter((message) => message.includes('status of 409'))
          .length,
    )
    .toBe(1);
  consoleErrors.splice(
    consoleErrors.findIndex((message) => message.includes('status of 409')),
    1,
  );
  await expect(failureMenu).toBeVisible();
  await expect(failureMenu.getByRole('alert')).toContainText(
    'session_load_failed',
  );
  await expect(failureMenu.getByRole('alert')).toContainText(
    'Saved session could not be loaded for this proof.',
  );
  await expect(stage).toHaveAttribute(
    'data-session-revision',
    String(beforeMenuFailure.revision),
  );
  expect(await readSession(page)).toEqual(beforeMenuFailure);
  await page.keyboard.press('Escape');
  await expect(failureMenu).toBeHidden();
  await expect(menuTrigger).toBeFocused();
  await saveAndWait(page);
  await issueAndWait(page, stage, 'Step right');
  await expect(minimap).toHaveAttribute(
    'data-minimap-revision',
    String(expeditionRevision + 1),
  );
  await reopenAndWait(page);
  await expect(stage).toHaveAttribute(
    'data-session-revision',
    String(expeditionRevision),
  );
  await expect(minimap).toHaveAttribute(
    'data-minimap-revision',
    String(expeditionRevision),
  );
  await expect(minimap).toHaveAttribute(
    'data-discovered-cells',
    String(initialDiscoveredCells),
  );

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
    const firstEnemy = await followRouteToFirstEncounter(page, stage);
    await expect(stage).toHaveAttribute('data-visible-enemies', '1');
    const encounterView = await readSession(page);
    expect(encounterView.world.minimap.cells.length).toBeGreaterThanOrEqual(
      initialDiscoveredCells,
    );
    await expect(minimap).toHaveAttribute(
      'data-discovered-cells',
      String(encounterView.world.minimap.cells.length),
    );
    await expect(minimap).toHaveAttribute('data-visible-enemies', '1');
    await expect(minimap.locator('svg text.enemy')).toHaveCount(1);
    const initiative = page.getByRole('navigation', {
      name: 'Initiative order',
    });
    await expect(initiative).toContainText(firstEnemy);
    const combatRevision = await stage.getAttribute('data-session-revision');
    await saveAndWait(page);
    await issueAndWait(page, stage, 'Turn right');
    await expect(minimap).toHaveAttribute('data-visible-enemies', '0');
    await expect(minimap.locator('svg text.enemy')).toHaveCount(0);
    await expect(minimap.locator('.cell.remembered')).not.toHaveCount(0);
    await reopenAndWait(page);
    await expect(stage).toHaveAttribute(
      'data-session-revision',
      String(combatRevision),
    );
    await expect(minimap).toHaveAttribute(
      'data-minimap-revision',
      String(combatRevision),
    );
    await expect(minimap).toHaveAttribute('data-visible-enemies', '1');
    for (let attempt = 0; attempt < 4; attempt += 1) {
      const actor = (await readSession(page)).world.visibleActors[0];
      if (actor !== undefined && actor.lateral === 0 && actor.depth > 0) {
        break;
      }
      await issueAndWait(
        page,
        stage,
        (actor?.lateral ?? 1) < 0 ? 'Turn left' : 'Turn right',
      );
    }

    const actionRow = page.locator('.action-row');
    for (let attempt = 0; attempt < 4; attempt += 1) {
      if (
        (await actionRow.locator('button[aria-pressed]:enabled').count()) > 0
      ) {
        break;
      }
      await issueAndWait(page, stage, 'Turn right');
    }
    const action = actionRow.locator('button[aria-pressed]:enabled').first();
    await expect(action).toBeEnabled();
    await action.click();
    await expect(action).toHaveAttribute('aria-pressed', 'true');
    await expect(page.getByRole('button', { name: firstEnemy })).toBeVisible();

    const beforePick = Number(
      await stage.getAttribute('data-session-revision'),
    );
    const canvas = page.locator('canvas');
    const bounds = await canvas.boundingBox();
    if (bounds === null) {
      throw new Error('renderer canvas has no browser bounds');
    }
    await clickRenderedTarget(
      page,
      bounds,
      beforePick,
      (await readSession(page)).world.visibleActors,
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

    expect(await settleFloorToVictory(page, stage)).toBe(true);
    await expect(stage).toHaveAttribute('data-session-outcome', 'victory');
    await expect(objective).toContainText('Ember den secured');
    await expect(objective).toContainText('floor is complete');
    await expect(minimap).toHaveAttribute('data-visible-enemies', '0');
    expect(Number(await minimap.getAttribute('data-discovered-cells'))).toBe(
      (await readSession(page)).world.minimap.cells.length,
    );
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
    await expect(minimap).toHaveAttribute(
      'data-minimap-revision',
      String(terminalRevision),
    );
  } else {
    const beforeTurn = Number(
      await stage.getAttribute('data-session-revision'),
    );
    await page.keyboard.press('e');
    await expect(stage).toHaveAttribute(
      'data-session-revision',
      String(beforeTurn + 1),
    );
    const turned = await readSession(page);
    const viewport = page.locator(
      '[data-renderer-backend="rusty-engine-three"]',
    );
    const torches = turned.world.scenePlacements.filter(
      (placement) => placement.content.kind === 'prop',
    ).length;
    const lights = turned.world.scenePlacements.filter(
      (placement) => placement.content.kind === 'point_light',
    ).length;
    expect(torches).toBeGreaterThan(0);
    expect(lights).toBe(torches);
    await assertAuthoredTorchLighting(page, turned, testInfo);
    await expect(viewport).toHaveAttribute(
      'data-visible-torches',
      String(torches),
    );
    await expect(viewport).toHaveAttribute(
      'data-visible-lights',
      String(lights),
    );
    await expect(viewport).toHaveAttribute(
      'data-scene-cells',
      String(turned.world.cells.length),
    );
    await assertVerticallySeparated(
      objective,
      page.getByRole('complementary', { name: 'Party vitality' }),
    );
    await assertAbove(mapToolbar, minimap);
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

test('renderer exposes a corrupt torch resource as a visible failure', async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== 'desktop-chromium');
  await page.route('**/assets/torch/medieval-torch.glb', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'model/gltf-binary',
      body: 'corrupt fixture asset',
    }),
  );
  await page.goto('/');
  const viewport = page.locator('[data-renderer-backend="rusty-engine-three"]');
  await expect(viewport).toHaveAttribute('data-renderer-status', 'error');
  await expect(viewport.getByRole('alert')).toContainText(
    /expected sha256:[0-9a-f]{64}, received sha256:[0-9a-f]{64}/u,
  );
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

async function verifyReadyParty(
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
    await expect(
      member.getByRole('button', {
        name: `${assignment.slot}: ${assignment.item}`,
      }),
    ).toBeVisible();
  }
  await expect(stash).toContainText('0 / 32');
  await expect(
    page.getByRole('button', { name: 'Begin expedition' }),
  ).toBeEnabled();

  await page.getByRole('button', { name: new RegExp('^Brann ·', 'u') }).click();
  const brann = page.getByRole('region', { name: 'Brann loadout' });
  const armor = brann.getByRole('button', { name: /^Scale Mail/u });
  await armor.click();
  const moveToStash = stash.getByRole('button', {
    name: 'Move selected to pack',
  });
  const unequipRevision = Number(
    await stage.getAttribute('data-session-revision'),
  );
  await moveToStash.click();
  await expect(stage).toHaveAttribute(
    'data-session-revision',
    String(unequipRevision + 1),
  );
  await expect(
    page.getByRole('button', { name: 'Begin expedition' }),
  ).toBeDisabled();

  const stashedArmor = stash.getByRole('button', { name: /^Scale Mail/u });
  const body = brann.getByRole('button', { name: 'Body: empty' });
  const equipRevision = Number(
    await stage.getAttribute('data-session-revision'),
  );
  if (dragAndDrop) {
    await stashedArmor.dragTo(body);
  } else {
    await stashedArmor.click();
    await expect(stashedArmor).toHaveAttribute('aria-pressed', 'true');
    await body.click();
  }
  await expect(stage).toHaveAttribute(
    'data-session-revision',
    String(equipRevision + 1),
  );
  await expect(
    brann.getByRole('button', { name: 'Body: Scale Mail' }),
  ).toBeVisible();
  await expect(stash).toContainText('0 / 32');
}

async function followRouteToFirstEncounter(
  page: Page,
  stage: Locator,
): Promise<string> {
  for (const step of ROUTE_TO_FIRST_ENCOUNTER) {
    const current = await readSession(page);
    if (current.world.visibleActors.length > 0) {
      break;
    }
    const label = stepLabel(step as RelativeStep);
    const button = page.getByRole('button', { name: label });
    if (await button.isEnabled()) {
      await issueAndWait(page, stage, label);
      continue;
    }
    const rotations =
      step === 'right' ? 1 : step === 'backward' ? 2 : step === 'left' ? 3 : 0;
    for (let rotation = 0; rotation < rotations; rotation += 1) {
      await issueAndWait(page, stage, 'Turn right');
    }
  }
  for (let attempt = 0; attempt < 4; attempt += 1) {
    const current = await readSession(page);
    const firstEnemy = current.world.visibleActors[0]?.name;
    const initiative = await page
      .getByRole('navigation', { name: 'Initiative order' })
      .textContent();
    if (firstEnemy !== undefined && initiative?.includes(firstEnemy) === true) {
      return firstEnemy;
    }
    await issueAndWait(page, stage, 'Turn right');
  }
  throw new Error('bounded route did not reveal and admit its first encounter');
}

async function settleFloorToVictory(
  page: Page,
  stage: Locator,
): Promise<boolean> {
  const joined = new Set<string>();
  const exploration: ExplorationDriver = {
    blockedTurns: new Map(),
    path: [],
    visited: new Set(),
  };
  let view = await readSession(page);
  for (let activation = 0; activation < 1_000; activation += 1) {
    for (const active of view.order) {
      if (active.side === 'opposition') {
        joined.add(active.name);
      }
    }
    for (const actor of view.world.visibleActors) {
      joined.add(actor.name);
    }
    if (view.outcome === 'victory') {
      await page.reload();
      await expect(stage).toHaveAttribute(
        'data-session-revision',
        String(view.revision),
      );
      return joined.size === ENEMY_NAMES.length;
    }
    const decision = view.decision;
    if (decision === null) {
      throw new Error('ongoing expedition omitted its party decision');
    }
    const action = decision.actions.find(
      (candidate) => candidate.legalTargetEntityIds.length > 0,
    );
    let command: SessionCommandDto;
    if (action !== undefined) {
      const targetEntityId = action.legalTargetEntityIds[0];
      if (targetEntityId === undefined) {
        throw new Error('legal action omitted its first target');
      }
      command = {
        kind: 'useAction',
        actorEntityId: decision.actorEntityId,
        expectedRevision: decision.expectedRevision,
        actionId: action.actionId,
        targetEntityId,
      };
    } else if (view.world.visibleActors.length > 0) {
      command = {
        kind: 'turnRight',
        actorEntityId: decision.actorEntityId,
        expectedRevision: decision.expectedRevision,
      };
    } else {
      const step = explorationStep(view, exploration);
      command =
        step === null
          ? {
              kind: 'turnRight',
              actorEntityId: decision.actorEntityId,
              expectedRevision: decision.expectedRevision,
            }
          : {
              kind: 'step',
              actorEntityId: decision.actorEntityId,
              expectedRevision: decision.expectedRevision,
              step,
            };
    }
    view = await postSessionCommand(page, command);
  }
  throw new Error(
    `bounded browser expedition stalled at round ${view.round}, party ${cellKey(view.world.minimap.party)}, facing ${view.world.facing}, visible ${view.world.visibleActors.map((actor) => actor.name).join(',')}, joined ${[...joined].join(',')}, visited ${exploration.visited.size}/${view.world.minimap.cells.length}`,
  );
}

async function readSession(page: Page): Promise<SessionView> {
  const response = await page.request.get('/api/v1/session');
  expect(response.ok()).toBe(true);
  return decodeSessionView(await response.json());
}

async function postSessionCommand(
  page: Page,
  command: SessionCommandDto,
): Promise<SessionView> {
  const response = await page.request.post('/api/v1/session/commands', {
    data: command,
  });
  expect(response.ok()).toBe(true);
  return decodeSessionView(await response.json());
}

interface ExplorationDriver {
  readonly blockedTurns: Map<string, number>;
  readonly path: WorldCell[];
  readonly visited: Set<string>;
}

function explorationStep(
  view: SessionView,
  driver: ExplorationDriver,
): RelativeStep | null {
  const origin = view.world.minimap.party;
  const originKey = cellKey(origin);
  driver.visited.add(originKey);
  if (driver.path.length === 0) {
    driver.path.push(origin);
  } else {
    const tail = driver.path.at(-1);
    if (tail === undefined) {
      throw new Error('nonempty exploration path omitted its tail');
    }
    if (cellKey(tail) !== originKey) {
      const existing = driver.path.findIndex(
        (cell) => cellKey(cell) === originKey,
      );
      if (existing >= 0) {
        driver.path.splice(existing + 1);
      } else {
        driver.path.push(origin);
      }
    }
  }
  if (view.decision === null) {
    return null;
  }
  const legal = view.decision.legalSteps.map((step) => ({
    step,
    destination: stepDestination(origin, view.world.facing, step),
  }));
  const unvisited = legal.find(
    ({ destination }) => !driver.visited.has(cellKey(destination)),
  );
  if (unvisited !== undefined) {
    driver.blockedTurns.delete(originKey);
    driver.visited.add(cellKey(unvisited.destination));
    driver.path.push(unvisited.destination);
    return unvisited.step;
  }
  const knownFloor = new Set(
    view.world.minimap.cells
      .filter((cell) => cell.terrain === 'floor')
      .map(cellKey),
  );
  const legalDestinations = new Set(
    legal.map(({ destination }) => cellKey(destination)),
  );
  const blockedUnvisitedNeighbor = cardinalNeighbors(origin).some(
    (cell) =>
      knownFloor.has(cellKey(cell)) &&
      !driver.visited.has(cellKey(cell)) &&
      !legalDestinations.has(cellKey(cell)),
  );
  const turns = driver.blockedTurns.get(originKey) ?? 0;
  if (blockedUnvisitedNeighbor && turns < 4) {
    driver.blockedTurns.set(originKey, turns + 1);
    return null;
  }
  driver.blockedTurns.delete(originKey);
  const parent = driver.path.at(-2);
  if (parent === undefined) {
    return null;
  }
  const backtrack = legal.find(
    ({ destination }) => cellKey(destination) === cellKey(parent),
  );
  if (backtrack === undefined) {
    return null;
  }
  driver.path.pop();
  return backtrack.step;
}

function stepDestination(
  origin: WorldCell,
  facing: SessionView['world']['facing'],
  step: RelativeStep,
): WorldCell {
  const forward =
    facing === 'north'
      ? { x: 0, y: -1 }
      : facing === 'east'
        ? { x: 1, y: 0 }
        : facing === 'south'
          ? { x: 0, y: 1 }
          : { x: -1, y: 0 };
  const right = { x: -forward.y, y: forward.x };
  const delta =
    step === 'forward'
      ? forward
      : step === 'backward'
        ? { x: -forward.x, y: -forward.y }
        : step === 'right'
          ? right
          : { x: -right.x, y: -right.y };
  return { x: origin.x + delta.x, y: origin.y + delta.y };
}

function cardinalNeighbors(cell: WorldCell): WorldCell[] {
  return [
    { x: cell.x, y: cell.y - 1 },
    { x: cell.x + 1, y: cell.y },
    { x: cell.x, y: cell.y + 1 },
    { x: cell.x - 1, y: cell.y },
  ];
}

function cellKey(cell: WorldCell): string {
  return `${cell.x},${cell.y}`;
}

function stepLabel(step: RelativeStep): string {
  return step === 'forward'
    ? 'Step forward'
    : step === 'backward'
      ? 'Step backward'
      : step === 'left'
        ? 'Step left'
        : 'Step right';
}

async function clickRenderedTarget(
  page: Page,
  bounds: { x: number; y: number; width: number; height: number },
  revision: number,
  visibleActors: SessionView['world']['visibleActors'],
): Promise<void> {
  for (const vertical of [0.5, 0.42, 0.58, 0.34, 0.66]) {
    for (const horizontal of [0.5, 0.35, 0.65, 0.2, 0.8]) {
      await page.mouse.click(
        bounds.x + bounds.width * horizontal,
        bounds.y + bounds.height * vertical,
      );
      await page.waitForTimeout(25);
      if (
        Number(
          await page
            .locator('.stage[data-session-revision]')
            .getAttribute('data-session-revision'),
        ) ===
        revision + 1
      ) {
        return;
      }
    }
  }
  throw new Error(
    `bounded RendererSurface target scan did not pick ${JSON.stringify(visibleActors)}`,
  );
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
    'Load saved session',
    '/api/v1/session/reopen',
    'Saved session reopened.',
  );
}

async function assertGameMenu(page: Page): Promise<void> {
  const trigger = page.getByRole('button', { name: 'Menu', exact: true });
  await trigger.click();
  const menu = page.getByRole('dialog', { name: 'Game menu' });
  await expect(menu).toBeVisible();
  await expect(
    menu.getByRole('button', { name: 'Close game menu' }),
  ).toBeFocused();
  await expect(
    menu.getByRole('button', { name: 'New / Restart expedition' }),
  ).toBeEnabled();
  await expect(
    menu.getByRole('button', { name: 'Save', exact: true }),
  ).toBeEnabled();
  await expect(
    menu.getByRole('button', { name: 'Load saved session' }),
  ).toBeEnabled();
  await expect(
    menu.getByRole('button', { name: 'Exit', exact: true }),
  ).toBeDisabled();
  await expect(menu).toContainText('native builds');
  await page.keyboard.press('Escape');
  await expect(menu).toBeHidden();
  await expect(trigger).toBeFocused();
}

async function restartAndWait(page: Page, stage: Locator): Promise<void> {
  const trigger = page.getByRole('button', { name: 'Menu', exact: true });
  await trigger.click();
  const menu = page.getByRole('dialog', { name: 'Game menu' });
  const response = page.waitForResponse(
    (candidate) =>
      new URL(candidate.url()).pathname === '/api/v1/session/restart' &&
      candidate.request().method() === 'POST',
  );
  await menu.getByRole('button', { name: 'New / Restart expedition' }).click();
  expect((await response).ok()).toBe(true);
  await expect(stage).toHaveAttribute('data-session-revision', '0');
  await expect(menu).toBeHidden();
  await expect(trigger).toBeFocused();
  await expect(
    page.locator('.persistence-notice, .map-persistence-notice'),
  ).toHaveText('New expedition started.');
}

async function persistenceRequestAndWait(
  page: Page,
  action: 'Load saved session' | 'Save',
  pathname: string,
  notice: string,
): Promise<void> {
  const response = page.waitForResponse(
    (candidate) =>
      new URL(candidate.url()).pathname === pathname &&
      candidate.request().method() === 'POST',
  );
  const trigger = page.getByRole('button', { name: 'Menu', exact: true });
  await trigger.click();
  const menu = page.getByRole('dialog', { name: 'Game menu' });
  await menu.getByRole('button', { name: action, exact: true }).click();
  expect((await response).ok()).toBe(true);
  await expect(menu).toBeHidden();
  await expect(trigger).toBeFocused();
  await expect(
    page.locator('.persistence-notice, .map-persistence-notice'),
  ).toHaveText(notice);
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

async function assertAbove(top: Locator, bottom: Locator): Promise<void> {
  const [topBox, bottomBox] = await Promise.all([
    top.boundingBox(),
    bottom.boundingBox(),
  ]);
  if (topBox === null || bottomBox === null) {
    throw new Error('map toolbar and minimap must have browser bounds');
  }
  expect(topBox.y + topBox.height).toBeLessThanOrEqual(bottomBox.y);
}

async function assertTransparentGapReachesCanvas(
  page: Page,
  top: Locator,
  bottom: Locator,
): Promise<void> {
  const [topBox, bottomBox] = await Promise.all([
    top.boundingBox(),
    bottom.boundingBox(),
  ]);
  if (topBox === null || bottomBox === null) {
    throw new Error('map toolbar and minimap must have browser bounds');
  }
  const tagName = await page.evaluate(
    ({ x, y }) => document.elementFromPoint(x, y)?.tagName ?? null,
    {
      x: bottomBox.x + bottomBox.width / 2,
      y:
        topBox.y + topBox.height + (bottomBox.y - topBox.y - topBox.height) / 2,
    },
  );
  expect(tagName).toBe('CANVAS');
}

interface TorchFalloffMetrics {
  readonly brightestTileLuminance: number;
  readonly brightestTileRedMinusBlue: number;
  readonly distantTileLuminance: number;
  readonly height: number;
  readonly tileColumns: number;
  readonly tileRows: number;
  readonly width: number;
}

async function assertAuthoredTorchLighting(
  page: Page,
  view: SessionView,
  testInfo: TestInfo,
): Promise<void> {
  const viewport = page.locator('[data-renderer-backend="rusty-engine-three"]');
  const torches = view.world.scenePlacements.filter(
    (placement) => placement.content.kind === 'prop',
  ).length;
  const lights = view.world.scenePlacements.filter(
    (placement) => placement.content.kind === 'point_light',
  ).length;
  expect(torches).toBeGreaterThan(0);
  expect(lights).toBe(torches);
  await expect(viewport).toHaveAttribute(
    'data-lighting-world-default',
    'disabled',
  );
  await expect(viewport).toHaveAttribute(
    'data-lighting-viewmodel-default',
    'neutral',
  );
  await expect(viewport).toHaveAttribute(
    'data-retained-light-count',
    String(lights),
  );
  const lightingPixels = await analyzeTorchFalloff(page);
  expect(lightingPixels.brightestTileLuminance).toBeGreaterThan(
    lightingPixels.distantTileLuminance + 8,
  );
  expect(lightingPixels.brightestTileRedMinusBlue).toBeGreaterThan(4);
  await testInfo.attach(`torch-falloff-${testInfo.project.name}.json`, {
    body: Buffer.from(JSON.stringify(lightingPixels, null, 2)),
    contentType: 'application/json',
  });
  await testInfo.attach(`torch-falloff-${testInfo.project.name}.png`, {
    body: await page.locator('canvas').screenshot(),
    contentType: 'image/png',
  });
}

async function analyzeTorchFalloff(page: Page): Promise<TorchFalloffMetrics> {
  const encodedScreenshot = (
    await page.locator('canvas').screenshot()
  ).toString('base64');
  return page.evaluate(async (encoded) => {
    const image = new Image();
    image.src = `data:image/png;base64,${encoded}`;
    await new Promise<void>((resolve, reject) => {
      image.addEventListener('load', () => resolve(), { once: true });
      image.addEventListener(
        'error',
        () => reject(new Error('decode failed')),
        {
          once: true,
        },
      );
    });
    const analysisCanvas = document.createElement('canvas');
    analysisCanvas.width = image.naturalWidth;
    analysisCanvas.height = image.naturalHeight;
    const context = analysisCanvas.getContext('2d', {
      willReadFrequently: true,
    });
    if (context === null) {
      throw new Error('screenshot analysis requires a detached 2D canvas');
    }
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(
      0,
      0,
      analysisCanvas.width,
      analysisCanvas.height,
    ).data;
    const tileColumns = 8;
    const tileRows = 6;
    const tiles: Array<{
      column: number;
      luminance: number;
      redMinusBlue: number;
      row: number;
    }> = [];
    for (let row = 0; row < tileRows; row += 1) {
      for (let column = 0; column < tileColumns; column += 1) {
        const startX = Math.floor(
          (column * analysisCanvas.width) / tileColumns,
        );
        const endX = Math.floor(
          ((column + 1) * analysisCanvas.width) / tileColumns,
        );
        const startY = Math.floor((row * analysisCanvas.height) / tileRows);
        const endY = Math.floor(((row + 1) * analysisCanvas.height) / tileRows);
        let luminance = 0;
        let redMinusBlue = 0;
        let samples = 0;
        for (let y = startY; y < endY; y += 2) {
          for (let x = startX; x < endX; x += 2) {
            const offset = (y * analysisCanvas.width + x) * 4;
            const red = pixels[offset] ?? 0;
            const green = pixels[offset + 1] ?? 0;
            const blue = pixels[offset + 2] ?? 0;
            luminance += red * 0.2126 + green * 0.7152 + blue * 0.0722;
            redMinusBlue += red - blue;
            samples += 1;
          }
        }
        tiles.push({
          column,
          luminance: luminance / samples,
          redMinusBlue: redMinusBlue / samples,
          row,
        });
      }
    }
    const brightest = tiles.reduce((current, tile) =>
      tile.luminance > current.luminance ? tile : current,
    );
    const distantTiles = tiles.filter(
      (tile) =>
        Math.abs(tile.column - brightest.column) +
          Math.abs(tile.row - brightest.row) >=
        4,
    );
    return {
      brightestTileLuminance: brightest.luminance,
      brightestTileRedMinusBlue: brightest.redMinusBlue,
      distantTileLuminance:
        distantTiles.reduce((total, tile) => total + tile.luminance, 0) /
        distantTiles.length,
      height: analysisCanvas.height,
      tileColumns,
      tileRows,
      width: analysisCanvas.width,
    };
  }, encodedScreenshot);
}

async function analyzeLocalOverview(
  page: Page,
  compact: boolean,
): Promise<{
  distinctPixelRatio: number;
  height: number;
  insetLuminanceRange: number;
  width: number;
}> {
  const encodedScreenshot = (
    await page.locator('canvas').screenshot()
  ).toString('base64');
  return page.evaluate(
    async ({ compact, encoded }) => {
      const image = new Image();
      image.src = `data:image/png;base64,${encoded}`;
      await new Promise<void>((resolve, reject) => {
        image.addEventListener('load', () => resolve(), { once: true });
        image.addEventListener(
          'error',
          () => reject(new Error('decode failed')),
          {
            once: true,
          },
        );
      });
      const canvas = document.createElement('canvas');
      canvas.width = image.naturalWidth;
      canvas.height = image.naturalHeight;
      const context = canvas.getContext('2d', { willReadFrequently: true });
      if (context === null) {
        throw new Error('overview analysis requires a detached 2D canvas');
      }
      context.drawImage(image, 0, 0);
      const pixels = context.getImageData(
        0,
        0,
        canvas.width,
        canvas.height,
      ).data;
      const viewport = compact
        ? { x: 0.66, y: 0.7, width: 0.3, height: 0.26 }
        : { x: 0.68, y: 0.62, width: 0.28, height: 0.32 };
      const startX = Math.floor(viewport.x * canvas.width);
      const endX = Math.floor((viewport.x + viewport.width) * canvas.width);
      const startY = Math.floor(
        (1 - viewport.y - viewport.height) * canvas.height,
      );
      const endY = Math.floor((1 - viewport.y) * canvas.height);
      const referenceStartX = Math.max(
        0,
        Math.floor((viewport.x - viewport.width - 0.05) * canvas.width),
      );
      let distinct = 0;
      let samples = 0;
      let minimumLuminance = Number.POSITIVE_INFINITY;
      let maximumLuminance = Number.NEGATIVE_INFINITY;
      for (let y = startY; y < endY; y += 2) {
        for (let x = startX; x < endX; x += 2) {
          const offset = (y * canvas.width + x) * 4;
          const referenceX = Math.min(
            canvas.width - 1,
            referenceStartX + (x - startX),
          );
          const referenceOffset = (y * canvas.width + referenceX) * 4;
          const red = pixels[offset] ?? 0;
          const green = pixels[offset + 1] ?? 0;
          const blue = pixels[offset + 2] ?? 0;
          const luminance = red * 0.2126 + green * 0.7152 + blue * 0.0722;
          minimumLuminance = Math.min(minimumLuminance, luminance);
          maximumLuminance = Math.max(maximumLuminance, luminance);
          const difference =
            Math.abs(red - (pixels[referenceOffset] ?? 0)) +
            Math.abs(green - (pixels[referenceOffset + 1] ?? 0)) +
            Math.abs(blue - (pixels[referenceOffset + 2] ?? 0));
          if (difference > 30) distinct += 1;
          samples += 1;
        }
      }
      return {
        distinctPixelRatio: distinct / samples,
        height: canvas.height,
        insetLuminanceRange: maximumLuminance - minimumLuminance,
        width: canvas.width,
      };
    },
    { compact, encoded: encodedScreenshot },
  );
}

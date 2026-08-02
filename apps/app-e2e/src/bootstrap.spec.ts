import { expect, test, type Locator, type Page } from '@playwright/test';

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
  'Cinder Eye',
  'Ember Watcher',
  'Goblin Scrapper',
  'Slag Cutpurse',
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
  await expect(objective).toContainText('all five dormant raiders');
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
  const minimap = page.locator('rr-minimap [role="img"]');
  const mapToolbar = page.locator('.map-toolbar');
  const initialView = await readSession(page);
  const initialDiscoveredCells = initialView.world.minimap.cells.length;
  const initialVisibleCells = initialView.world.minimap.cells.filter(
    (cell) => cell.visible,
  ).length;
  await expect(minimap).toHaveAttribute('data-minimap-revision', '8');
  await expect(minimap).toHaveAttribute(
    'data-discovered-cells',
    String(initialDiscoveredCells),
  );
  await expect(minimap).toHaveAttribute(
    'data-visible-cells',
    String(initialVisibleCells),
  );
  await expect(minimap).toHaveAttribute('data-visible-enemies', '0');
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
  await expect(stage).toHaveAttribute('data-session-revision', '8');
  await expect(minimap).toHaveAttribute('data-minimap-revision', '8');
  await expect(minimap).toHaveAttribute(
    'data-discovered-cells',
    String(initialDiscoveredCells),
  );
  await saveAndWait(page);
  await issueAndWait(page, stage, 'Step right');
  await expect(minimap).toHaveAttribute('data-minimap-revision', '9');
  await reopenAndWait(page);
  await expect(stage).toHaveAttribute('data-session-revision', '8');
  await expect(minimap).toHaveAttribute('data-minimap-revision', '8');
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
    expect(encounterView.world.minimap.cells.length).toBeGreaterThan(
      initialDiscoveredCells,
    );
    expect(
      encounterView.world.minimap.cells.filter((cell) => cell.visible).length,
    ).toBeLessThan(encounterView.world.minimap.cells.length);
    await expect(minimap).toHaveAttribute(
      'data-discovered-cells',
      String(encounterView.world.minimap.cells.length),
    );
    await expect(minimap).toHaveAttribute('data-visible-enemies', '1');
    await expect(minimap.locator('svg text.enemy')).toHaveCount(1);
    await expect(minimap.locator('.cell.remembered')).not.toHaveCount(0);
    const initiative = page.getByRole('navigation', {
      name: 'Initiative order',
    });
    await expect(initiative).toContainText(firstEnemy);
    const combatRevision = await stage.getAttribute('data-session-revision');
    await saveAndWait(page);
    await issueAndWait(page, stage, 'Turn right');
    await expect(minimap).toHaveAttribute('data-visible-enemies', '0');
    await expect(minimap.locator('svg text.enemy')).toHaveCount(0);
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
      if ((await actionRow.locator('button:enabled').count()) > 0) {
        break;
      }
      await issueAndWait(page, stage, 'Turn right');
    }
    const action = actionRow.locator('button:enabled').first();
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
    await page.keyboard.press('e');
    await expect(stage).toHaveAttribute('data-session-revision', '9');
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
